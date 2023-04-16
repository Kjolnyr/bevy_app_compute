use bevy::{core::cast_slice, prelude::*, reflect::TypeUuid, render::render_resource::ShaderRef};
use bevy_app_compute::prelude::*;

#[derive(TypeUuid)]
#[uuid = "b8420da2-3fa7-4321-87b6-04c00b0d8712"]
struct FirstPassShader;

impl ComputeShader for FirstPassShader {
    fn shader() -> ShaderRef {
        "shaders/first_pass.wgsl".into()
    }
}

#[derive(TypeUuid)]
#[uuid = "1a17d778-b013-4da0-a65a-7904f8f60274"]
struct SecondPassShader;

impl ComputeShader for SecondPassShader {
    fn shader() -> ShaderRef {
        "shaders/second_pass.wgsl".into()
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(AppComputePlugin::<FirstPassShader>::default())
        .add_plugin(AppComputePlugin::<SecondPassShader>::default())
        .add_system(on_click_compute)
        .run();
}

fn on_click_compute(buttons: Res<Input<MouseButton>>, mut app_compute: ResMut<AppCompute>) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    };

    let value: f32 = 5.;
    let input_buffer: Vec<f32> = vec![1., 2., 3., 4.];

    let worker = app_compute
        .worker()
        .add_uniform("value", value)
        .add_storage("input", input_buffer)
        .add_storage("output", vec![0f32; 4])
        .add_storage("final", vec![0f32; 4])
        .add_staging_buffer("staging", "final")
        .pass::<FirstPassShader>([4, 1, 1], &["value", "input", "output"]) // add `value` to each element of `output`
        .pass::<SecondPassShader>([4, 1, 1], &["output", "final"]) // multiply each element of `output` by itself
        .read_staging_buffers()
        .submit()
        .map_staging_buffers()
        .now();

    let result = worker.get_data("staging");
    let value: &[f32] = cast_slice(&result);
    println!("value: {:?}", value); // [36.0, 49.0, 64.0, 81.0]
}
