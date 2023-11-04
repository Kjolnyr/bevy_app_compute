use core::panic;
use std::{marker::PhantomData, ops::Deref};

use bevy::{
    prelude::{Res, ResMut, Resource},
    render::{
        render_resource::{Buffer, ComputePipeline},
        renderer::{RenderDevice, RenderQueue},
    },
    utils::{HashMap, Uuid},
};
use bytemuck::{bytes_of, cast_slice, from_bytes, AnyBitPattern, NoUninit};
use wgpu::{
    BindGroupDescriptor, BindGroupEntry, CommandEncoder, CommandEncoderDescriptor,
    ComputePassDescriptor,
};

use crate::{
    error::{Error, Result},
    pipeline_cache::{AppPipelineCache, CachedAppComputePipelineId},
    traits::ComputeWorker,
    worker_builder::AppComputeWorkerBuilder,
};

#[derive(PartialEq, Clone, Copy)]
pub enum RunMode {
    Continuous,
    OneShot(bool),
}

#[derive(PartialEq)]
pub enum WorkerState {
    Created,
    Available,
    Working,
    FinishedWorking,
}

#[derive(Clone, Debug)]
pub(crate) enum Step {
    ComputePass(ComputePass),
    Swap(String, String),
}

#[derive(Clone, Debug)]
pub(crate) struct ComputePass {
    pub(crate) workgroups: [u32; 3],
    pub(crate) vars: Vec<String>,
    pub(crate) shader_uuid: Uuid,
}

#[derive(Clone, Debug)]
pub(crate) struct StagingBuffer {
    pub(crate) mapped: bool,
    pub(crate) buffer: Buffer,
}

/// Struct to manage data transfers from/to the GPU
/// it also handles the logic of your compute work.
/// By default, the run mode of the workers is set to continuous,
/// meaning it will run every frames. If you want to run it deterministically
/// use the function `one_shot()` in the builder
#[derive(Resource)]
pub struct AppComputeWorker<W: ComputeWorker> {
    pub(crate) state: WorkerState,
    render_device: RenderDevice,
    render_queue: RenderQueue,
    cached_pipeline_ids: HashMap<Uuid, CachedAppComputePipelineId>,
    pipelines: HashMap<Uuid, Option<ComputePipeline>>,
    buffers: HashMap<String, Buffer>,
    staging_buffers: HashMap<String, StagingBuffer>,
    steps: Vec<Step>,
    command_encoder: Option<CommandEncoder>,
    run_mode: RunMode,
    _phantom: PhantomData<W>,
}

impl<W: ComputeWorker> From<&AppComputeWorkerBuilder<'_, W>> for AppComputeWorker<W> {
    /// Create a new [`AppComputeWorker<W>`].
    fn from(builder: &AppComputeWorkerBuilder<W>) -> Self {
        let render_device = builder.world.resource::<RenderDevice>().clone();
        let render_queue = builder.world.resource::<RenderQueue>().clone();

        let pipelines = builder
            .cached_pipeline_ids
            .iter()
            .map(|(uuid, _)| (*uuid, None))
            .collect();

        let command_encoder =
            Some(render_device.create_command_encoder(&CommandEncoderDescriptor { label: None }));

        Self {
            state: WorkerState::Created,
            render_device,
            render_queue,
            cached_pipeline_ids: builder.cached_pipeline_ids.clone(),
            pipelines,
            buffers: builder.buffers.clone(),
            staging_buffers: builder.staging_buffers.clone(),
            steps: builder.steps.clone(),
            command_encoder,
            run_mode: builder.run_mode,
            _phantom: PhantomData::default(),
        }
    }
}

