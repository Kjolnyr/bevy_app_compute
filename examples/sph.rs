use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::{
    core::Pod,
    input::mouse::{MouseScrollUnit, MouseWheel},
    math::vec2,
    prelude::*,
    render::camera::CameraProjection,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
    window::{PresentMode, PrimaryWindow, WindowPlugin},
};

use bevy_app_compute::prelude::*;
use bytemuck::Zeroable;
use rand::distributions::{Distribution, Uniform};

const NUM_PARTICLES: u32 = 1000;

#[derive(Resource, ShaderType, Pod, Zeroable, Clone, Copy)]
#[repr(C)]
struct Parameters {
    speed: f32,
    gravity_multiplier: f32,
    rest_density: f32,
    gas_constant: f32,
    viscosity: f32,
    length_scale: f32,
    particle_radius: f32,
    particle_area: f32,
    delta_time: f32,
    max_pressure_grad: f32,
    x_min: f32,
    x_max: f32,
    y_min: f32,
    y_max: f32,
}

impl Default for Parameters {
    fn default() -> Self {
        Self {
            speed: 1.0,
            gravity_multiplier: 98.0,
            rest_density: 1.0,
            gas_constant: 2.0e9,
            viscosity: 0.08,
            length_scale: 1.0,
            particle_radius: 0.1,
            particle_area: 1.0,
            delta_time: 0.001, 
            max_pressure_grad: 2000.0,
            x_min: -10.0,
            x_max: 80.0,
            y_min: 0.0, 
            y_max: 10.0,
        }
    }
}

#[derive(ShaderType, Pod, Zeroable, Clone, Copy)]
#[repr(C)]
struct Particle {
    position: Vec2,
    velocity: Vec2,
}

#[derive(ShaderType, Pod, Zeroable, Clone, Copy, Debug)]
#[repr(C)]
struct Density {
    value: f32,
    number: f32,
}

#[derive(TypeUuid)]
#[uuid = "2545ae14-a9bc-4f03-9ea4-4eb43d1075a7"]
struct SphShader;

impl ComputeShader for SphShader {
    fn shader() -> ShaderRef {
        "shaders/sph.wgsl".into()
    }
}

#[derive(TypeUuid)]
#[uuid = "5747af29-0ff7-4f3a-8051-a4ce52fcb4a8"]
struct DensityShader;

impl ComputeShader for DensityShader { 
    fn shader() -> ShaderRef {
        ShaderRef::from("shaders/sph_density.wgsl")
    }
}

struct BoidWorker;

