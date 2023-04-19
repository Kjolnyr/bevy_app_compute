// https://github.com/gfx-rs/wgpu-rs/blob/master/examples/boids/compute.wgsl

struct Params {
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
var<uniform> delta_time: f32;

@group(0) @binding(2)
var<storage> boids_src: array<Boid>;
@group(0) @binding(3)
var<storage, read_write> boids_dst: array<Boid>;


@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) invocation_id: vec3<u32>) {

    let total_boids = arrayLength(&boids_src);
    let index = invocation_id.x;

    if (index >= total_boids) {
        return;
    }

    var vPos = boids_src[index].pos;
    var vVel = boids_src[index].vel;

    var cMass : vec2<f32> = vec2<f32>(0.0, 0.0);
    var cVel : vec2<f32> = vec2<f32>(0.0, 0.0);
    var colVel : vec2<f32> = vec2<f32>(0.0, 0.0);
    var cMassCount : i32 = 0;
    var cVelCount : i32 = 0;

    var pos: vec2<f32>;
    var vel: vec2<f32>;

    var i: u32 = 0u;

    loop {
        if (i >= total_boids) {
            break;
        }
        if (i == index) {
            continue;
        }

        pos = boids_src[i].pos;
        vel = boids_src[i].vel;

        if (distance(pos, vPos) < params.rule1Distance) {
            cMass = cMass + pos;
            cMassCount = cMassCount + 1;
        }
        if (distance(pos, vPos) < params.rule2Distance) {
            colVel = colVel - (pos - vPos);
        }
        if (distance(pos, vPos) < params.rule3Distance) {
            cVel = cVel + vel;
            cVelCount = cVelCount + 1;
        }

        continuing {
            i = i + 1u;
        }
    }

    if (cMassCount > 0) {
        cMass = cMass * (1.0 / f32(cMassCount)) - vPos;
    }

    if (cVelCount > 0) {
        cVel = cVel * (1.0 / f32(cVelCount));
    }

    vVel = vVel + (cMass * params.rule1Scale) +
        (colVel * params.rule2Scale) +
        (cVel * params.rule3Scale);

    // clamp velocity for a more pleasing simulation
    vVel = normalize(vVel) * clamp(length(vVel), 0.0, 0.1);

    // kinematic update
    vPos = vPos + (vVel * delta_time);
    

    // Wrap around boundary
    if (vPos.x < -1.0) {
        vPos.x = 1.0 + (1.0 + vPos.x);
    }
    if (vPos.x > 1.0) {
        vPos.x = -1.0 + (vPos.x - 1.0);
    }
    if (vPos.y < -1.0) {
        vPos.y = 1.0 + (1.0 + vPos.y);
    }
    if (vPos.y > 1.0) {
        vPos.y = -1.0 + (vPos.y - 1.0);
    }

    // Write back
    boids_dst[index].pos = vPos;
    boids_dst[index].vel = vVel;
    
}