use bevy::{
    render::{
        render_resource::{
            encase::{private::WriteInto, StorageBuffer, UniformBuffer},
            Buffer, ComputePipeline, ShaderType,
        },
        renderer::{RenderDevice, RenderQueue},
    },
    utils::HashMap,
};
use wgpu::{
    util::BufferInitDescriptor, BindGroupDescriptor, BindGroupEntry, BufferDescriptor,
    BufferUsages, CommandEncoder, CommandEncoderDescriptor, ComputePassDescriptor,
};


// This struct is the core of this plugin
// It's goal is to create the context to send to the GPU (uniforms, storages)
// It's also responsible for mapping buffers in order to get data back from the GPU
pub struct AppComputeWorker {
    render_device: RenderDevice,
    render_queue: RenderQueue,
    pipeline: ComputePipeline,
    layouts: HashMap<usize, Vec<(String, Buffer)>>,
    staging_buffers: HashMap<String, (String, Buffer)>,
    command_encoder: Option<CommandEncoder>,
}

impl AppComputeWorker {
    pub fn new(
        render_device: RenderDevice,
        render_queue: RenderQueue,
        pipeline: ComputePipeline,
    ) -> Self {
        let command_encoder =
            Some(render_device.create_command_encoder(&CommandEncoderDescriptor { label: None }));

        Self {
            render_device,
            render_queue,
            pipeline,
            layouts: HashMap::default(),
            staging_buffers: HashMap::default(),
            command_encoder,
        }
    }

    pub fn get_buffer_by_name(&self, name: &str) -> Option<&Buffer> {
        let result = self
            .layouts
            .values()
            .flat_map(|a| a.iter())
            .filter(|(var_name, _)| var_name == name)
            .next();

        if let Some((_, buffer)) = result {
            Some(buffer)
        } else {
            None
        }
    }

    pub fn add_uniform<T: ShaderType + WriteInto>(
        &mut self,
        layout_index: usize,
        name: &str,
        uniform: T,
    ) {
        T::assert_uniform_compat();
        let bindings = match self.layouts.entry(layout_index) {
            bevy::utils::hashbrown::hash_map::Entry::Occupied(o) => o.into_mut(),
            bevy::utils::hashbrown::hash_map::Entry::Vacant(v) => v.insert(vec![]),
        };

        let mut buffer = UniformBuffer::new(Vec::new());
        buffer.write::<T>(&uniform).unwrap();

        bindings.push((
            name.to_owned(),
            self.render_device
                .create_buffer_with_data(&BufferInitDescriptor {
                    label: None,
                    contents: buffer.as_ref(),
                    usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
                }),
        ))
    }

    pub fn add_storage<T: ShaderType + WriteInto>(
        &mut self,
        layout_index: usize,
        name: &str,
        storage: T,
    ) {
        let bindings = match self.layouts.entry(layout_index) {
            bevy::utils::hashbrown::hash_map::Entry::Occupied(o) => o.into_mut(),
            bevy::utils::hashbrown::hash_map::Entry::Vacant(v) => v.insert(vec![]),
        };

        let mut buffer = StorageBuffer::new(Vec::new());
        buffer.write::<T>(&storage).unwrap();

        bindings.push((
            name.to_owned(),
            self.render_device
                .create_buffer_with_data(&BufferInitDescriptor {
                    label: None,
                    contents: buffer.as_ref(),
                    usage: BufferUsages::COPY_DST | BufferUsages::COPY_SRC | BufferUsages::STORAGE,
                }),
        ));
    }

    pub fn run(&mut self, workgroups: (u32, u32, u32)) -> bool {
        let result = self.dispatch(workgroups);
        if !result {
            return result;
        };

        self.read_staging_buffers();
        self.submit();
        self.map_staging_buffers();
        self.poll();

        true
    }

    pub fn dispatch(&mut self, workgroups: (u32, u32, u32)) -> bool {
        let bindings = match self.layouts.entry(0) {
            bevy::utils::hashbrown::hash_map::Entry::Occupied(o) => o.into_mut(),
            bevy::utils::hashbrown::hash_map::Entry::Vacant(v) => v.insert(vec![]),
        };

        let mut entries = vec![];

        for (idx, (_, buffer)) in bindings.iter().enumerate() {
            entries.push(BindGroupEntry {
                binding: idx as u32,
                resource: buffer.as_entire_binding(),
            })
        }

        let bind_group_layout = self.pipeline.get_bind_group_layout(0);
        let bind_group = self.render_device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &entries,
        });

        let Some(encoder) = &mut self.command_encoder else { return false; };

        let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor::default());
        cpass.set_pipeline(&self.pipeline);
        cpass.set_bind_group(0, &bind_group, &[]);
        cpass.dispatch_workgroups(workgroups.0, workgroups.1, workgroups.2);

        true
    }

    pub fn read(&mut self, target: &str, size: u64) {
        let Some((from, staging_buffer)) = &self.staging_buffers.get(target) else { return; };

        let Some(encoder) = &mut self.command_encoder else { return; };

        let Some((_, buffer)) = self
            .layouts
            .values()
            .flat_map(|a| a.iter())
            .filter(|(var_name, _)| var_name == from)
            .next() else { return; };

        encoder.copy_buffer_to_buffer(&buffer, 0, &staging_buffer, 0, size);
    }

    pub fn read_staging_buffers(&mut self) {
        for (_, (from, staging_buffer)) in self.staging_buffers.iter_mut() {
            let Some(encoder) = &mut self.command_encoder else { return; };

            let Some((_, buffer)) = self
            .layouts
            .values()
            .flat_map(|a| a.iter())
            .filter(|(var_name, _)| var_name == from)
            .next() else { return; };

            encoder.copy_buffer_to_buffer(&buffer, 0, &staging_buffer, 0, buffer.size());
        }
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

    pub fn map_staging_buffers(&mut self) {
        for (_, (_, staging_buffer)) in self.staging_buffers.iter_mut() {
            let buffer_slice = staging_buffer.slice(..);

            buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                let err = result.err();

                if err.is_some() {
                    panic!("Error while map_async buffer");
                }
            });
        }
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

    pub fn add_staging_buffer(&mut self, name: &str, from: &str, size: usize) {
        let buffer = self.render_device.create_buffer(&BufferDescriptor {
            label: None,
            size: size as u64,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        self.staging_buffers
            .insert(name.to_owned(), (from.to_owned(), buffer));
    }

    pub fn submit(&mut self) {
        let encoder = self.command_encoder.take().unwrap();

        self.render_queue.submit(Some(encoder.finish()));
    }

    pub fn poll(&self) {
        self.render_device.wgpu_device().poll(wgpu::Maintain::Wait);
    }
}
