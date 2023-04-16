# Bevy App Compute

![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)
[![Doc](https://docs.rs/bevy_app_compute/badge.svg)](https://docs.rs/bevy_app_compute)
[![Crate](https://img.shields.io/crates/v/bevy_app_compute.svg)](https://crates.io/crates/bevy_app_compute)


Dispatch and run compute shaders on bevy from App World .

## Getting Started

Add the following line to your `Cargo.toml`

```toml
[dependencies]
bevy_app_compute = "0.10.1"
```

## Usage

### Setup

Make an empty struct and implement `ComputeShader` on it. the `shader()` fn should point to your shader source code.
You need to derive `TypeUuid` as well and assign a unique Uuid.
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

Add the plugin to your app:

```rust
use bevy::prelude::*;
use bevy_app_compute::AppComputePlugin;

fn main() {
    App::new()
        .add_plugin(AppComputePlugin::<SimpleShader>::default())
}
```

And then use the `AppCompute` resource to build workers. These workers will let you configure and run some compute shaders from App World!

```rust
use bevy::prelude::*;
use bevy_app_compute::prelude::*;

fn my_system(
    app_compute: Res<AppCompute>,
) {

    // Get a fresh worker from AppCompute
    let worker = app_compute.worker()

        // Add some uniforms and storages
        .add_uniform("value", 5.)
        .add_storage("storage", vec![1., 2., 3., 4.])

        // Add a staging buffer and link it to a storage
        // It will let you get data back from the GPU.
        .add_staging_buffer("staging", "storage")

        // Dispatch some work to your shader.
        // You need to specify workgroups and used variables
        .pass::<SimpleShader>([4, 1, 1], &["value", "storage"])

        // Copy the data from the storage to your staging buffer
        .read_staging_buffers()

        // Submit! This is where the magic happens.
        .submit()

        // Map your buffers back to CPU world.
        .map_staging_buffers()

        // Get the data immediately, this will block your system.
        .now();

    let result = worker.get_data("staging");
    let value: &[f32] = cast_slice(&result);

    println!("value: {:?}", value);
}
```


### Non blocking computation

You can run your compute shaders without blocking the current system as well. Please be aware that WGPU doesn't support multiple queues, so your submit will be blended in bevy's render queue. This is not ideal.

```rust
use bevy::prelude::*;
use bevy_app_compute::prelude::*;

fn my_sender_system(
    mut app_compute: ResMut<AppCompute>
) {
    
    app_compute
        .worker()
        .add_storage("storage", vec![0f32; 30])
        .add_staging_buffer("staging", "storage")
        .pass::<HeavyShader>([30, 1, 1], &["storage"])
        .read_staging_buffers()
        .submit()
        .map_staging_buffers();

        // We do not call `now()` here.
}

fn my_receiver_system(
    app_compute: Res<AppCompute>,
    mut worker_events: EventReader<FinishedWorkerEvent>
) {
    // An event is fired once a worker has finished processing.
    // This, for now, happens in the next frame.
    for ev in &mut worker_events.iter() {
        let id = &ev.0;

        let Some(worker) = app_compute.get_worker(*id) else { continue; };

        let data = worker.get_data("staging");
        let values: &[f32] = cast_slice(&data);

        println!("values: {:?}", values);
    }
}
```


### Multiple passes

You can have multiple passes without having to copy data back to the CPU in between:

```rust
let worker = app_compute
        .worker()
        .add_uniform("value", 5.)
        .add_storage("input", vec![1., 2., 3., 4.])
        .add_storage("output", vec![0f32; 4])
        .add_storage("final", vec![0f32; 4])
        .add_staging_buffer("staging", "final")
        // Here we run two passes
        // add `value` to each element of `output`
        .pass::<FirstPassShader>([4, 1, 1], &["value", "input", "output"])
        // multiply each element of `output` by itself 
        .pass::<SecondPassShader>([4, 1, 1], &["output", "final"]) 
        .read_staging_buffers()
        .submit()
        .map_staging_buffers()
        .now();

    let result = worker.get_data("staging");
    let value: &[f32] = cast_slice(&result);

    println!("value: {:?}", value); // [36.0, 49.0, 64.0, 81.0]
```


## Examples

See [examples](https://github.com/kjolnyr/bevy_app_compute/tree/main/examples)


## Bevy version mapping

|Bevy|bevy_easings|
|---|---|
|main|main|
|0.10|0.10|