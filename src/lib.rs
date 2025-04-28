#![doc = include_str!("../README.md")]

use bevy::{
    asset::{AssetEvent, Assets},
    ecs::{
        event::EventReader,
        system::{Res, ResMut},
    },
    render::render_resource::Shader,
};
use pipeline_cache::PipelineCache;

mod error;
mod pipeline_cache;
mod plugin;
mod traits;
mod worker;
mod worker_builder;

/// Helper module to import most used elements.
pub mod prelude {
    pub use crate::{
        plugin::{
            AppComputePlugin, AppComputeWorkerPlugin, BevyEasyComputePostUpdateSet,
            BevyEasyComputeSet,
        },
        traits::{ComputeShader, ComputeWorker},
        worker::AppComputeWorker,
        worker_builder::AppComputeWorkerBuilder,
    };

    // Since these are always used when using this crate
    pub use bevy::render::render_resource::{ShaderRef, ShaderType};
}

pub(crate) fn extract_shaders(
    mut pipeline_cache: ResMut<PipelineCache>,
    shaders: Res<Assets<Shader>>,
    mut events: EventReader<AssetEvent<Shader>>,
) {
    for event in events.read() {
        match event {
            AssetEvent::Added { id: shader_id } | AssetEvent::Modified { id: shader_id } => {
                if let Some(shader) = shaders.get(*shader_id) {
                    pipeline_cache.set_shader(*shader_id, shader);
                }
            }
            AssetEvent::Removed { id: shader_id } => pipeline_cache.remove_shader(*shader_id),
            AssetEvent::LoadedWithDependencies { id: shader_id } => {
                if let Some(shader) = shaders.get(*shader_id) {
                    pipeline_cache.set_shader(*shader_id, shader);
                }
            }
            AssetEvent::Unused { id: _ } => (),
        }
    }
}
