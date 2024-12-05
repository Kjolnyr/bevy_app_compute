use std::marker::PhantomData;

use bevy::{
    prelude::*,
    render::renderer::{RenderAdapter, RenderDevice},
};

use crate::{
    extract_shaders, pipeline_cache::AppPipelineCache, process_pipeline_queue_system,
    traits::ComputeWorker, worker::AppComputeWorker,
};

/// The main plugin. Always include it if you want to use `bevy_easy_compute`
pub struct AppComputePlugin;

impl Plugin for AppComputePlugin {
    fn build(&self, _app: &mut App) {}

    fn finish(&self, app: &mut App) {
        let render_device = app.world().resource::<RenderDevice>().clone();
        let render_adapter = app.world().resource::<RenderAdapter>().clone();

        app.configure_sets(Update, BevyEasyComputeSet::ExtractPipelines)
            .configure_sets(PostUpdate, BevyEasyComputePostUpdateSet::ExecuteCompute);

        app.insert_resource(AppPipelineCache::new(render_device, render_adapter))
            .add_systems(PreUpdate, extract_shaders)
            .add_systems(
                Update,
                process_pipeline_queue_system.in_set(BevyEasyComputeSet::ExtractPipelines),
            );
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Extracts data from GPU and enables it to be read
pub enum BevyEasyComputeSet {
    /// Extracts data from GPU and enables it to be read
    ExtractPipelines,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Sends needed data to the GPU and runs the compute shader
pub enum BevyEasyComputePostUpdateSet {
    /// Sends needed data to the GPU and runs the compute shader
    ///
    /// Shaders will also be checked for completion in this set
    ExecuteCompute,
}

/// Plugin to initialise your [`AppComputeWorker<W>`] structs.
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
    fn build(&self, _app: &mut App) {}

    fn finish(&self, app: &mut App) {
        let worker = W::build(app.world_mut());

        app.insert_resource(worker)
            .add_systems(
                Update,
                AppComputeWorker::<W>::extract_pipelines
                    .in_set(BevyEasyComputeSet::ExtractPipelines)
                    .after(process_pipeline_queue_system),
            )
            .add_systems(
                PostUpdate,
                (AppComputeWorker::<W>::unmap_all, AppComputeWorker::<W>::run)
                    .in_set(BevyEasyComputePostUpdateSet::ExecuteCompute)
                    .chain(),
            );
    }
}
