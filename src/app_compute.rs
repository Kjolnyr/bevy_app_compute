use bevy::{
    prelude::*,
    render::{
        render_resource::ComputePipeline,
        renderer::{RenderDevice, RenderQueue},
    },
    utils::{HashMap, Uuid},
};

use crate::{
    pipeline_cache::{AppPipelineCache, CachedAppComputePipelineId},
    plugin::ComputeInfo,
    worker::{AppComputeWorker, WorkerId, WorkerState},
    ComputeShader, FinishedWorkerEvent,
};

// Struct responsible for creating new workers and processing tasks
// It requires <C> so that we don't mix tasks between different <C>
#[derive(Resource)]
pub struct AppCompute {
    pub(crate) render_device: RenderDevice,
    pub(crate) render_queue: RenderQueue,
    pub(crate) cached_pipeline_ids: HashMap<Uuid, CachedAppComputePipelineId>,
    pub(crate) pipelines: HashMap<Uuid, Option<ComputePipeline>>,
    pub(crate) workers: Vec<AppComputeWorker>,
}

impl FromWorld for AppCompute {
    fn from_world(world: &mut bevy::prelude::World) -> Self {
        let compute_info = world.resource::<ComputeInfo>();

        Self {
            render_device: compute_info.device.clone(),
            render_queue: compute_info.queue.clone(),
            cached_pipeline_ids: HashMap::default(),
            pipelines: HashMap::default(),
            workers: vec![],
        }
    }
}

impl AppCompute {
    pub fn worker(&mut self) -> &mut AppComputeWorker {
        self.workers.push(AppComputeWorker::new(
            Uuid::new_v4().into(),
            self.render_device.clone(),
            self.render_queue.clone(),
            self.pipelines.clone(),
        ));

        self.workers.last_mut().unwrap()
    }

    pub fn get_worker(&self, id: WorkerId) -> Option<&AppComputeWorker> {
        for worker in &self.workers {
            if worker.id == id {
                return Some(worker);
            }
        }
        None
    }

    pub fn extract_pipeline<C: ComputeShader>(
        mut app_compute: ResMut<Self>,
        pipeline_cache: Res<AppPipelineCache>,
    ) {
        let Some(pipeline) = app_compute.pipelines.get(&C::TYPE_UUID) else { return; };
        if pipeline.is_some() {
            return;
        };

        let Some(cached_id) = app_compute.cached_pipeline_ids.get(&C::TYPE_UUID) else { return; };

        let cached_id = cached_id.clone();

        app_compute.pipelines.insert(
            C::TYPE_UUID,
            pipeline_cache.get_compute_pipeline(cached_id).cloned(),
        );
    }

    pub fn remove_finished_workers(
        mut app_compute: ResMut<Self>,
        mut worker_events: EventReader<FinishedWorkerEvent>,
    ) {
        for ev in worker_events.iter() {
            let id = &ev.0;

            app_compute.workers.retain_mut(|worker| worker.id != *id)
        }
    }

    pub fn poll_render_device(
        mut app_compute: ResMut<Self>,
        mut finished_worker_events: EventWriter<FinishedWorkerEvent>,
    ) {
        let mut finished_workers = vec![];
        for (idx, worker) in app_compute.workers.iter().enumerate() {
            let Some(index) = &worker.submission_index else { continue; };
            app_compute
                .render_device
                .wgpu_device()
                .poll(wgpu::MaintainBase::WaitForSubmissionIndex(index.clone()));

            finished_workers.push(idx);
            finished_worker_events.send(worker.id.into());
        }

        for idx in finished_workers {
            app_compute.workers[idx].state = WorkerState::Finished;
        }
    }
}
