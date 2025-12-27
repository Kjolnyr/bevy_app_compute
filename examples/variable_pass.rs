//! Example showing how to execute compute shaders on-demand

use bevy::prelude::*;
use bevy_app_compute::prelude::*;

///Just some random random unsigned int function to not add unnecessary dependency just for a sake of an example
fn rand_int() -> u32 {
    static mut X: u32 = 0x12345678;

    unsafe {
        X = X.wrapping_mul(1103515245).wrapping_add(12345);
        X
    }
}

impl ComputeShader for VariablePassShader {
    fn shader() -> ShaderRef {
        "shaders/variable_pass.wgsl".into()
    }
}

#[derive(TypePath)]
struct VariablePassShader;

#[derive(Resource)]
struct VariablePassComputeWorker;

impl ComputeWorker for VariablePassComputeWorker {
    fn build(world: &mut World) -> AppComputeWorker<Self> {
        let worker = AppComputeWorkerBuilder::new(world)
            .add_staging("counter", &0)
            .add_pass::<VariablePassShader>([1, 1, 1], &["counter"])
            .one_shot()
            .build();

        worker
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(AppComputePlugin)
        .add_systems(Startup, setup)
        .add_plugins(AppComputeWorkerPlugin::<VariablePassComputeWorker>::default())
        .add_systems(Update, (on_click_compute, read_data))
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera3d::default());
    println!("Click anywhere in the window to trigger a one-shot compute job");
}

fn on_click_compute(
    buttons: Res<ButtonInput<MouseButton>>,
    mut compute_worker: ResMut<AppComputeWorker<VariablePassComputeWorker>>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let x = rand_int() % 64;
    let y = rand_int() % 64;
    let z = rand_int() % 64;

    println!("Generated random workgroups at x={}, y={}, z={}, result should={}", x, y, z, x * y * z);

    compute_worker.set_workgroups([x, y, z], 0);
    compute_worker.execute();
}

fn read_data(mut compute_worker: ResMut<AppComputeWorker<VariablePassComputeWorker>>) {
    if !compute_worker.ready() {
        return;
    };

    let result: u32 = compute_worker.read("counter");
    println!("got {:?}", result);

    compute_worker.write("counter", &0);

}
