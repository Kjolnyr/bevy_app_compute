//! GPU storage buffer extraction utilities.
//!
//! # Background
//!
//! Bevy allows users to define storage buffers on custom materials via the
//! `#[storage]` attribute on properties with type [`Handle<ShaderStorageBuffer>`].
//! However, Bevy maps this handle to a [`Buffer`] in the render world.
//!
//! The problem is that `bevy_app_compute` was designed to run in the main world,
//! so we need to extract [`Buffer`]s back to the main world. The bulk of the actual
//! data sits on the GPU though, so we're really just cloning handles around.

use bevy::{
    platform::collections::HashMap,
    prelude::*,
    render::{
        MainWorld,
        render_asset::RenderAssets,
        render_resource::Buffer,
        storage::{GpuShaderStorageBuffer, ShaderStorageBuffer},
    },
};

/// GPU buffers created via [`ShaderStorageBuffer`] assets, available in the main world.
///
/// This is essentially [`RenderAssets<GpuShaderStorageBuffer>`] but extracted to
/// the main world so compute workers can access prepared GPU buffers.
#[derive(Resource, Default)]
pub struct MainWorldShaderStorageBufferAssets {
    buffers: HashMap<AssetId<ShaderStorageBuffer>, Buffer>,
}

impl MainWorldShaderStorageBufferAssets {
    /// Gets the GPU buffer for the given asset ID.
    pub fn get(&self, id: &AssetId<ShaderStorageBuffer>) -> Option<&Buffer> {
        self.buffers.get(id)
    }

    /// Inserts a GPU buffer for the given asset ID.
    pub(crate) fn insert(&mut self, id: AssetId<ShaderStorageBuffer>, buffer: Buffer) {
        self.buffers.insert(id, buffer);
    }
}

/// Extracts GPU storage buffers from the render world to the main world.
///
/// This system runs in the render app's [`ExtractSchedule`] and copies buffer
/// handles (not data) to make them accessible to compute workers in the main world.
///
/// Note: Cloning [`Buffer`] is cheap as it only clones a handle/reference to the
/// GPU resource, not the actual GPU memory.
pub(crate) fn extract_gpu_storage_buffers_to_main(
    mut main_world: ResMut<MainWorld>,
    gpu_buffers: Res<RenderAssets<GpuShaderStorageBuffer>>,
) {
    let mut main_buffers = MainWorldShaderStorageBufferAssets::default();

    for (id, gpu_buffer) in gpu_buffers.iter() {
        // Cloning Buffer is cheap - it's just a handle to GPU memory
        main_buffers.insert(id.clone(), gpu_buffer.buffer.clone());
    }

    main_world.insert_resource(main_buffers);
}
