use bevy::prelude::*;

use bevy_easy_compute::prelude::*;
use bytemuck::{Pod, Zeroable};

use rand::distributions::{Distribution, Uniform};

use crate::NUM_BOIDS;

#[derive(ShaderType, Pod, Zeroable, Clone, Copy)]
#[repr(C)]
struct Params {
    speed: f32,
    rule_1_distance: f32,
    rule_2_distance: f32,
    rule_3_distance: f32,
    rule_1_scale: f32,
    rule_2_scale: f32,
    rule_3_scale: f32,
}

#[derive(ShaderType, Pod, Zeroable, Clone, Copy)]
#[repr(C)]
pub struct Boid {
    pub pos: Vec2,
    pub vel: Vec2,
}

#[derive(TypePath)]
struct BoidsShader;

impl ComputeShader for BoidsShader {
    fn shader() -> ShaderRef {
        "shaders/boids.wgsl".into()
    }
}

pub struct BoidWorker;

impl ComputeWorker for BoidWorker {
    fn build(world: &mut World) -> AppComputeWorker<Self> {
        let params = Params {
            speed: 0.04,
            rule_1_distance: 0.1,
            rule_2_distance: 0.025,
            rule_3_distance: 0.025,
            rule_1_scale: 0.02,
            rule_2_scale: 0.05,
            rule_3_scale: 0.005,
        };

        let mut initial_boids_data = Vec::with_capacity(NUM_BOIDS as usize);
        let mut rng = rand::thread_rng();
        let unif = Uniform::new_inclusive(-1., 1.);

        for _ in 0..NUM_BOIDS {
            initial_boids_data.push(Boid {
                pos: Vec2::new(unif.sample(&mut rng), unif.sample(&mut rng)),
                vel: Vec2::new(
                    unif.sample(&mut rng) * params.speed,
                    unif.sample(&mut rng) * params.speed,
                ),
            });
        }

        AppComputeWorkerBuilder::new(world)
            .add_uniform("params", &params)
            .add_staging("boids_src", &initial_boids_data)
            .add_staging("boids_dst", &initial_boids_data)
            .add_pass::<BoidsShader>(
                [NUM_BOIDS / 64, 1, 1],
                &["params", "boids_src", "boids_dst"],
            )
            .add_swap("boids_src", "boids_dst")
            .build()
    }
}
