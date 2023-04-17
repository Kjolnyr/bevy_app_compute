//! Simple, hello world example to show the basic concept

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
            .build();

        worker
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(AppComputePlugin)
        .add_plugin(AppComputeWorkerPlugin::<SimpleComputeWorker>::default())
        .add_system(test)
        .run();
}

fn test(mut compute_worker: ResMut<AppComputeWorker<SimpleComputeWorker>>) {
    if !compute_worker.ready() {
        return;
    };

    let result: Vec<f32> = compute_worker.read("values").unwrap();

    compute_worker.write("values", &[2., 3., 4., 5.]).ok();

    println!("got {:?}", result)
}