impl ComputeWorker for BoidWorker {
    fn build(world: &mut World) -> AppComputeWorker<Self> {
        let params = world.get_resource::<Parameters>().unwrap().clone();

        let mut initial_boids_data = Vec::with_capacity(NUM_PARTICLES as usize);
        let mut rng = rand::thread_rng();
        let unif = Uniform::new_inclusive(-1., 1.);

        for i in 0..NUM_PARTICLES {
            initial_boids_data.push(Particle {
                position: Vec2::new(
                        (i % 80) as f32 * (params.particle_radius * 3.0) + 1.0 + params.x_min,
                        (i / 100) as f32 * (params.particle_radius * 3.0) + 1.0 + params.y_min,
                    ),
                velocity: 10.0
                    * Vec2::new(
                        unif.sample(&mut rng) * params.speed,
                        unif.sample(&mut rng) * params.speed,
                    ),
            });
        }

        AppComputeWorkerBuilder::new(world)
            .add_uniform("params", &params)
            .add_staging("particles_src", &initial_boids_data)
            .add_staging("particles_dst", &initial_boids_data)
            .add_staging("density", &vec![Density { value: 0.0, number: 0.0 }; NUM_PARTICLES as usize]) 
            .add_pass::<DensityShader>(
                [NUM_PARTICLES, 1, 1],
                &["params", "particles_src", "density"],
            )
            .add_pass::<SphShader>(
                [NUM_PARTICLES, 1, 1],
                &["params", "particles_src", "density", "particles_dst"],
            )
            .add_swap("particles_src", "particles_dst")
            .build()
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: (1200., 1800.).into(),
                present_mode: PresentMode::AutoNoVsync,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(PanCamPlugin)
        .add_plugins(LogDiagnosticsPlugin::default())
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .insert_resource(ClearColor(Color::DARK_GRAY))
        .insert_resource(Parameters::default())
        .add_plugins(AppComputePlugin)
        .add_plugins(AppComputeWorkerPlugin::<BoidWorker>::default())
        .add_systems(Startup, setup)
        .add_systems(Update, move_entities)
        .run()
}

#[derive(Component)]
struct BoidEntity(pub usize);

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    params: Res<Parameters>,
) {
    commands.spawn((
        Camera2dBundle {
            projection: OrthographicProjection {
                far: 1000.,
                near: -1000.,
                scale: 0.01,
                ..default()
            },
            ..default()
        },
        PanCam {
            grab_buttons: vec![MouseButton::Right, MouseButton::Middle], // which buttons should drag the camera
            enabled: true, // when false, controls are disabled. See toggle example.
            zoom_to_cursor: true, // whether to zoom towards the mouse or the center of the screen
            min_scale: 0.001, // prevent the camera from zooming too far in
            max_scale: Some(100.0), // prevent the camera from zooming too far out
            ..default()
        },
    ));

    // commands.spawn(Camera2dBundle {
    //     projection: OrthographicProjection {
    //         far: 1000.,
    //         near: -1000.,
    //         ..default()
    //     },
    //     ..default()
    // });

    let boid_mesh = meshes.add(shape::Circle::new(params.particle_radius).into());
    let boid_material = materials.add(Color::ANTIQUE_WHITE.into());

    // First boid in red, so we can follow it easily
    commands.spawn((
        BoidEntity(0),
        MaterialMesh2dBundle {
            mesh: Mesh2dHandle(boid_mesh.clone()),
            material: materials.add(Color::ORANGE_RED.into()),
            ..Default::default()
        },
    ));

    for i in 1..NUM_PARTICLES {
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
    time: Res<Time>,
    mut parameters: ResMut<Parameters>,
    mut worker: ResMut<AppComputeWorker<BoidWorker>>,
    mut q_boid: Query<(&mut Transform, &BoidEntity), With<BoidEntity>>,
) {
    if !worker.ready() {
        return;
    }

    // for x in worker.read_vec::<Density>("density") {
    //     println!("{:?}", x); 
    // }

    let boids = worker.read_vec::<Particle>("particles_dst");

    parameters.delta_time = time.delta_seconds() * 0.1;
    worker.write("params", parameters.as_ref());

    q_boid
        .par_iter_mut()
        .for_each(|(mut transform, boid_entity)| {
            transform.translation = boids[boid_entity.0].position.extend(0.0);
        });
}

/// Plugin that adds the necessary systems for `PanCam` components to work
#[derive(Default)]
pub struct PanCamPlugin;

/// System set to allow ordering of `PanCamPlugin`
#[derive(Debug, Clone, Copy, SystemSet, PartialEq, Eq, Hash)]
pub struct PanCamSystemSet;

impl Plugin for PanCamPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (camera_movement, camera_zoom).in_set(PanCamSystemSet),
        )
        .register_type::<PanCam>();
    }
}

