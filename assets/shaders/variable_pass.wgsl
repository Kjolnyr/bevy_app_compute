//Shader to count workgroups

@group(0) @binding(0)
var<storage, read_write> counter : atomic<u32>;

@compute @workgroup_size(1, 1, 1)
fn main(
    @builtin(local_invocation_id) lid : vec3<u32>
) {
    if (lid.x == 0u && lid.y == 0u && lid.z == 0u) {
        atomicAdd(&counter, 1u);
    }
}