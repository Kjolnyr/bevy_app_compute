# Bevy App Compute

![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)
[![Doc](https://docs.rs/bevy_app_compute/badge.svg)](https://docs.rs/bevy_app_compute)
[![Crate](https://img.shields.io/crates/v/bevy_app_compute.svg)](https://crates.io/crates/bevy_app_compute)


Dispatch and run compute shaders on bevy from App World .

## Getting Started

Add the following line to your `Cargo.toml`

```toml
[dependencies]
bevy_app_compute = { git = "https://github.com/Kjolnyr/bevy_app_compute.git", branch = "dev" }
```

## Usage

### Setup

Declare your shaders in structs implementing `ComputeShader`. The `shader()` fn should point to your shader source code.
You need to derive `TypeUuid` as well and assign a unique Uuid:

```rust
#[derive(TypeUuid)]
#[uuid = "2545ae14-a9bc-4f03-9ea4-4eb43d1075a7"]
struct SimpleShader;

impl ComputeShader for SimpleShader {
    fn shader() -> ShaderRef {
        "shaders/simple.wgsl".into()
    }
}
```

Next, declare a struct implementing `ComputeWorker` to declare the bindings and the logic of your worker:

```rust
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

```

Don't forget to add a shader file to your `assets/` folder:

```rust

@group(0) @binding(0)
var<uniform> uni: f32;

@group(0) @binding(1)
var<storage, read_write> my_storage: array<f32>;

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    my_storage[invocation_id.x] = my_storage[invocation_id.x] + uni;
}
```

Add the `AppComputePlugin` plugin to your app, as well as one `AppComputeWorkerPlugin` per struct implementing `ComputeWorker`:

```rust
use bevy::prelude::*;
use bevy_app_compute::AppComputePlugin;

fn main() {
    App::new()
        .add_plugin(AppComputePlugin)
        .add_plugin(AppComputeWorkerPlugin::<SimpleComputeWorker>::default());
}
```

Your compute worker will now run every frame, during the `PostUpdate` stage. To read/write from it, use the `AppComputeWorker<T>` resource!

```rust
fn my_system(
    mut compute_worker: ResMut<AppComputeWorker<SimpleComputeWorker>>
) {
    if !compute_worker.available() {
        return;
    };

    let values = compute_worker.read("values");
    let result: &[f32] = cast_slice(&values);

    compute_worker.write("values", [2., 3., 4., 5.]);

    println!("got {:?}", result)
}
```

### Multiple passes

You can have multiple passes without having to copy data back to the CPU in between:

```rust
let worker = AppComputeWorkerBuilder::new(world)
    .add_uniform("value", &3.)
    .add_storage("input", &[1., 2., 3., 4.])
    .add_staging("output", &[0f32; 4])
    // add each item + `value` from `input` to `output`
    .add_pass::<FirstPassShader>([4, 1, 1], &["value", "input", "output"]) 
    // multiply each element of `output` by itself
    .add_pass::<SecondPassShader>([4, 1, 1], &["output"]) 
    .build();

    // the `output` buffer will contain [16.0, 25.0, 36.0, 49.0]

```


## Examples

See [examples](https://github.com/kjolnyr/bevy_app_compute/tree/main/examples)


## Bevy version mapping

|Bevy|bevy_app_compute|
|---|---|
|main|main|
|0.10|0.10.1|