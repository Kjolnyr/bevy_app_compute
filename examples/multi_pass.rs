use bevy::{core::cast_slice, prelude::*, reflect::TypeUuid, render::render_resource::ShaderRef};
use bevy_app_compute::prelude::*;

#[derive(TypeUuid)]
#[uuid = "5a4f7163-88cd-4a59-94c7-fb51abe389b8"]
struct FirstPassShader;

impl ComputeShader for FirstPassShader {
    fn shader() -> ShaderRef {
        "shaders/first_pass.wgsl".into()
    }
}

#[derive(TypeUuid)]
#[uuid = "7ada0206-7871-404b-b197-5e2477e7073f"]
struct SecondPassShader;

impl ComputeShader for SecondPassShader {
    fn shader() -> ShaderRef {
        "shaders/second_pass.wgsl".into()
    }
}

#[derive(Resource)]
struct SimpleComputeWorker;

impl ComputeWorker for SimpleComputeWorker {
    fn build(world: &mut World) -> AppComputeWorker<Self> {
        let worker = AppComputeWorkerBuilder::new(world)
            .add_uniform("value", &3.)
            .add_storage("input", &[1., 2., 3., 4.])
            .add_staging("output", &[0f32; 4])
            .add_pass::<FirstPassShader>([4, 1, 1], &["value", "input", "output"]) // add each item + `value` from `input` to `output`
            .add_pass::<SecondPassShader>([4, 1, 1], &["output"]) // multiply each element of `output` by itself
            .build();

        // [1. + 3., 2. + 3., 3. + 3., 4. + 3.] = [4., 5., 6., 7.]
        // [4. * 4., 5. * 5., 6. * 6., 7. * 7.] = [16., 25., 36., 49.]

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

fn test(compute_worker: Res<AppComputeWorker<SimpleComputeWorker>>) {
    if !compute_worker.ready() {
        return;
    };

    let values = compute_worker.read("output");
    let result: &[f32] = cast_slice(&values);

    println!("got {:?}", result) // [16., 25., 36., 49.]
}
