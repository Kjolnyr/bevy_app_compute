use std::{future::IntoFuture, marker::PhantomData};

use bevy::{
    prelude::*,
    render::{
        render_resource::ComputePipeline,
        renderer::{RenderDevice, RenderQueue},
    },
    tasks::{AsyncComputeTaskPool, Task},
};
use futures_lite::future;

use crate::{worker::AppComputeWorker, ComputeShader, WorkerEvent};


// Struct responsible for creating new workers and processing tasks
// It requires <C> so that we don't mix tasks between different <C> 
#[derive(Resource)]
pub struct AppCompute<C: ComputeShader> {
    render_device: RenderDevice,
    render_queue: RenderQueue,
    pub(crate) pipeline: Option<ComputePipeline>,
    tasks: Vec<Task<AppComputeWorker>>,
    _phantom: PhantomData<C>,
}

impl<C: ComputeShader> FromWorld for AppCompute<C> {
    fn from_world(world: &mut bevy::prelude::World) -> Self {
        let render_device = world.resource::<RenderDevice>().clone();
        let render_queue = world.resource::<RenderQueue>().clone();

        Self {
            render_device,
            render_queue,
            pipeline: None,
            tasks: vec![],
            _phantom: PhantomData::default(),
        }
    }
}

impl<C: ComputeShader> AppCompute<C> {
    pub fn worker(&self) -> Option<AppComputeWorker> {
        if let Some(pipeline) = &self.pipeline {
            // Probably could avoid cloning with some cursed lifetime rust code
            Some(AppComputeWorker::new(
                self.render_device.clone(),
                self.render_queue.clone(),
                pipeline.clone(),
            ))
        } else {
            None
        }
    }

    // Add a new compute tasks to the queue, this allow running compute shaders without blocking the main thread
    pub fn queue(&mut self, mut worker: AppComputeWorker, workgroups: (u32, u32, u32)) {
        let pool = AsyncComputeTaskPool::get();

        let task = pool.spawn(async move {
            worker.run(workgroups);
            worker
        });

        self.tasks.push(task);
    }

    // Process the tasks and send an event once finished with the data
    pub fn process_tasks(
        mut app_compute: ResMut<Self>,
        mut worker_events: EventWriter<WorkerEvent<C>>,
    ) {
        if app_compute.tasks.is_empty() {
            return;
        }

        let mut indices_to_remove = vec![];

        for (idx, task) in &mut app_compute.tasks.iter_mut().enumerate() {
            let Some(worker) = future::block_on(future::poll_once(task.into_future())) else { continue; };

            worker_events.send(WorkerEvent::new(worker));

            indices_to_remove.push(idx);
        }

        for idx in indices_to_remove {
            let _ = app_compute.tasks.remove(idx);
        }
    }
}
