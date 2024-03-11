use bevy::{
    prelude::World,
    reflect::TypePath,
    render::render_resource::{BindGroupLayout, ShaderDefVal, ShaderRef},
};
use wgpu::PushConstantRange;

use crate::worker::AppComputeWorker;

/// Trait to declare [`AppComputeWorker<W>`] structs.
pub trait ComputeWorker: Sized + Send + Sync + 'static {
    fn build(world: &mut World) -> AppComputeWorker<Self>;
}

/// Trait to declare your shaders.
pub trait ComputeShader: TypePath + Send + Sync + 'static {
    /// Implement your [`ShaderRef`]
    ///
    /// Usually, it comes from a path:
    /// ```
    /// fn shader() -> ShaderRef {
    ///     "shaders/my_shader.wgsl".into()
    /// }
    /// ```
    fn shader() -> ShaderRef;

    /// If you don't want to use wgpu's reflection for
    /// your binding layout, you can declare them here.
    fn layouts<'a>() -> &'a [BindGroupLayout] {
        &[]
    }

    fn shader_defs<'a>() -> &'a [ShaderDefVal] {
        &[]
    }
    fn push_constant_ranges<'a>() -> &'a [PushConstantRange] {
        &[]
    }

    /// By default, the shader entry point is `main`.
    /// You can change it from here.
    fn entry_point<'a>() -> &'a str {
        "main"
    }
}