fn camera_zoom(
    mut query: Query<(&PanCam, &mut OrthographicProjection, &mut Transform)>,
    mut scroll_events: EventReader<MouseWheel>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
) {
    let pixels_per_line = 100.; // Maybe make configurable?
    let scroll = scroll_events
        .read()
        .map(|ev| match ev.unit {
            MouseScrollUnit::Pixel => ev.y,
            MouseScrollUnit::Line => ev.y * pixels_per_line,
        })
        .sum::<f32>();

    if scroll == 0. {
        return;
    }

    let window = primary_window.single();
    let window_size = Vec2::new(window.width(), window.height());
    let mouse_normalized_screen_pos = window
        .cursor_position()
        .map(|cursor_pos| (cursor_pos / window_size) * 2. - Vec2::ONE)
        .map(|p| Vec2::new(p.x, -p.y));

    for (cam, mut proj, mut pos) in &mut query {
        if cam.enabled {
            let old_scale = proj.scale;
            proj.scale = (proj.scale * (1. + -scroll * 0.001)).max(cam.min_scale);

            // Apply max scale constraint
            if let Some(max_scale) = cam.max_scale {
                proj.scale = proj.scale.min(max_scale);
            }

            // If there is both a min and max boundary, that limits how far we can zoom. Make sure we don't exceed that
            let scale_constrained = BVec2::new(
                cam.min_x.is_some() && cam.max_x.is_some(),
                cam.min_y.is_some() && cam.max_y.is_some(),
            );

            if scale_constrained.x || scale_constrained.y {
                let bounds_width = if let (Some(min_x), Some(max_x)) = (cam.min_x, cam.max_x) {
                    max_x - min_x
                } else {
                    f32::INFINITY
                };

                let bounds_height = if let (Some(min_y), Some(max_y)) = (cam.min_y, cam.max_y) {
                    max_y - min_y
                } else {
                    f32::INFINITY
                };

                let bounds_size = vec2(bounds_width, bounds_height);
                let max_safe_scale = max_scale_within_bounds(bounds_size, &proj, window_size);

                if scale_constrained.x {
                    proj.scale = proj.scale.min(max_safe_scale.x);
                }

                if scale_constrained.y {
                    proj.scale = proj.scale.min(max_safe_scale.y);
                }
            }

            // Move the camera position to normalize the projection window
            if let (Some(mouse_normalized_screen_pos), true) =
                (mouse_normalized_screen_pos, cam.zoom_to_cursor)
            {
                let proj_size = proj.area.max / old_scale;
                let mouse_world_pos = pos.translation.truncate()
                    + mouse_normalized_screen_pos * proj_size * old_scale;
                pos.translation = (mouse_world_pos
                    - mouse_normalized_screen_pos * proj_size * proj.scale)
                    .extend(pos.translation.z);

                // As we zoom out, we don't want the viewport to move beyond the provided boundary. If the most recent
                // change to the camera zoom would move cause parts of the window beyond the boundary to be shown, we
                // need to change the camera position to keep the viewport within bounds. The four if statements below
                // provide this behavior for the min and max x and y boundaries.
                let proj_size = proj.area.size();

                let half_of_viewport = proj_size / 2.;

                if let Some(min_x_bound) = cam.min_x {
                    let min_safe_cam_x = min_x_bound + half_of_viewport.x;
                    pos.translation.x = pos.translation.x.max(min_safe_cam_x);
                }
                if let Some(max_x_bound) = cam.max_x {
                    let max_safe_cam_x = max_x_bound - half_of_viewport.x;
                    pos.translation.x = pos.translation.x.min(max_safe_cam_x);
                }
                if let Some(min_y_bound) = cam.min_y {
                    let min_safe_cam_y = min_y_bound + half_of_viewport.y;
                    pos.translation.y = pos.translation.y.max(min_safe_cam_y);
                }
                if let Some(max_y_bound) = cam.max_y {
                    let max_safe_cam_y = max_y_bound - half_of_viewport.y;
                    pos.translation.y = pos.translation.y.min(max_safe_cam_y);
                }
            }
        }
    }
}

/// max_scale_within_bounds is used to find the maximum safe zoom out/projection
/// scale when we have been provided with minimum and maximum x boundaries for
/// the camera.
fn max_scale_within_bounds(
    bounds_size: Vec2,
    proj: &OrthographicProjection,
    window_size: Vec2, //viewport?
) -> Vec2 {
    let mut p = proj.clone();
    p.scale = 1.;
    p.update(window_size.x, window_size.y);
    let base_world_size = p.area.size();
    bounds_size / base_world_size
}

