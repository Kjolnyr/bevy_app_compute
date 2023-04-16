use std::{borrow::Cow, marker::PhantomData};

use bevy::{
    prelude::{AssetServer, World},
    render::{
        render_resource::{
            encase::{private::WriteInto, StorageBuffer, UniformBuffer},
            Buffer, ComputePipelineDescriptor, ShaderRef, ShaderType,
        },
        renderer::RenderDevice,
    },
    utils::{HashMap, Uuid},
};
use wgpu::{util::BufferInitDescriptor, BufferDescriptor, BufferUsages};

use crate::{
    pipeline_cache::{AppPipelineCache, CachedAppComputePipelineId},
    traits::{ComputeShader, ComputeWorker},
    worker::{AppComputeWorker, ComputePass, StaggingBuffers},
};

pub struct AppComputeWorkerBuilder<'a, W: ComputeWorker> {
    pub(crate) world: &'a mut World,
    pub(crate) cached_pipeline_ids: HashMap<Uuid, CachedAppComputePipelineId>,
    pub(crate) buffers: HashMap<String, Buffer>,
    pub(crate) staging_buffers: HashMap<String, StaggingBuffers>,
    pub(crate) passes: Vec<ComputePass>,
    _phantom: PhantomData<W>,
}

impl<'a, W: ComputeWorker> AppComputeWorkerBuilder<'a, W> {
    pub fn new(world: &'a mut World) -> Self {
        Self {
            world,
            cached_pipeline_ids: HashMap::default(),
            buffers: HashMap::default(),
            staging_buffers: HashMap::default(),
            passes: vec![],
            _phantom: PhantomData::default(),
        }
    }

    pub fn add_uniform<T: ShaderType + WriteInto>(&mut self, name: &str, uniform: &T) -> &mut Self {
        T::assert_uniform_compat();

        let mut buffer = UniformBuffer::new(Vec::new());
        buffer.write::<T>(uniform).unwrap();

        let render_device = self.world.resource::<RenderDevice>();

        self.buffers.insert(
            name.to_owned(),
            render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some(name),
                contents: buffer.as_ref(),
                usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            }),
        );
        self
    }

    pub fn add_storage<T: ShaderType + WriteInto>(&mut self, name: &str, storage: &T) -> &mut Self {
        let mut buffer = StorageBuffer::new(Vec::new());
        buffer.write::<T>(storage).unwrap();

        let render_device = self.world.resource::<RenderDevice>();

        self.buffers.insert(
            name.to_owned(),
            render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some(name),
                contents: buffer.as_ref(),
                usage: BufferUsages::COPY_DST | BufferUsages::STORAGE,
            }),
        );
        self
    }

    pub fn add_rw_storage<T: ShaderType + WriteInto>(
        &mut self,
        name: &str,
        storage: &T,
    ) -> &mut Self {
        let mut buffer = StorageBuffer::new(Vec::new());
        buffer.write::<T>(storage).unwrap();

        let render_device = self.world.resource::<RenderDevice>();

        self.buffers.insert(
            name.to_owned(),
            render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some(name),
                contents: buffer.as_ref(),
                usage: BufferUsages::COPY_DST | BufferUsages::COPY_SRC | BufferUsages::STORAGE,
            }),
        );
        self
    }

    pub fn add_staging<T: ShaderType + WriteInto>(
        &mut self,
        name: &str,
        location: &T,
    ) -> &mut Self {
        self.add_rw_storage(name, location);

        let buffer = self.buffers.get(name).unwrap();

        let render_device = self.world.resource::<RenderDevice>();

        self.staging_buffers.insert(
            name.to_owned(),
            StaggingBuffers::new(&render_device, buffer.size()),
        );

        self
    }

    pub fn add_empty_storage(&mut self, name: &str, size: u64) -> &mut Self {
        let render_device = self.world.resource::<RenderDevice>();

        self.buffers.insert(
            name.to_owned(),
            render_device.create_buffer(&BufferDescriptor {
                label: Some(name),
                size,
                usage: BufferUsages::COPY_DST | BufferUsages::COPY_SRC | BufferUsages::STORAGE,
                mapped_at_creation: false,
            }),
        );
        self
    }

    pub fn add_pass<S: ComputeShader>(&mut self, workgroups: [u32; 3], vars: &[&str]) -> &mut Self {
        let pipeline_cache = self.world.resource::<AppPipelineCache>();

        let asset_server = self.world.resource::<AssetServer>();
        let shader = match S::shader() {
            ShaderRef::Default => None,
            ShaderRef::Handle(handle) => Some(handle),
            ShaderRef::Path(path) => Some(asset_server.load(path)),
        }
        .unwrap();

        let cached_id = pipeline_cache.queue_app_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: S::layouts().to_vec(),
            push_constant_ranges: S::push_constant_ranges().to_vec(),
            shader_defs: S::shader_defs().to_vec(),
            entry_point: Cow::Borrowed(S::entry_point()),
            shader,
        });

        self.cached_pipeline_ids.insert(S::TYPE_UUID, cached_id);

        self.passes.push(ComputePass {
            workgroups,
            vars: vars.into_iter().map(|a| String::from(*a)).collect(),
            shader_uuid: S::TYPE_UUID,
        });
        self
    }

    pub fn build(&self) -> AppComputeWorker<W> {
        AppComputeWorker::from(self)
    }
}
