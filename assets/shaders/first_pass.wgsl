

@group(0) @binding(0)
var<uniform> value: f32;

@group(0) @binding(1)
var<storage> input: array<f32>;

@group(0) @binding(2)
var<storage, read_write> output: array<f32>;

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) invocation_id: vec3<u32>) {

    output[invocation_id.x] = input[invocation_id.x] + value;
}