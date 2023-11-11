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
    worker::{AppComputeWorker, ComputePass, RunMode, StagingBuffer, Step},
};

/// A builder struct to build [`AppComputeWorker<W>`]
/// from your structs implementing [`ComputeWorker`]
pub struct AppComputeWorkerBuilder<'a, W: ComputeWorker> {
    pub(crate) world: &'a mut World,
    pub(crate) cached_pipeline_ids: HashMap<Uuid, CachedAppComputePipelineId>,
    pub(crate) buffers: HashMap<String, Buffer>,
    pub(crate) staging_buffers: HashMap<String, StagingBuffer>,
    pub(crate) steps: Vec<Step>,
    pub(crate) run_mode: RunMode,
    _phantom: PhantomData<W>,
}

impl<'a, W: ComputeWorker> AppComputeWorkerBuilder<'a, W> {
    /// Create a new builder.
    ///
    /// Since it requests `&mut World`, you cannot create builders from non exclusive systems.
    pub fn new(world: &'a mut World) -> Self {
        Self {
            world,
            cached_pipeline_ids: HashMap::default(),
            buffers: HashMap::default(),
            staging_buffers: HashMap::default(),
            steps: vec![],
            run_mode: RunMode::Continuous,
            _phantom: PhantomData,
        }
    }

    /// Add a new uniform buffer to the worker, and fill it with `uniform`.
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

    /// Add a new storage buffer to the worker, and fill it with `storage`. It will be read only.
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

    /// Add a new read/write storage buffer to the worker, and fill it with `storage`.
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

    /// Create two staging buffers, one to read from and one to write to.
    /// Additionally, it will create a read/write storage buffer to access from
    /// your shaders.
    /// The buffer will be filled with `data`
    pub fn add_staging<T: ShaderType + WriteInto>(&mut self, name: &str, data: &T) -> &mut Self {
        self.add_rw_storage(name, data);
        let buffer = self.buffers.get(name).unwrap();

        let render_device = self.world.resource::<RenderDevice>();

        let staging = StagingBuffer {
            mapped: true,
            buffer: render_device.create_buffer(&BufferDescriptor {
                label: Some(name),
                size: buffer.size(),
                usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
                mapped_at_creation: true,
            }),
        };

        self.staging_buffers.insert(name.to_owned(), staging);

        self
    }

    /// Add a new empty uniform buffer to the worker.
    pub fn add_empty_uniform(&mut self, name: &str, size: u64) -> &mut Self {
        let render_device = self.world.resource::<RenderDevice>();

        self.buffers.insert(
            name.to_owned(),
            render_device.create_buffer(&BufferDescriptor {
                label: Some(name),
                size,
                usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
                mapped_at_creation: false,
            }),
        );

        self
    }

    /// Add a new empty storage buffer to the worker. It will be read only.
    pub fn add_empty_storage(&mut self, name: &str, size: u64) -> &mut Self {
        let render_device = self.world.resource::<RenderDevice>();

        self.buffers.insert(
            name.to_owned(),
            render_device.create_buffer(&BufferDescriptor {
                label: Some(name),
                size,
                usage: BufferUsages::COPY_DST | BufferUsages::STORAGE,
                mapped_at_creation: false,
            }),
        );
        self
    }

    /// Add a new empty read/write storage buffer to the worker.
    pub fn add_empty_rw_storage(&mut self, name: &str, size: u64) -> &mut Self {
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

    /// Create two staging buffers, one to read from and one to write to.
    /// Additionally, it will create a read/write storage buffer to access from
    /// your shaders.
    /// The buffer will empty.
    pub fn add_empty_staging(&mut self, name: &str, size: u64) -> &mut Self {
        self.add_empty_rw_storage(name, size);

        let buffer = self.buffers.get(name).unwrap();

        let render_device = self.world.resource::<RenderDevice>();

        let staging = StagingBuffer {
            mapped: true,
            buffer: render_device.create_buffer(&BufferDescriptor {
                label: Some(name),
                size: buffer.size(),
                usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
                mapped_at_creation: true,
            }),
        };

        self.staging_buffers.insert(name.to_owned(), staging);

        self
    }

    /// Add a new compute pass to your worker.
    /// They will run sequentially in the order you insert them.
    pub fn add_pass<S: ComputeShader>(&mut self, workgroups: [u32; 3], vars: &[&str]) -> &mut Self {
        if !self.cached_pipeline_ids.contains_key(&S::TYPE_UUID) {
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
        }

        self.steps.push(Step::ComputePass(ComputePass {
            workgroups,
            vars: vars.iter().map(|a| String::from(*a)).collect(),
            shader_uuid: S::TYPE_UUID,
        }));
        self
    }

    pub fn add_swap(&mut self, buffer_a: &str, buffer_b: &str) -> &mut Self {
        self.steps
            .push(Step::Swap(buffer_a.to_owned(), buffer_b.to_owned()));
        self
    }

    /// The worker will run every frames.
    /// This is the default mode.
    pub fn continuous(&mut self) -> &mut Self {
        self.run_mode = RunMode::Continuous;
        self
    }

    /// The worker will run when requested.
    pub fn one_shot(&mut self) -> &mut Self {
        self.run_mode = RunMode::OneShot(false);
        self
    }

    /// Build an [`AppComputeWorker<W>`] from this builder.
    pub fn build(&self) -> AppComputeWorker<W> {
        AppComputeWorker::from(self)
    }
}
