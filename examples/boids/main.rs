//! Example showing how to calculate boids data from compute shaders
//! Reimplementation of https://github.com/gfx-rs/wgpu-rs/blob/master/examples/boids/main.rs

mod worker;

use bevy::color::palettes::css;
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};

use bevy::{
    prelude::*,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
    window::PrimaryWindow,
};

use bevy_easy_compute::prelude::*;

use worker::{Boid, BoidWorker};

const NUM_BOIDS: u32 = 2_000;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(LogDiagnosticsPlugin::default())
        .add_plugins(FrameTimeDiagnosticsPlugin)
        .add_plugins(AppComputePlugin)
        .add_plugins(AppComputeWorkerPlugin::<BoidWorker>::default())
        .insert_resource(ClearColor(css::BLACK.into()))
        .add_systems(Startup, setup)
        .add_systems(Update, move_entities)
        .run();
}

#[derive(Component)]
struct BoidEntity(pub usize);

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2dBundle::default());

    let boid_mesh = meshes.add(RegularPolygon::new(2., 3));
    let boid_material = materials.add(Color::from(css::ANTIQUE_WHITE));

    // First boid in red, so we can follow it easily
    commands.spawn((
        BoidEntity(0),
        MaterialMesh2dBundle {
            mesh: Mesh2dHandle(boid_mesh.clone()),
            material: materials.add(Color::from(css::ORANGE_RED)),
            ..Default::default()
        },
    ));

    for i in 1..NUM_BOIDS {
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
    worker: ResMut<AppComputeWorker<BoidWorker>>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    mut q_boid: Query<(&mut Transform, &BoidEntity), With<BoidEntity>>,
) {
    if !worker.ready() {
        return;
    }

    let window = q_window.single();

    let boids = worker.read_vec::<Boid>("boids_dst");

    q_boid
        .par_iter_mut()
        .for_each(|(mut transform, boid_entity)| {
            let world_pos = Vec2::new(
                (window.width() / 2.) * (boids[boid_entity.0].pos.x),
                (window.height() / 2.) * (boids[boid_entity.0].pos.y),
            );

            transform.translation = world_pos.extend(0.);
            transform.look_to(Vec3::Z, boids[boid_entity.0].vel.extend(0.));
        });
}
