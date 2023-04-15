

@group(0) @binding(0)
var<storage, read_write> my_storage: array<f32>;


fn index_from_coords(coords: vec3<u32>) -> u32 {
    return coords.z * 65535u * 65535u + coords.y * 65535u + coords.x;
}

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) invocation_id: vec3<u32>) {


    for (var i = 0; i < 1000000; i += 1) {
        my_storage[index_from_coords(invocation_id)] += f32(i);
    }

}