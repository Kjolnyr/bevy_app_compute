use bevy::{
    prelude::*,
    render::{
        extract_resource::ExtractResource,
        render_resource::{
            BindGroup, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries,
            PipelineCache, ShaderStages,
            binding_types::{storage_buffer, uniform_buffer},
        },
        renderer::RenderDevice,
    },
};
use bevy_app_compute::prelude::*;

use crate::worker::{
    CELLS_IN_BUFFER, CELLS_OUT_BUFFER, GameOfLifeWorker, SETTINGS_BUFFER, Settings,
};

/// The bind group layout for the minimal data needed to render particle
#[derive(Resource, ExtractResource, Clone)]
pub struct ParticleBindGroupLayout {
    /// The bind group layout descriptor
    pub bind_group_descriptor: BindGroupLayoutDescriptor,
}

impl FromWorld for ParticleBindGroupLayout {
    fn from_world(_: &mut World) -> Self {
        let bind_group_descriptor = BindGroupLayoutDescriptor::new(
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

        Self {
            bind_group_descriptor,
        }
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
    let pipeline_cache = world.resource::<PipelineCache>();

    let bind_group = render_device.create_bind_group(
        None,
        &pipeline_cache.get_bind_group_layout(&bind_group_layout.bind_group_descriptor),
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
