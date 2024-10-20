use bevy::prelude::*;

use bevy_easy_compute::prelude::*;
use bytemuck::{Pod, Zeroable};
use rand::{distributions::Uniform, Rng};
use wgpu::BufferUsages;

use crate::DIMENSIONS;
pub const NUMBER_OF_CELLS: u32 = DIMENSIONS.0 * DIMENSIONS.1;
const WORKGROUP_SIZE: u32 = 8;

#[derive(ShaderType, Pod, Zeroable, Clone, Copy)]
#[repr(C)]
pub struct Settings {
    size: UVec2,
}

#[derive(TypePath)]
struct GameOfLifeShader;

impl ComputeShader for GameOfLifeShader {
    fn shader() -> ShaderRef {
        "shaders/game_of_life.wgsl".into()
    }
}

pub const SETTINGS_BUFFER: &str = "settings_buffer";
pub const CELLS_IN_BUFFER: &str = "cells_in_buffer";
pub const CELLS_OUT_BUFFER: &str = "cells_out_buffer";

pub struct GameOfLifeWorker;

impl ComputeWorker for GameOfLifeWorker {
    fn build(world: &mut World) -> AppComputeWorker<Self> {
        let settings = Settings {
            size: UVec2::new(DIMENSIONS.0, DIMENSIONS.1),
        };

        let mut initial_cell_data = Vec::with_capacity(NUMBER_OF_CELLS as usize);
        let mut rng = rand::thread_rng();
        let range = Uniform::new(0.0, 1.0);

        info!("Generating {NUMBER_OF_CELLS} random cells...");
        for _ in 0..NUMBER_OF_CELLS {
            let random = rng.sample(range);
            if random > 0.75 {
                initial_cell_data.push(1);
            } else {
                initial_cell_data.push(0);
            }
        }
        info!("...done");

        AppComputeWorkerBuilder::new(world)
            // Allow buffers to be visible in the render pass.
            // But it doesn't seem that this is actually needed for rendering?
            .set_extra_buffer_usages(Some(BufferUsages::VERTEX))
            //
            // Create buffers
            .add_uniform(SETTINGS_BUFFER, &settings)
            .add_storage(CELLS_IN_BUFFER, &initial_cell_data)
            .add_storage(CELLS_OUT_BUFFER, &initial_cell_data)
            .add_pass::<GameOfLifeShader>(
                [
                    DIMENSIONS.0 / WORKGROUP_SIZE,
                    DIMENSIONS.1 / WORKGROUP_SIZE,
                    1,
                ],
                &[SETTINGS_BUFFER, CELLS_IN_BUFFER, CELLS_OUT_BUFFER],
            )
            .add_swap(CELLS_IN_BUFFER, CELLS_OUT_BUFFER)
            .build()
    }
}
