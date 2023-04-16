use std::{borrow::Cow, marker::PhantomData};

use bevy::{
    prelude::*,
    render::{
        render_resource::{ComputePipelineDescriptor, ShaderRef},
        renderer::{self, RenderAdapter, RenderAdapterInfo, RenderDevice, RenderQueue},
        settings::WgpuSettings,
    },
};

use crate::{
    extract_shaders, pipeline_cache::AppPipelineCache, process_pipeline_queue_system, AppCompute,
    ComputeShader, FinishedWorkerEvent,
};

#[derive(Resource)]
pub struct ComputeInfo {
    pub(crate) device: RenderDevice,
    pub(crate) queue: RenderQueue,
}

impl From<(RenderDevice, RenderQueue, RenderAdapterInfo, RenderAdapter)> for ComputeInfo {
    fn from(value: (RenderDevice, RenderQueue, RenderAdapterInfo, RenderAdapter)) -> Self {
        Self {
            device: value.0,
            queue: value.1,
        }
    }
}

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
            let wgpu_settings = WgpuSettings::default();

            let Some(backends) = wgpu_settings.backends else { return; };

            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                backends,
                dx12_shader_compiler: wgpu_settings.dx12_shader_compiler.clone(),
            });

            let request_adapter_options = wgpu::RequestAdapterOptions {
                power_preference: wgpu_settings.power_preference,
                compatible_surface: None,
                ..Default::default()
            };

            let compute_info: ComputeInfo = futures_lite::future::block_on(
                renderer::initialize_renderer(&instance, &wgpu_settings, &request_adapter_options),
            )
            .into();

            app.insert_resource(AppPipelineCache::new(compute_info.device.clone()));
            app.insert_resource(compute_info);

            // We assume these has never been registered if AppComputeCache is none
            app.add_system(extract_shaders.in_base_set(CoreSet::PreUpdate))
                .add_system(process_pipeline_queue_system);
        }

        if app
            .world
            .get_resource::<Events<FinishedWorkerEvent>>()
            .is_none()
        {
            app.add_event::<FinishedWorkerEvent>();
        }

        if app.world.get_resource::<AppCompute>().is_none() {
            app.init_resource::<AppCompute>()
                .add_system(AppCompute::poll_render_device)
                .add_system(AppCompute::remove_finished_workers.in_base_set(CoreSet::PostUpdate));
        }

        let pipeline_cache = app.world.resource::<AppPipelineCache>();

        let asset_server = app.world.resource::<AssetServer>();
        let shader = match C::shader() {
            ShaderRef::Default => None,
            ShaderRef::Handle(handle) => Some(handle),
            ShaderRef::Path(path) => Some(asset_server.load(path)),
        }
        .unwrap();

        let cached_id = pipeline_cache.queue_app_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: C::layouts().to_vec(),
            push_constant_ranges: C::push_constant_ranges().to_vec(),
            shader_defs: C::shader_defs().to_vec(),
            entry_point: Cow::Borrowed(C::entry_point()),
            shader,
        });

        let mut app_compute = app.world.resource_mut::<AppCompute>();
        app_compute
            .cached_pipeline_ids
            .insert(C::TYPE_UUID, cached_id);

        app_compute.pipelines.insert(C::TYPE_UUID, None);

        app.add_system(AppCompute::extract_pipeline::<C>);
    }
}
