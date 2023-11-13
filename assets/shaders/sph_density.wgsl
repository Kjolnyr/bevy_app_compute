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
var<storage, read_write> density: array<Density>;

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let num_particles = arrayLength(&particles_src);

    let i = invocation_id.x;
    if (i >= num_particles) {
        return;
    }

    var number_density_i = 0.0;
    let position_i = particles_src[i].position;

    var j: u32 = 0u;
    loop {
        if (j >= num_particles) {
            break;
        }
        let position_j = particles_src[j].position;
	let x_ij = position_i - position_j;
	
	number_density_i += cubic_spline_kernel(x_ij, params.length_scale);

        continuing {
            j = j + 1u;
        }
    }

    density[i].value = number_density_i;
    density[i].number = number_density_i;
}

const PI: f32 = 3.14159;

fn cubic_spline_kernel(r: vec2<f32>, h: f32) -> f32 {
  let q = length(r) / h;
  if (q < 1.0) {
      let normalizer = 40.0 / (7.0 * PI * h * h);
      if (q < 0.5) {
	  let q2 = q * q;
	  let value = 6.0 * (q2 * q - q2) + 1.0;
	  return value * normalizer;
      } else {
	let u = 1.0 - q;
	let value = 2.0 * u * u * u;
	return value * normalizer;
      }
    }
  return 0.0;
}
