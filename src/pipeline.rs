use std::{borrow::Cow, marker::PhantomData};

use bevy::{
    prelude::*,
    render::render_resource::{ComputePipelineDescriptor, ShaderRef},
};

use crate::ComputeShader;

use super::pipeline_cache::{AppPipelineCache, CachedAppComputePipelineId};


#[derive(Resource, Clone)]
pub struct AppComputePipeline<C: ComputeShader> {
    pub(crate) app_compute_pipeline: CachedAppComputePipelineId,
    _phantom: PhantomData<C>,
}

impl<C: ComputeShader> FromWorld for AppComputePipeline<C> {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        let shader = match C::shader() {
            ShaderRef::Default => None,
            ShaderRef::Handle(handle) => Some(handle),
            ShaderRef::Path(path) => Some(asset_server.load(path)),
        }
        .unwrap();

        let app_pipeline_cache = world.resource::<AppPipelineCache>();

        let app_compute_pipeline =
            app_pipeline_cache.queue_app_compute_pipeline(ComputePipelineDescriptor {
                label: None,
                layout: C::layouts().to_vec(),
                push_constant_ranges: C::push_constant_ranges().to_vec(),
                shader: shader,
                shader_defs: C::shader_defs().to_vec(),
                entry_point: Cow::from(C::entry_point()),
            });

        Self {
            app_compute_pipeline,
            _phantom: PhantomData::default(),
        }
    }
}
