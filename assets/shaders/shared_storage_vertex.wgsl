#import bevy_pbr::{
    mesh_functions,
    view_transformations::position_world_to_clip,
    forward_io::{Vertex, VertexOutput},
}

@group(2) @binding(100) 
var<storage, read> heights: array<f32>;

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

    let local_pos = vertex.position;

    let grid_size = 11u;
    let x_index = u32(vertex.uv.x * f32(grid_size - 1u));
    let z_index = u32(vertex.uv.y * f32(grid_size - 1u));
    let height_index = z_index * grid_size + x_index;
    
    let height = heights[height_index];

    var modified_position = local_pos;
    modified_position.y += height;

    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    out.world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local, 
        vec4<f32>(modified_position, 1.0)
    );
    out.position = position_world_to_clip(out.world_position.xyz);

    // compute normals from adjacent heights
    let hL = heights[max(0u, height_index - 1u)];
    let hR = heights[min(120u, height_index + 1u)];
    let hD = heights[max(0u, height_index - grid_size)];
    let hU = heights[min(120u, height_index + grid_size)];
    
    let normal = normalize(vec3<f32>(hL - hR, 2.0, hD - hU));
    
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        normal,
        vertex.instance_index
    );
    
    out.uv = vertex.uv;
    out.instance_index = vertex.instance_index;

    #ifdef VERTEX_TANGENTS
        out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
            world_from_local,
            vertex.tangent,
            vertex.instance_index
        );
    #endif

    return out;
}
