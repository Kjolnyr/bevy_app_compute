use std::marker::PhantomData;

use app_compute::AppCompute;
use bevy::{
    prelude::{AssetEvent, Assets, EventReader, Res, ResMut, Shader},
    render::render_resource::ShaderRef,
};

use pipeline::AppComputePipeline;
use pipeline_cache::AppPipelineCache;
use worker::AppComputeWorker;

mod app_compute;
mod pipeline;
mod pipeline_cache;
mod plugin;
mod worker;

pub mod prelude {
    pub use crate::{
        app_compute::AppCompute, plugin::AppComputePlugin, ComputeShader, WorkerEvent,
    };
}

pub fn process_pipeline_queue_system<C: ComputeShader>(
    mut pipeline_cache: ResMut<AppPipelineCache>,
    app_pipeline: Res<AppComputePipeline<C>>,
    mut app_compute: ResMut<AppCompute<C>>,
) {
    pipeline_cache.process_queue();

    if app_compute.pipeline.is_some() {
        return;
    }

    app_compute.pipeline = pipeline_cache
        .get_compute_pipeline(app_pipeline.app_compute_pipeline)
        .cloned();
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

pub trait ComputeShader: Send + Sync + 'static {
    fn shader() -> ShaderRef;

    fn entry_point<'a>() -> &'a str {
        "main"
    }
}

pub struct WorkerEvent<C: ComputeShader> {
    pub worker: AppComputeWorker,
    _phantom: PhantomData<C>,
}

impl<C: ComputeShader> WorkerEvent<C> {
    pub fn new(worker: AppComputeWorker) -> Self {
        Self {
            worker,
            _phantom: PhantomData::default(),
        }
    }
}
