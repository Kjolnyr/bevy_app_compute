use std::marker::PhantomData;

use bevy::{prelude::*, render::renderer::RenderDevice};

use crate::{
    extract_shaders, pipeline::AppComputePipeline, pipeline_cache::AppPipelineCache,
    process_pipeline_queue_system, AppCompute, ComputeShader, WorkerEvent,
};

pub struct AppComputePlugin<C: ComputeShader> {
    _phantom: PhantomData<C>,
}

impl<C: ComputeShader> Default for AppComputePlugin<C> {
    fn default() -> Self {
        Self {
            _phantom: Default::default(),
        }
    }
}

impl<C: ComputeShader> Plugin for AppComputePlugin<C> {
    fn build(&self, app: &mut App) {
        if app.world.get_resource::<AppPipelineCache>().is_none() {
            let render_device = app.world.resource::<RenderDevice>().clone();
            app.insert_resource(AppPipelineCache::new(render_device));
        }

        app.init_resource::<AppComputePipeline<C>>()
            .init_resource::<AppCompute<C>>()
            .add_event::<WorkerEvent<C>>()
            .add_system(AppCompute::<C>::process_tasks)
            .add_system(extract_shaders)
            .add_system(process_pipeline_queue_system::<C>);
    }
}
