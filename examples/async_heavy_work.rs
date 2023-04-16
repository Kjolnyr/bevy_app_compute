use bevy::{core::cast_slice, prelude::*, reflect::TypeUuid, render::render_resource::ShaderRef};
use bevy_app_compute::{prelude::*, FinishedWorkerEvent};

#[derive(TypeUuid)]
#[uuid = "769df528-80d7-4df2-9033-5ea7c182d553"]
pub struct HeavyShader;

impl ComputeShader for HeavyShader {
    fn shader() -> ShaderRef {
        "shaders/heavy_work.wgsl".into()
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(AppComputePlugin::<HeavyShader>::default())
        .add_system(test)
        .add_system(receive_data)
        .run();
}

fn test(buttons: Res<Input<MouseButton>>, mut app_compute: ResMut<AppCompute>) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    };

    app_compute
        .worker()
        .add_storage("storage", vec![0f32; 30])
        .add_staging_buffer("staging", "storage")
        .pass::<HeavyShader>([30, 1, 1], &["storage"])
        .read_staging_buffers()
        .submit()
        .map_staging_buffers();
}

fn receive_data(
    app_compute: Res<AppCompute>,
    mut worker_events: EventReader<FinishedWorkerEvent>
) {
    for ev in &mut worker_events.iter() {
        let id = &ev.0;

        let Some(worker) = app_compute.get_worker(*id) else { continue; };

        let data = worker.get_data("staging");
        let values: &[f32] = cast_slice(&data);

        println!("values: {:?}", values);
    }
}
