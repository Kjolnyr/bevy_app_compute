use bevy::{
    prelude::World,
    reflect::TypeUuid,
    render::render_resource::{BindGroupLayout, ShaderDefVal, ShaderRef},
};
use wgpu::PushConstantRange;

use crate::worker::AppComputeWorker;

pub trait ComputeWorker: Sized + Send + Sync + 'static {
    fn build(world: &mut World) -> AppComputeWorker<Self>;
}

pub trait ComputeShader: TypeUuid + Send + Sync + 'static {
    fn shader() -> ShaderRef;

    fn layouts<'a>() -> &'a [BindGroupLayout] {
        &[]
    }

    fn shader_defs<'a>() -> &'a [ShaderDefVal] {
        &[]
    }
    fn push_constant_ranges<'a>() -> &'a [PushConstantRange] {
        &[]
    }

    fn entry_point<'a>() -> &'a str {
        "main"
    }
}
