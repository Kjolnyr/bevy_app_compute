//! Conway's classic Game Of Life using compute shaders.
//!
//! This example demonstrates rendering buffer data directly, *without* copying compute data back to
//! the CPU, updating entities and copying back to the GPU for a render pass. The default
//! dimensions should simulate over 1 million cells with ease.
//!
//! Inspired by https://webgpu.github.io/webgpu-samples/?sample=gameOfLife

mod bind_groups;
mod worker;
/// Rendering code
mod render {
    pub mod draw_plugin;
    mod graph_node;
    mod pipeline;
}

use bevy::color::palettes::css;
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};

use bevy::prelude::*;

use bevy_easy_compute::prelude::*;
use render::draw_plugin::DrawPlugin;

use crate::bind_groups::get_buffers_for_renderer;
use worker::GameOfLifeWorker;

const DIMENSIONS: (u32, u32) = (1500, 1000);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(LogDiagnosticsPlugin::default())
        .add_plugins(FrameTimeDiagnosticsPlugin)
        .add_plugins(AppComputePlugin)
        .add_plugins(AppComputeWorkerPlugin::<GameOfLifeWorker>::default())
        .add_plugins(DrawPlugin)
        .insert_resource(ClearColor(css::BLACK.into()))
        .add_systems(Startup, get_buffers_for_renderer)
        .run();
}
