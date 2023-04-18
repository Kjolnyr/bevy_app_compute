//! Example showing how to calculate boids data from compute shaders
//! For now they are stupid and just fly straight, need to fix this later on.
//! Reimplementation of https://github.com/gfx-rs/wgpu-rs/blob/master/examples/boids/main.rs

use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};

use bevy::{
    core::Pod,
    prelude::*,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
    window::PrimaryWindow,
};

use bevy_app_compute::prelude::*;
use bytemuck::Zeroable;

use rand::distributions::{Distribution, Uniform};

// In debug mode, don't go over 1_500
//const NUM_BOIDS: u32 = 1_500;

// In release mode, this is totally ok.
const NUM_BOIDS: u32 = 15_000;

#[derive(ShaderType, Pod, Zeroable, Clone, Copy)]
#[repr(C)]
struct Params {
    delta_t: f32,
    rule_1_distance: f32,
    rule_2_distance: f32,
    rule_3_distance: f32,
    rule_1_scale: f32,
    rule_2_scale: f32,
    rule_3_scale: f32,
}

#[derive(ShaderType, Pod, Zeroable, Clone, Copy)]
#[repr(C)]
struct Boid {
    pos: Vec2,
    vel: Vec2,
}

#[derive(TypeUuid)]
#[uuid = "2545ae14-a9bc-4f03-9ea4-4eb43d1075a7"]
struct BoidsShader;

impl ComputeShader for BoidsShader {
    fn shader() -> ShaderRef {
        "shaders/boids.wgsl".into()
    }
}

struct BoidWorker;

impl ComputeWorker for BoidWorker {
    fn build(world: &mut World) -> AppComputeWorker<Self> {
        let params = Params {
            delta_t: 0.04f32,
            rule_1_distance: 0.1,
            rule_2_distance: 0.025,
            rule_3_distance: 0.025,
            rule_1_scale: 0.02,
            rule_2_scale: 0.05,
            rule_3_scale: 0.005,
        };

        let mut initial_boids_data = Vec::with_capacity(NUM_BOIDS as usize);
        let mut rng = rand::thread_rng();
        let unif = Uniform::new_inclusive(-1., 1.);

        for _ in 0..NUM_BOIDS {
            initial_boids_data.push(Boid {
                pos: Vec2::new(unif.sample(&mut rng), unif.sample(&mut rng)),
                vel: Vec2::new(unif.sample(&mut rng) * 0.001, unif.sample(&mut rng) * 0.001),
            });
        }

        AppComputeWorkerBuilder::new(world)
            .add_uniform("params", &params)
            .add_staging("boids_src", &initial_boids_data)
            .add_staging("boids_dst", &initial_boids_data)
            .add_pass::<BoidsShader>([NUM_BOIDS, 1, 1], &["params", "boids_src", "boids_dst"])
            .add_swap("boids_src", "boids_dst")
            .build()
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(AppComputePlugin)
        .add_plugin(AppComputeWorkerPlugin::<BoidWorker>::default())
        .insert_resource(ClearColor(Color::DARK_GRAY))
        .add_startup_system(setup)
        .add_system(move_entities)
        .run()
}

#[derive(Component)]
struct BoidEntity(pub usize);

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2dBundle::default());

    let boid_mesh = meshes.add(shape::RegularPolygon::new(3., 3).into());
    let boid_material = materials.add(Color::ANTIQUE_WHITE.into());

    for i in 0..NUM_BOIDS {
        commands.spawn((
            BoidEntity(i as usize),
            MaterialMesh2dBundle {
                mesh: Mesh2dHandle(boid_mesh.clone()),
                material: boid_material.clone(),
                ..Default::default()
            },
        ));
    }
}

fn move_entities(
    worker: Res<AppComputeWorker<BoidWorker>>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    mut q_boid: Query<(&mut Transform, &BoidEntity), With<BoidEntity>>,
) {
    if !worker.ready() {
        return;
    }

    let window = q_window.single();

    let boids: Vec<Boid> = worker.read("boids_dst").unwrap();

    q_boid.par_iter_mut().for_each_mut(|(mut transform, boid_entity)| {

        let world_pos = Vec2::new(
            (window.width() / 2.) * (boids[boid_entity.0].pos.x * 2. - 1.),
            (window.height() / 2.) * (boids[boid_entity.0].pos.y * 2. - 1.),
        );

        transform.translation = world_pos.extend(0.);

    });
}
