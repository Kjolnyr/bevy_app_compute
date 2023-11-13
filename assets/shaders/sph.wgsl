struct Parameters {
    speed: f32,
    gravity_multiplier: f32,
    rest_density: f32,
    gas_constant: f32,
    viscosity: f32,
    length_scale: f32,
    particle_radius: f32,
    particle_area: f32,
    delta_time: f32,
    max_pressure_grad: f32,
    x_min: f32,
    x_max: f32,
    y_min: f32,
    y_max: f32,
}

struct Particle {
    position: vec2<f32>,
    velocity: vec2<f32>,
}

struct Density {
    value: f32,
    number: f32,
}

@group(0) @binding(0)
var<uniform> params: Parameters;

@group(0) @binding(1)
var<storage, read> particles_src: array<Particle>;
@group(0) @binding(2)
var<storage, read> density: array<Density>;
@group(0) @binding(3)
var<storage, read_write> particles_dst: array<Particle>;

const gas_constant: f32 = 200000000000.0; 
const rest_density: f32 = 0.031; 

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let num_particles = arrayLength(&particles_src);

    let i = invocation_id.x;
    if (i >= num_particles) {
        return;
    }

    var velocity_i = particles_src[i].velocity + vec2<f32>(0.0, -params.gravity_multiplier) * params.delta_time;
    var position_i = particles_src[i].position + vec2<f32>(0.0, -params.gravity_multiplier) * params.delta_time * params.delta_time;

    // var velocity_i = particles_src[i].velocity;
    // var position_i = particles_src[i].position;

    let density_i = density[i];
    var pressure_i = vec2<f32>(0.0, 0.0);

    let point_pressure_i = gas_constant * (density_i.value / rest_density);
    let normalized_point_pressure_i = point_pressure_i / (density_i.number * density_i.number + 1e-4);


    var j: u32 = 0u;
    loop {
        if (j >= num_particles) {
            break;
        }

        if (j == i) {
            continue;
        }

        let position_j = particles_src[j].position;
        let velocity_j = particles_src[j].velocity;
	let density_j = density[j];

	let x_ij = position_i - position_j;
	let grad_w_ij = spiky_kernel_grad(x_ij, params.length_scale * 0.8);

	let point_pressure_j = gas_constant * (density_j.value / rest_density);
	let normalized_point_pressure_j = point_pressure_j / (density_j.number * density_j.number + 1e-4);

	pressure_i = pressure_i + (normalized_point_pressure_i + normalized_point_pressure_j) * grad_w_ij;
   
	// let overlap = 2.0 * params.particle_radius - distance(position_i, position_j);
        // if overlap > 0.0 {
	//     velocity_i = ((velocity_i - dot(velocity_i - velocity_j, position_i - position_j) /
	// 		   (length(position_i - position_j) * length(position_i - position_j) + 1e-4) * (position_i - position_j))); 
	//   // velocity_i = 0.98 * velocity_i;
	//   // position_i = position_i + 0.9 * normalize(position_i - position_j) * (overlap + 1e-4) / 2.0;  
        // }

        // if (distance(pos, position_i) < params.rule2Distance) {
        //     colVel = colVel - (pos - position_i);
        // }
        // if (distance(pos, position_i) < params.rule3Distance) {
        //     cVel = cVel + vel;
        //     cVelCount = cVelCount + 1;
        // }

        continuing {
            j = j + 1u;
        }
    }

    pressure_i = normalize(pressure_i) * clamp(length(pressure_i), 0.0, 1000.0);

    // pressure_i.grad +=
    //   (normalized_point_pressure_i + normalized_point_pressure_j) * grad_w_ij;
    // pressure_i.grad = pressure_i.grad.clamp_length_max(params.max_pressure_grad);
    

    // kinematic update
    velocity_i = velocity_i + pressure_i * params.delta_time;
    position_i = position_i + velocity_i * params.delta_time; 

    // velocity_i = velocity_i + vec2<f32>(0.0, -params.gravity_multiplier) * params.speed;
    // clamp velocity for a more pleasing simulation
    // velocity_i = normalize(velocity_i) * clamp(length(velocity_i), 0.0, 100.0 * params.speed); 
    
    // Wrap around boundary
    if (position_i.x < params.x_min) {
        position_i.x = params.x_min + (params.x_min - position_i.x);
	velocity_i.x = -velocity_i.x;
    }
    if (position_i.x > params.x_max) {
        position_i.x = params.x_max + (params.x_max - position_i.x);
	velocity_i.x = -velocity_i.x;
    }
    if (position_i.y < params.y_min) {
        position_i.y = params.y_min + (params.y_min - position_i.y);
	velocity_i.y = -velocity_i.y;
    }
    if (position_i.y > params.y_max) {
        position_i.y = params.y_max + (params.y_max - position_i.y);
	velocity_i.y = -velocity_i.y;
    }
 
    // Write back
    particles_dst[i].position = position_i;
    particles_dst[i].velocity = velocity_i;
}


const PI: f32 = 3.14159;


fn spiky_kernel_grad(r: vec2<f32>, h: f32) -> vec2<f32> {
  let length = length(r);

  if (length < h * h) {
    let grad_normalizer = 30.0 / (PI * pow(h, 5.0));
      let deviation = h - length;
      return -grad_normalizer * deviation * deviation * r / (length + 1e-5);
  } 

  return vec2<f32>(0.0, 0.0);
}