fn camera_movement(
    primary_window: Query<&Window, With<PrimaryWindow>>,
    mouse_buttons: Res<Input<MouseButton>>,
    mut query: Query<(&PanCam, &mut Transform, &OrthographicProjection)>,
    mut last_pos: Local<Option<Vec2>>,
) {
    let window = primary_window.single();
    let window_size = Vec2::new(window.width(), window.height());

    // Use position instead of MouseMotion, otherwise we don't get acceleration movement
    let current_pos = match window.cursor_position() {
        Some(c) => Vec2::new(c.x, -c.y),
        None => return,
    };
    let delta_device_pixels = current_pos - last_pos.unwrap_or(current_pos);

    for (cam, mut transform, projection) in &mut query {
        if cam.enabled
            && cam
                .grab_buttons
                .iter()
                .any(|btn| mouse_buttons.pressed(*btn))
        {
            let proj_size = projection.area.size();

            let world_units_per_device_pixel = proj_size / window_size;

            // The proposed new camera position
            let delta_world = delta_device_pixels * world_units_per_device_pixel;
            let mut proposed_cam_transform = transform.translation - delta_world.extend(0.);

            // Check whether the proposed camera movement would be within the provided boundaries, override it if we
            // need to do so to stay within bounds.
            if let Some(min_x_boundary) = cam.min_x {
                let min_safe_cam_x = min_x_boundary + proj_size.x / 2.;
                proposed_cam_transform.x = proposed_cam_transform.x.max(min_safe_cam_x);
            }
            if let Some(max_x_boundary) = cam.max_x {
                let max_safe_cam_x = max_x_boundary - proj_size.x / 2.;
                proposed_cam_transform.x = proposed_cam_transform.x.min(max_safe_cam_x);
            }
            if let Some(min_y_boundary) = cam.min_y {
                let min_safe_cam_y = min_y_boundary + proj_size.y / 2.;
                proposed_cam_transform.y = proposed_cam_transform.y.max(min_safe_cam_y);
            }
            if let Some(max_y_boundary) = cam.max_y {
                let max_safe_cam_y = max_y_boundary - proj_size.y / 2.;
                proposed_cam_transform.y = proposed_cam_transform.y.min(max_safe_cam_y);
            }

            transform.translation = proposed_cam_transform;
        }
    }
    *last_pos = Some(current_pos);
}

/// A component that adds panning camera controls to an orthographic camera
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct PanCam {
    /// The mouse buttons that will be used to drag and pan the camera
    pub grab_buttons: Vec<MouseButton>,
    /// Whether camera currently responds to user input
    pub enabled: bool,
    /// When true, zooming the camera will center on the mouse cursor
    ///
    /// When false, the camera will stay in place, zooming towards the
    /// middle of the screen
    pub zoom_to_cursor: bool,
    /// The minimum scale for the camera
    ///
    /// The orthographic projection's scale will be clamped at this value when zooming in
    pub min_scale: f32,
    /// The maximum scale for the camera
    ///
    /// If present, the orthographic projection's scale will be clamped at
    /// this value when zooming out.
    pub max_scale: Option<f32>,
    /// The minimum x position of the camera window
    ///
    /// If present, the orthographic projection will be clamped to this boundary both
    /// when dragging the window, and zooming out.
    pub min_x: Option<f32>,
    /// The maximum x position of the camera window
    ///
    /// If present, the orthographic projection will be clamped to this boundary both
    /// when dragging the window, and zooming out.
    pub max_x: Option<f32>,
    /// The minimum y position of the camera window
    ///
    /// If present, the orthographic projection will be clamped to this boundary both
    /// when dragging the window, and zooming out.
    pub min_y: Option<f32>,
    /// The maximum y position of the camera window
    ///
    /// If present, the orthographic projection will be clamped to this boundary both
    /// when dragging the window, and zooming out.
    pub max_y: Option<f32>,
}

impl Default for PanCam {
    fn default() -> Self {
        Self {
            grab_buttons: vec![MouseButton::Left, MouseButton::Right, MouseButton::Middle],
            enabled: true,
            zoom_to_cursor: true,
            min_scale: 0.00001,
            max_scale: None,
            min_x: None,
            max_x: None,
            min_y: None,
            max_y: None,
        }
    }
}
