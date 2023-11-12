#![doc = include_str!("../README.md")]

use bevy::prelude::{AssetEvent, Assets, EventReader, Res, ResMut, Shader};

use pipeline_cache::AppPipelineCache;

mod error;
mod pipeline_cache;
mod plugin;
mod traits;
mod worker;
mod worker_builder;

/// Helper module to import most used elements.
pub mod prelude {
    pub use crate::{
        plugin::{AppComputePlugin, AppComputeWorkerPlugin},
        traits::{ComputeShader, ComputeWorker},
        worker::AppComputeWorker,
        worker_builder::AppComputeWorkerBuilder,
    };

    // Since these are always used when using this crate
    pub use bevy::{
        reflect::TypeUuid,
        render::render_resource::{ShaderRef, ShaderType},
    };
}

pub(crate) fn process_pipeline_queue_system(mut pipeline_cache: ResMut<AppPipelineCache>) {
    pipeline_cache.process_queue();
}

pub(crate) fn extract_shaders(
    mut pipeline_cache: ResMut<AppPipelineCache>,
    shaders: Res<Assets<Shader>>,
    mut events: EventReader<AssetEvent<Shader>>,
) {
    for event in events.iter() {
        match event {
            AssetEvent::Created { handle } | AssetEvent::Modified { handle } => {
                if let Some(shader) = shaders.get(handle) {
                    pipeline_cache.set_shader(handle, shader);
                }
            }
            AssetEvent::Removed { handle } => pipeline_cache.remove_shader(handle),
        }
    }
}
