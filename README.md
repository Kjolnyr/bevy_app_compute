# Bevy App Compute

![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)
[![Doc](https://docs.rs/bevy_app_compute/badge.svg)](https://docs.rs/bevy_app_compute)
[![Crate](https://img.shields.io/crates/v/bevy_mod_compute.svg)](https://crates.io/crates/bevy_mod_compute)


Dispatch and run compute shaders on bevy from App World .

## Usage

### Setup

Make an empty struct and implement `ComputeShader` on it. the `shader()` fn should point to your shader source code:
```rust
struct SimpleComputeShader;

impl ComputeShader for SimpleComputeShader {
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
        .add_plugin(AppComputePlugin::<SimpleComputeShader>::default());
}
```

And then use the `AppCompute<T>` resource to run compute shaders!

```rust
use bevy::prelude::*;
use bevy_app_compute::prelude::*;

fn my_system(
    app_compute: Res<AppCompute<SimpleComputeShader>>,
) {

    // Create a new worker
    let Some(mut worker) = app_compute.worker() else { return; };

    // Add some uniforms and storages with default values
    worker.add_uniform(0, "uni", 5f32);
    worker.add_storage(0, "storage", vec![0f32; 8]);

    // Create a buffer needed to get data back from the GPU
    // It has to be linked to a storage.
    worker.add_staging_buffer("staging", "storage", std::mem::size_of::<f32>() * 8);

    // run the shader
    worker.run((8, 1, 1));

    // You can read data from your staging buffer now
    let result = worker.get_data("staging");
    let value: &[f32] = cast_slice(&result);

    println!("value: {:?}", value);
}
```


### Asynchronous computation

You can run your compute shaders asynchronously to avoid loosing frame time.

```rust
use bevy::prelude::*;
use bevy_app_compute::prelude::*;

fn my_sender_system(
    mut app_compute: ResMut<AppCompute<MyComputeShader>>
    ){
    
    let Some(mut worker) = app_compute.worker() else { return; };

    worker.add_storage(0, "storage", [0f32; 30]);
    worker.add_staging_buffer("staging", "storage", std::mem::size_of::<f32>() * 30);

    // queue your worker for later use
    app_compute.queue(worker, (30, 1, 1));
}

fn my_receiver_system(
mut worker_events: EventReader<WorkerEvent<MyComputeShader>>
) {
    // An event is fired once a worker has finished processing
    for ev in &mut worker_events.iter() {
        let worker = &ev.worker;

        let result = worker.get_data("staging");
        let value: &[f32] = cast_slice(&result);

        println!("got {} items back!", value.len());
    }
}
```

## Examples

See [examples](https://github.com/kjolnyr/bevy_app_compute/tree/main/examples)

