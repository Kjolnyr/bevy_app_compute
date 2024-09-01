use bevy::{prelude::*, winit::WinitPlugin};
use bevy_easy_compute::prelude::{
    AppComputePlugin, AppComputeWorker, AppComputeWorkerPlugin, ComputeWorker,
};

// TODO: Look into Bevy's `bevy_ci_testing` plugin: https://docs.rs/bevy/latest/bevy/dev_tools/ci_testing/index.html
// As far as I can see it just disables unneeded plugins, but it may do more than that.

// The maximum number of frames to run waiting for the compute worker to finish.
// We're not controlling the FPS, so this isn't necessarily a refecltion of how much time the
// compute workers should take.
const MAX_FRAMES_TO_READY: i16 = 10;

pub fn build_app<T>() -> App
where
    T: ComputeWorker,
{
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.build().disable::<WinitPlugin>())
        .add_plugins(AppComputePlugin)
        .add_plugins(AppComputeWorkerPlugin::<T>::default());
    app.finish();
    app.cleanup();

    let mut is_ready = false;
    for _ in 0..MAX_FRAMES_TO_READY {
        app.update();
        let compute_worker = app.world().get_resource::<AppComputeWorker<T>>();
        if compute_worker.unwrap().ready() {
            is_ready = true;
            break;
        }
    }
    if !is_ready {
        panic!("Compute worker didn't complete in {MAX_FRAMES_TO_READY} frames.")
    }

    app
}
