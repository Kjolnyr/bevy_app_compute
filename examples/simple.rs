//! Simple, hello world example to show the basic concept

use bevy::prelude::*;
use bevy_app_compute::prelude::*;

#[derive(TypePath)]
struct SimpleShader;

impl ComputeShader for SimpleShader {
    fn shader() -> ShaderRef {
        "shaders/simple.wgsl".into()
    }
}

#[derive(Resource)]
struct SimpleComputeWorker;

impl ComputeWorker for SimpleComputeWorker {
    fn build(world: &mut World) -> AppComputeWorker<Self> {
        let worker = AppComputeWorkerBuilder::new(world)
            .add_uniform("uni", &5.)
            .add_staging("values", &[1., 2., 3., 4.])
            .add_pass::<SimpleShader>([4, 1, 1], &["uni", "values"])
            .build();

        worker
    }
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                // Do not create a window on startup.
                .set(WindowPlugin {
                    primary_window: None,
                    exit_condition: bevy::window::ExitCondition::DontExit,
                    close_when_requested: false,
                    primary_cursor_options: None,
                }),
        )
        .add_plugins(AppComputePlugin)
        .add_plugins(AppComputeWorkerPlugin::<SimpleComputeWorker>::default())
        .add_systems(Update, test)
        .run();
}

fn test(mut compute_worker: ResMut<AppComputeWorker<SimpleComputeWorker>>) {
    if !compute_worker.ready() {
        return;
    };

    let result: Vec<f32> = compute_worker.read_vec("values");

    compute_worker.write_slice::<f32>("values", &[2., 3., 4., 5.]);

    println!("got {:?}", result)
}
