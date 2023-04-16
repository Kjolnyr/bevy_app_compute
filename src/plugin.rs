use std::marker::PhantomData;

use bevy::{prelude::*, render::renderer::RenderDevice};

use crate::{
    extract_shaders, pipeline_cache::AppPipelineCache, process_pipeline_queue_system,
    traits::ComputeWorker, worker::AppComputeWorker, FinishedWorkerEvent,
};

pub struct AppComputePlugin;

impl Plugin for AppComputePlugin {
    fn build(&self, app: &mut App) {
        let render_device = app.world.resource::<RenderDevice>().clone();

        app.insert_resource(AppPipelineCache::new(render_device))
            .add_event::<FinishedWorkerEvent>()
            .add_system(extract_shaders.in_base_set(CoreSet::PreUpdate))
            .add_system(process_pipeline_queue_system);
    }
}

pub struct AppComputeWorkerPlugin<W: ComputeWorker> {
    _phantom: PhantomData<W>,
}

impl<W: ComputeWorker> Default for AppComputeWorkerPlugin<W> {
    fn default() -> Self {
        Self {
            _phantom: Default::default(),
        }
    }
}

impl<W: ComputeWorker> Plugin for AppComputeWorkerPlugin<W> {
    fn build(&self, app: &mut App) {
        let worker = W::build(&mut app.world);

        app.insert_resource(worker)
            .add_system(AppComputeWorker::<W>::extract_pipelines)
            .add_system(AppComputeWorker::<W>::run.in_base_set(CoreSet::PostUpdate))
            .add_system(
                AppComputeWorker::<W>::unmap_all
                    .in_base_set(CoreSet::PostUpdate)
                    .before(AppComputeWorker::<W>::run),
            );
    }
}