//! Example showing how to execute compute shaders on demand

use bevy::{prelude::*, reflect::TypeUuid, render::render_resource::ShaderRef};
use bevy_app_compute::prelude::*;

#[derive(TypeUuid)]
#[uuid = "2545ae14-a9bc-4f03-9ea4-4eb43d1075a7"]
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
            .one_shot()
            .build();

        worker
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((AppComputePlugin, AppComputeWorkerPlugin::<SimpleComputeWorker>::default()))
        .add_systems(Update, (read_data, on_click_compute))
        .run();
}

fn on_click_compute(
    buttons: Res<Input<MouseButton>>,
    mut compute_worker: ResMut<AppComputeWorker<SimpleComputeWorker>>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }

    compute_worker.execute();
}

fn read_data(mut compute_worker: ResMut<AppComputeWorker<SimpleComputeWorker>>) {
    if !compute_worker.ready() {
        return;
    };

    let result: Vec<f32> = compute_worker.read_vec("values");

    compute_worker.write_slice("values", &result);

    println!("got {:?}", result)
}
