use std::marker::PhantomData;

use bevy::{
    prelude::*,
    render::{
        renderer::{self, RenderAdapter, RenderAdapterInfo, RenderDevice, RenderQueue},
        settings::WgpuSettings,
    },
};

use crate::{
    extract_shaders, pipeline::AppComputePipeline, pipeline_cache::AppPipelineCache,
    process_pipeline_queue_system, AppCompute, ComputeShader, WorkerEvent,
};

#[derive(Resource)]
pub struct ComputeInfo {
    pub(crate) device: RenderDevice,
    pub(crate) queue: RenderQueue
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

        if app.world.get_resource::<AppPipelineCache>().is_none() {
            app.insert_resource(AppPipelineCache::new(compute_info.device.clone()));
        }

        app.insert_resource(compute_info)
            .init_resource::<AppComputePipeline<C>>()
            .init_resource::<AppCompute<C>>()
            .add_event::<WorkerEvent<C>>()
            .add_system(AppCompute::<C>::process_tasks)
            .add_system(extract_shaders)
            .add_system(process_pipeline_queue_system::<C>);
    }
}
