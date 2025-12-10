@group(0) @binding(0)
var<uniform> time: f32;

@group(0) @binding(1)
var<storage, read_write> heights: array<f32>;

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let idx = invocation_id.x;
    if (idx < 121u) {
        let grid_size = 11u;
        let x = f32(idx % grid_size);
        let z = f32(idx / grid_size);
        
        heights[idx] = sin(x * 0.5 + time) * cos(z * 0.5 + time) * 0.1;
    }
}
