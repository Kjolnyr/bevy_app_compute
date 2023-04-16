use core::panic;

use bevy::{
    prelude::Resource,
    render::{
        render_resource::{
            encase::{private::WriteInto, StorageBuffer, UniformBuffer},
            Buffer, ComputePipeline, ShaderType,
        },
        renderer::{RenderDevice, RenderQueue},
    },
    utils::{HashMap, Uuid},
};
use wgpu::{
    util::BufferInitDescriptor, BindGroupDescriptor, BindGroupEntry, BufferDescriptor,
    BufferUsages, CommandEncoder, CommandEncoderDescriptor, ComputePassDescriptor, SubmissionIndex,
};

use crate::ComputeShader;

#[derive(Clone, Copy, PartialEq)]
pub struct WorkerId(pub(crate) Uuid);

impl From<Uuid> for WorkerId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

#[derive(PartialEq)]
pub enum WorkerState {
    Created,
    Working,
    Finished,
}

// This struct is the core of this plugin
// It's goal is to create the context to send to the GPU (uniforms, storages)
// It's also responsible for mapping buffers in order to get data back from the GPU
#[derive(Resource)]
pub struct AppComputeWorker {
    pub id: WorkerId,
    pub(crate) state: WorkerState,
    render_device: RenderDevice,
    render_queue: RenderQueue,
    pipelines: HashMap<Uuid, Option<ComputePipeline>>,
    buffers: HashMap<String, Buffer>,
    staging_buffers: HashMap<String, (String, Buffer)>,
    command_encoder: Option<CommandEncoder>,
    pub(crate) submission_index: Option<SubmissionIndex>,
}

impl AppComputeWorker {
    pub(crate) fn new(
        id: WorkerId,
        render_device: RenderDevice,
        render_queue: RenderQueue,
        pipelines: HashMap<Uuid, Option<ComputePipeline>>,
    ) -> Self {
        let command_encoder =
            Some(render_device.create_command_encoder(&CommandEncoderDescriptor { label: None }));

        Self {
            id,
            state: WorkerState::Created,
            render_device,
            render_queue,
            pipelines,
            buffers: HashMap::default(),
            staging_buffers: HashMap::default(),
            command_encoder,
            submission_index: None,
        }
    }

    pub fn get_buffer_by_name(&self, name: &str) -> Option<&Buffer> {
        self.buffers.get(name)
    }

    pub fn add_uniform<T: ShaderType + WriteInto>(&mut self, name: &str, uniform: T) -> &mut Self {
        T::assert_uniform_compat();

        let mut buffer = UniformBuffer::new(Vec::new());
        buffer.write::<T>(&uniform).unwrap();

        self.buffers.insert(
            name.to_owned(),
            self.render_device
                .create_buffer_with_data(&BufferInitDescriptor {
                    label: Some(name),
                    contents: buffer.as_ref(),
                    usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
                }),
        );
        self
    }

    pub fn add_storage<T: ShaderType + WriteInto>(&mut self, name: &str, storage: T) -> &mut Self {
        let mut buffer = StorageBuffer::new(Vec::new());
        buffer.write::<T>(&storage).unwrap();

        self.buffers.insert(
            name.to_owned(),
            self.render_device
                .create_buffer_with_data(&BufferInitDescriptor {
                    label: Some(name),
                    contents: buffer.as_ref(),
                    usage: BufferUsages::COPY_DST | BufferUsages::COPY_SRC | BufferUsages::STORAGE,
                }),
        );
        self
    }

    pub fn pass<C: ComputeShader>(&mut self, workgroups: [u32; 3], vars: &[&str]) -> &mut Self {
        let mut entries = vec![];
        for (index, var) in vars.iter().enumerate() {
            let Some(buffer) = self.buffers.get(*var) else { panic!("Couldn't find {var} in self.buffers."); };

            let entry = BindGroupEntry {
                binding: index as u32,
                resource: buffer.as_entire_binding(),
            };

            entries.push(entry);
        }

        let Some(maybe_pipeline) = self.pipelines.get(&C::TYPE_UUID) else { panic!("No pipeline in app_compute.pipelines"); };

        //TODO: Handle that, that's not panic material.
        let Some(pipeline) = maybe_pipeline else { panic!("Pipeline isn't ready yet."); };

        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = self.render_device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &entries,
        });

        let Some(encoder) = &mut self.command_encoder else { panic!("Unable to unwrap encoder!"); };
        {
            let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor { label: None });
            cpass.set_pipeline(&pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            cpass.dispatch_workgroups(workgroups[0], workgroups[1], workgroups[2])
        }
        self
    }

    pub fn read(&mut self, target: &str) {
        let Some((from, staging_buffer)) = &self.staging_buffers.get(target) else { return; };

        let Some(encoder) = &mut self.command_encoder else { return; };

        let Some(buffer) = self.buffers.get(from) else { panic!("Unable to find buffer {from}"); };

        encoder.copy_buffer_to_buffer(&buffer, 0, &staging_buffer, 0, staging_buffer.size());
    }

    pub fn read_staging_buffers(&mut self) -> &mut Self {
        for (_, (from_buffer_name, staging_buffer)) in &self.staging_buffers {
            let Some(encoder) = &mut self.command_encoder else { return self; };
            let Some(buffer) = self.buffers.get(from_buffer_name) else { panic!("Unable to find buffer {from_buffer_name}"); };
            encoder.copy_buffer_to_buffer(&buffer, 0, &staging_buffer, 0, staging_buffer.size());
        }
        self
    }

    pub fn map(&mut self, target: &str) {
        let Some((_,staging_buffer)) = &self.staging_buffers.get(target) else { return; };

        let buffer_slice = staging_buffer.slice(..);

        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let err = result.err();

            if err.is_some() {
                panic!("Error while map_async buffer");
            }
        });
    }

    pub fn map_staging_buffers(&mut self) -> &mut Self {
        for (_, (_, staging_buffer)) in self.staging_buffers.iter_mut() {
            let buffer_slice = staging_buffer.slice(..);

            buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                let err = result.err();
                if err.is_some() {
                    panic!("Error while map_async buffer");
                }
            });
        }
        self
    }

    pub fn get_data(&self, target: &str) -> Vec<u8> {
        let Some((_,staging_buffer)) = &self.staging_buffers.get(target) else { return vec![]; };

        let result = staging_buffer
            .slice(..)
            .get_mapped_range()
            .as_ref()
            .to_vec();

        staging_buffer.unmap();

        result
    }

    pub fn add_staging_buffer(&mut self, name: &str, from: &str) -> &mut Self {
        let Some(from_buffer) = self.buffers.get(from) else { panic!("Unable to find buffer {from}"); };

        let buffer = self.render_device.create_buffer(&BufferDescriptor {
            label: None,
            size: from_buffer.size(),
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        self.staging_buffers
            .insert(name.to_owned(), (from.to_owned(), buffer));
        self
    }

    pub fn submit(&mut self) -> &mut Self {
        let encoder = self.command_encoder.take().unwrap();
        let index = self.render_queue.submit(Some(encoder.finish()));
        self.submission_index = Some(index);
        self.state = WorkerState::Working;
        self
    }

    pub fn now(&mut self) -> &Self {
        self.render_device.wgpu_device().poll(wgpu::Maintain::Wait);
        self.state = WorkerState::Finished;
        self
    }
}
