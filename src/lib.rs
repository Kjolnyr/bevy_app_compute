use app_compute::AppCompute;
use bevy::{
    prelude::{AssetEvent, Assets, EventReader, Res, ResMut, Shader},
    reflect::TypeUuid,
    render::render_resource::{BindGroupLayout, ShaderDefVal, ShaderRef},
};

use pipeline_cache::AppPipelineCache;
use wgpu::PushConstantRange;
use worker::WorkerId;

mod app_compute;
mod pipeline_cache;
mod plugin;
mod worker;

pub mod prelude {
    pub use crate::{app_compute::AppCompute, plugin::AppComputePlugin, ComputeShader};
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

pub trait ComputeShader: TypeUuid + Sized + Send + Sync + 'static {
    fn shader() -> ShaderRef;

    fn layouts<'a>() -> &'a [BindGroupLayout] {
        &[]
    }

    fn shader_defs<'a>() -> &'a [ShaderDefVal] {
        &[]
    }
    fn push_constant_ranges<'a>() -> &'a [PushConstantRange] {
        &[]
    }

    fn entry_point<'a>() -> &'a str {
        "main"
    }
}

pub struct FinishedWorkerEvent(pub WorkerId);

impl From<WorkerId> for FinishedWorkerEvent {
    fn from(id: WorkerId) -> Self {
        Self(id)
    }
}
