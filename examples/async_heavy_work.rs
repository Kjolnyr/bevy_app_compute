use bevy::{core::cast_slice, prelude::*, render::render_resource::ShaderRef};
use bevy_app_compute::prelude::*;

pub struct HeavyComputeShader;

impl ComputeShader for HeavyComputeShader {
    fn shader() -> ShaderRef {
        "shaders/heavy_work.wgsl".into()
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(AppComputePlugin::<HeavyComputeShader>::default())
        .add_system(test)
        .add_system(receive_data)
        .run();
}

fn test(buttons: Res<Input<MouseButton>>, mut app_compute: ResMut<AppCompute<HeavyComputeShader>>) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    };

    let storage_vec = vec![0f32; 30];
    let Some(mut worker) = app_compute.worker() else { return; };

    worker.add_storage(0, "storage", storage_vec);
    worker.add_staging_buffer("staging", "storage", std::mem::size_of::<f32>() * 30);

    app_compute.queue(worker, (30, 1, 1));
}

fn receive_data(mut worker_events: EventReader<WorkerEvent<HeavyComputeShader>>) {
    for ev in &mut worker_events.iter() {
        let worker = &ev.worker;

        let result = worker.get_data("staging");

        let value: &[f32] = cast_slice(&result);

        println!("got {} items back", value.len());
    }
}