impl<W: ComputeWorker> AppComputeWorker<W> {
    #[inline]
    fn dispatch(&mut self, index: usize) -> Result<()> {
        let compute_pass = match &self.steps[index] {
            Step::ComputePass(compute_pass) => compute_pass,
            Step::Swap(_, _) => return Err(Error::InvalidStep(format!("{:?}", self.steps[index]))),
        };

        let mut entries = vec![];
        for (index, var) in compute_pass.vars.iter().enumerate() {
            let Some(buffer) = self
                    .buffers
                    .get(var)
                    else { return Err(Error::BufferNotFound(var.to_owned())) };

            let entry = BindGroupEntry {
                binding: index as u32,
                resource: buffer.as_entire_binding(),
            };

            entries.push(entry);
        }

        let Some(maybe_pipeline) = self
                .pipelines
                .get(&compute_pass.shader_uuid)
                else { return Err(Error::PipelinesEmpty) };

        let Some(pipeline) = maybe_pipeline else {
                return Err(Error::PipelineNotReady);
            };

        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = self.render_device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &entries,
        });

        let Some(encoder) = &mut self.command_encoder else { return Err(Error::EncoderIsNone) };
        {
            let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor { label: None });
            cpass.set_pipeline(&pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            cpass.dispatch_workgroups(
                compute_pass.workgroups[0],
                compute_pass.workgroups[1],
                compute_pass.workgroups[2],
            )
        }

        Ok(())
    }

    #[inline]
    fn swap(&mut self, index: usize) -> Result<()> {
        let (buf_a_name, buf_b_name) = match &self.steps[index] {
            Step::ComputePass(_) => {
                return Err(Error::InvalidStep(format!("{:?}", self.steps[index])))
            }
            Step::Swap(a, b) => (a.as_str(), b.as_str()),
        };

        if !self.buffers.contains_key(buf_a_name) {
            return Err(Error::BufferNotFound(buf_a_name.to_owned()));
        }

        if !self.buffers.contains_key(buf_b_name) {
            return Err(Error::BufferNotFound(buf_b_name.to_owned()));
        }

        let [buffer_a, buffer_b] = self.buffers.get_many_mut([buf_a_name, buf_b_name]).unwrap();
        std::mem::swap(buffer_a, buffer_b);

        Ok(())
    }

    #[inline]
    fn read_staging_buffers(&mut self) -> Result<&mut Self> {
        for (name, staging_buffer) in &self.staging_buffers {
            let Some(encoder) = &mut self.command_encoder else { return Err(Error::EncoderIsNone); };
            let Some(buffer) = self
                .buffers
                .get(name)
                else { return Err(Error::BufferNotFound(name.to_owned()))};

            encoder.copy_buffer_to_buffer(
                &buffer,
                0,
                &staging_buffer.buffer,
                0,
                staging_buffer.buffer.size(),
            );
        }
        Ok(self)
    }

    #[inline]
    fn map_staging_buffers(&mut self) -> &mut Self {
        for (_, staging_buffer) in self.staging_buffers.iter_mut() {
            let read_buffer_slice = staging_buffer.buffer.slice(..);

            read_buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                let err = result.err();
                if err.is_some() {
                    let some_err = err.unwrap();
                    panic!("{}", some_err.to_string());
                }
            });

            staging_buffer.mapped = true;
        }
        self
    }

    /// Read data from `target` staging buffer, return raw bytes
    #[inline]
    pub fn try_read_raw<'a>(&'a self, target: &str) -> Result<(impl Deref<Target = [u8]> + 'a)> {
        let Some(staging_buffer) = &self
            .staging_buffers
            .get(target)
            else { return Err(Error::StagingBufferNotFound(target.to_owned()))};

        let result = staging_buffer.buffer.slice(..).get_mapped_range();

        Ok(result)
    }

    /// Read data from `target` staging buffer, return raw bytes
    /// Panics on error.
    #[inline]
    pub fn read_raw<'a>(&'a self, target: &str) -> (impl Deref<Target = [u8]> + 'a) {
        self.try_read_raw(target).unwrap()
    }

    /// Try Read data from `target` staging buffer, return a single `B: Pod`
    #[inline]
    pub fn try_read<'a, B: AnyBitPattern>(&'a self, target: &str) -> Result<B> {
        let result = from_bytes::<B>(&self.try_read_raw(target)?).clone();
        Ok(result)
    }

    /// Try Read data from `target` staging buffer, return a single `B: Pod`
    /// In case of error, this function will panic.
    #[inline]
    pub fn read<B: AnyBitPattern>(&self, target: &str) -> B {
        self.try_read(target).unwrap()
    }

    /// Try Read data from `target` staging buffer, return a vector of `B: Pod`
    #[inline]
    pub fn try_read_vec<B: AnyBitPattern>(&self, target: &str) -> Result<Vec<B>> {
        let bytes = self.try_read_raw(target)?;
        Ok(cast_slice::<u8, B>(&bytes).to_vec())
    }

    /// Try Read data from `target` staging buffer, return a vector of `B: Pod`
    /// In case of error, this function will panic.
    #[inline]
    pub fn read_vec<B: AnyBitPattern>(&self, target: &str) -> Vec<B> {
        self.try_read_vec(target).unwrap()
    }

    /// Write data to `target` buffer.
    #[inline]
    pub fn try_write<T: NoUninit>(&mut self, target: &str, data: &T) -> Result<()> {
        let Some(buffer) = &self
            .buffers
            .get(target)
            else { return Err(Error::BufferNotFound(target.to_owned())) };

        let bytes = bytes_of(data);

        self.render_queue.write_buffer(buffer, 0, bytes);

        Ok(())
    }

    /// Write data to `target` buffer.
    /// In case of error, this function will panic.
    #[inline]
    pub fn write<T: NoUninit>(&mut self, target: &str, data: &T) {
        self.try_write(target, data).unwrap()
    }

    /// Write data to `target` buffer.
    #[inline]
    pub fn try_write_slice<T: NoUninit>(&mut self, target: &str, data: &[T]) -> Result<()> {
        let Some(buffer) = &self
            .buffers
            .get(target)
            else { return Err(Error::BufferNotFound(target.to_owned())) };

        let bytes = cast_slice(data);

        self.render_queue.write_buffer(buffer, 0, bytes);

        Ok(())
    }

    /// Write data to `target` buffer.
    /// In case of error, this function will panic.
    #[inline]
    pub fn write_slice<T: NoUninit>(&mut self, target: &str, data: &[T]) {
        self.try_write_slice(target, data).unwrap()
    }

    fn submit(&mut self) -> &mut Self {
        let encoder = self.command_encoder.take().unwrap();
        self.render_queue.submit(Some(encoder.finish()));
        self.state = WorkerState::Working;
        self
    }

    #[inline]
    fn poll(&self) -> bool {
        self.render_device
            .wgpu_device()
            .poll(wgpu::MaintainBase::Wait)
    }

    /// Check if the worker is ready to be read from.
    #[inline]
    pub fn ready(&self) -> bool {
        self.state == WorkerState::FinishedWorking
    }

    /// Tell the worker to execute the compute shader at the end of the current frame
    #[inline]
    pub fn execute(&mut self) {
        match self.run_mode {
            RunMode::Continuous => {}
            RunMode::OneShot(_) => self.run_mode = RunMode::OneShot(true),
        }
    }

    #[inline]
    fn ready_to_execute(&self) -> bool {
        (self.state != WorkerState::Working) && (self.run_mode != RunMode::OneShot(false))
    }

    pub(crate) fn run(mut worker: ResMut<Self>) {
        if worker.ready() {
            worker.state = WorkerState::Available;
        }

        if worker.ready_to_execute() {
            // Workaround for interior mutability
            for i in 0..worker.steps.len() {
                let result = match worker.steps[i] {
                    Step::ComputePass(_) => worker.dispatch(i),
                    Step::Swap(_, _) => worker.swap(i),
                };

                if let Err(err) = result {
                    match err {
                        Error::PipelineNotReady => return,
                        _ => panic!("{:?}", err),
                    }
                }
            }

            worker.read_staging_buffers().unwrap();
            worker.submit();
            worker.map_staging_buffers();
        }

        if worker.run_mode != RunMode::OneShot(false) && worker.poll() {
            worker.state = WorkerState::FinishedWorking;
            worker.command_encoder = Some(
                worker
                    .render_device
                    .create_command_encoder(&CommandEncoderDescriptor { label: None }),
            );

            match worker.run_mode {
                RunMode::Continuous => {}
                RunMode::OneShot(_) => worker.run_mode = RunMode::OneShot(false),
            };
        }
    }

    pub(crate) fn unmap_all(mut worker: ResMut<Self>) {
        for (_, staging_buffer) in &mut worker.staging_buffers {
            if staging_buffer.mapped {
                staging_buffer.buffer.unmap();
                staging_buffer.mapped = false;
            }
        }
    }

    pub(crate) fn extract_pipelines(
        mut worker: ResMut<Self>,
        pipeline_cache: Res<AppPipelineCache>,
    ) {
        for (uuid, cached_id) in &worker.cached_pipeline_ids.clone() {
            let Some(pipeline) = worker.pipelines.get(uuid) else { continue; };

            if pipeline.is_some() {
                continue;
            };

            let cached_id = *cached_id;

            worker.pipelines.insert(
                *uuid,
                pipeline_cache.get_compute_pipeline(cached_id).cloned(),
            );
        }
    }
}
