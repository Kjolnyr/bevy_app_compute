use bevy::{core::cast_slice, prelude::*, reflect::TypeUuid, render::render_resource::ShaderRef};
use bevy_app_compute::prelude::*;

#[derive(TypeUuid)]
#[uuid = "2545ae14-a9bc-4f03-9ea4-4eb43d1075a7"]
struct SimpleShader;

impl ComputeShader for SimpleShader {
    fn shader() -> ShaderRef {
        "shaders/simple.wgsl".into()
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(AppComputePlugin::<SimpleShader>::default())
        .add_system(on_click_compute)
        .run();
}

fn on_click_compute(buttons: Res<Input<MouseButton>>, mut app_compute: ResMut<AppCompute>) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    };

    let value: f32 = 5.;

    let worker = app_compute
        .worker()
        .add_uniform("value", value)
        .add_storage("storage", vec![1., 2., 3., 4.])
        .add_staging_buffer("staging", "storage")
        .pass::<SimpleShader>([4, 1, 1], &["value", "storage"])
        .read_staging_buffers()
        .submit()
        .map_staging_buffers()
        .now();

    let result = worker.get_data("staging");

    let value: &[f32] = cast_slice(&result);

    println!("value: {:?}", value);
}
