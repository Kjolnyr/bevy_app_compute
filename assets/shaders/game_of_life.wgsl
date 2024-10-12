struct Settings {
    /// Dimensions of the view onto the simulation
    size: vec2<u32>,
}

@binding(0) @group(0) var<uniform> settings: Settings;
@binding(1) @group(0) var<storage, read> current: array<u32>;
@binding(2) @group(0) var<storage, read_write> next: array<u32>;

const blockSize = 8;

fn getIndex(x: u32, y: u32) -> u32 {
    let h = settings.size.y;
    let w = settings.size.x;

    return (y % h) * w + (x % w);
}

fn getCell(x: u32, y: u32) -> u32 {
    return current[getIndex(x, y)];
}

fn countNeighbors(x: u32, y: u32) -> u32 {
    return getCell(x - 1, y - 1) + getCell(x, y - 1) + getCell(x + 1, y - 1) + getCell(x - 1, y) + getCell(x + 1, y) + getCell(x - 1, y + 1) + getCell(x, y + 1) + getCell(x + 1, y + 1);
}

@compute @workgroup_size(blockSize, blockSize)
fn main(@builtin(global_invocation_id) grid: vec3u) {
    let x = grid.x;
    let y = grid.y;
    let n = countNeighbors(x, y);
    next[getIndex(x, y)] = select(u32(n == 3u), u32(n == 2u || n == 3u), getCell(x, y) == 1u);
}

struct VertexInput {
    @builtin(vertex_index) index: u32,
    @builtin(instance_index) instance: u32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vertex(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    var local_position: vec2<f32>;
    var pixel_size = 0.4;
    // The GPU view target is in the range: `[-1.0, -1.0, 1.0, 1.0]`. So here we scale the viewport
    // coordinates to that.
    var factor: vec2<f32> = 1.0 / (vec2(f32(settings.size.x), f32(settings.size.y)) / 2.0);

    let index = square_indices[input.index];
    local_position = square_vertices[index] * factor * pixel_size;
    let x_coordinate = input.instance % settings.size.x;
    let y_coordinate = input.instance / settings.size.x;
    let position = vec2(f32(x_coordinate), f32(y_coordinate));
    let particle_position = (position * factor) - 1.0;
    let view_position = vec4<f32>(particle_position + local_position, 0.0, 1.0);
    out.position = view_position;

    let cell = next[input.instance];
    if cell == 1 {
        out.color = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    } else {
        out.color = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }
    return out;
}

@fragment
fn fragment(@location(0) color: vec4<f32>) -> @location(0) vec4<f32> {
    return color;
}

var<private> square_vertices: array<vec2<f32>, 4> = array<vec2<f32>, 4>(
    vec2<f32>(-1, -1),
    vec2<f32>(1, -1),
    vec2<f32>(-1, 1),
    vec2<f32>(1, 1),
);

var<private> square_indices: array<u32, 6> = array<u32, 6>(
    0, 1, 2,
    1, 3, 2
);
