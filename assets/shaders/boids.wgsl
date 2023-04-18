// https://github.com/gfx-rs/wgpu-rs/blob/master/examples/boids/compute.wgsl

struct Params {
    deltaT : f32,
    rule1Distance : f32,
    rule2Distance : f32,
    rule3Distance : f32,
    rule1Scale : f32,
    rule2Scale : f32,
    rule3Scale : f32
}

struct Boid {
    pos: vec2<f32>,
    vel: vec2<f32>
}

@group(0) @binding(0)
var<uniform> params: Params;


@group(0) @binding(1)
var<storage> boids_src: array<Boid>;
@group(0) @binding(2)
var<storage, read_write> boids_dst: array<Boid>;


@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) invocation_id: vec3<u32>) {

    // for reflection for now.
    let x = params;

    let total_boids = arrayLength(&boids_src);
    let index = invocation_id.x;

    if (index >= total_boids) {
        return;
    }

    var pos = boids_src[index].pos;
    var vel = boids_src[index].vel;

    var target_pos = pos + vel;

    if (target_pos.x < 0.) {
        target_pos.x = 1. + target_pos.x;
    } else if (target_pos.x > 1.) {
        target_pos.x = 1. - target_pos.x;
    }

    if (target_pos.y < 0.) {
        target_pos.y = 1. + target_pos.y;
    } else if (target_pos.y > 1.) {
        target_pos.y = 1. - target_pos.y;
    }

    boids_dst[index].pos = target_pos;
    
}