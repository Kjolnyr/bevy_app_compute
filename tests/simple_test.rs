use bevy::prelude::*;
use bevy_easy_compute::prelude::*;

mod common;

const INPUT_BUFFER_NAME: &str = "values";

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
            .add_staging(INPUT_BUFFER_NAME, &[1., 2., 3., 4.])
            .add_pass::<SimpleShader>([4, 1, 1], &["uni", INPUT_BUFFER_NAME])
            .build();

        worker
    }
}

#[test]
fn increments_floats() {
    fn test(compute_worker: ResMut<AppComputeWorker<SimpleComputeWorker>>) {
        let result: Vec<f32> = compute_worker.read_vec(INPUT_BUFFER_NAME);
        assert!(result == [6.0, 7.0, 8.0, 9.0]);
    }

    let mut app = common::build_app::<SimpleComputeWorker>();
    app.add_systems(Update, test);
    app.update();
}

// Crude test to test that the tests work. Therefore we want to make sure that the test harness isn't returning
// false positives.
#[test]
#[should_panic]
fn should_panic() {
    fn test(compute_worker: ResMut<AppComputeWorker<SimpleComputeWorker>>) {
        let result: Vec<f32> = compute_worker.read_vec(INPUT_BUFFER_NAME);
        assert!(result == [0.0]);
    }

    let mut app = common::build_app::<SimpleComputeWorker>();
    app.add_systems(Update, test);
    app.update();
}
