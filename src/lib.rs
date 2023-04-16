use bevy::prelude::{AssetEvent, Assets, EventReader, Res, ResMut, Shader};

use pipeline_cache::AppPipelineCache;
use worker::WorkerId;

mod pipeline_cache;
mod plugin;
mod traits;
mod worker;
mod worker_builder;

pub mod prelude {
    pub use crate::{
        plugin::{AppComputePlugin, AppComputeWorkerPlugin},
        traits::{ComputeShader, ComputeWorker},
        worker::AppComputeWorker,
        worker_builder::AppComputeWorkerBuilder,
        FinishedWorkerEvent,
    };
}

pub fn process_pipeline_queue_system(mut pipeline_cache: ResMut<AppPipelineCache>) {
    pipeline_cache.process_queue();
}

pub fn extract_shaders(
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

pub struct FinishedWorkerEvent(pub WorkerId);

impl From<WorkerId> for FinishedWorkerEvent {
    fn from(id: WorkerId) -> Self {
        Self(id)
    }
}
