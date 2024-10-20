use bevy::{
    prelude::*,
    render::{
        extract_resource::ExtractResource,
        render_resource::{
            binding_types::{storage_buffer, uniform_buffer},
            BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, ShaderStages,
        },
        renderer::RenderDevice,
    },
};
use bevy_easy_compute::prelude::*;

use crate::worker::{
    GameOfLifeWorker, Settings, CELLS_IN_BUFFER, CELLS_OUT_BUFFER, SETTINGS_BUFFER,
};

/// The bind group layout for the minimal data needed to render particle
#[derive(Resource, ExtractResource, Clone)]
pub struct ParticleBindGroupLayout {
    /// The bind group layout itself
    pub bind_group_layout: BindGroupLayout,
}

impl FromWorld for ParticleBindGroupLayout {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let bind_group_layout = render_device.create_bind_group_layout(
            "ParticlesLayout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                (
                    uniform_buffer::<Settings>(false),
                    storage_buffer::<Vec<u32>>(false),
                    storage_buffer::<Vec<u32>>(false),
                ),
            ),
        );

        Self { bind_group_layout }
    }
}

/// The bind group data for rendering particles as pixels
#[derive(Resource, ExtractResource, Clone)]
pub struct ParticleBindGroup {
    /// The bind group itself
    pub bind_group: BindGroup,
}

pub fn get_buffers_for_renderer(world: &mut World) {
    world.init_resource::<ParticleBindGroupLayout>();
    let render_device = world.resource::<RenderDevice>();
    let bind_group_layout = world.resource::<ParticleBindGroupLayout>();
    let compute_worker = world.resource::<AppComputeWorker<GameOfLifeWorker>>();

    let bind_group = render_device.create_bind_group(
        None,
        &bind_group_layout.bind_group_layout,
        &BindGroupEntries::sequential((
            compute_worker
                .get_buffer(SETTINGS_BUFFER)
                .expect("Couldn't get settings buffer")
                .as_entire_binding(),
            compute_worker
                .get_buffer(CELLS_IN_BUFFER)
                .expect("Couldn't get cells in buffer")
                .as_entire_binding(),
            compute_worker
                .get_buffer(CELLS_OUT_BUFFER)
                .expect("Couldn't get cells out buffer")
                .as_entire_binding(),
        )),
    );

    let bindings = ParticleBindGroup { bind_group };
    world.insert_resource(bindings);
}
