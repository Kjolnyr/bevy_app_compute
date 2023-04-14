use bevy::{core::cast_slice, prelude::*, render::render_resource::ShaderRef};
use bevy_app_compute::prelude::*;

struct SimpleComputeShader;

impl ComputeShader for SimpleComputeShader {
    fn shader() -> ShaderRef {
        "shaders/simple.wgsl".into()
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(AppComputePlugin::<SimpleComputeShader>::default())
        .add_system(on_click_compute)
        .run();
}

fn on_click_compute(
    buttons: Res<Input<MouseButton>>,
    app_compute: Res<AppCompute<SimpleComputeShader>>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    };

    let Some(mut worker) = app_compute.worker() else { return; };

    worker.add_uniform(0, "uni", 5f32);
    worker.add_storage(0, "storage", vec![0f32; 8]);
    worker.add_staging_buffer("staging", "storage", std::mem::size_of::<f32>() * 8);

    worker.run((8, 1, 1));

    let result = worker.get_data("staging");

    let value: &[f32] = cast_slice(&result);

    println!("value: {:?}", value); // [1.0, 5.0, 24.999998, 124.999985, 624.9999, 3124.9993, 15624.996, 78124.98]
}
