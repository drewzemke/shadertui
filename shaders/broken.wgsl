@group(0) @binding(0) var<storage, read_write> output: array<vec4<f32>>;
@group(0) @binding(1) var<uniform> uniforms: Uniforms;

struct Uniforms {
    resolution: vec2<f32>,
    time: f32,
    _padding: f32,
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    // This is intentionally broken - missing variable declaration
    let coords = undefined_variable;
    
    let index = u32(coords.y * uniforms.resolution.x + coords.x);
    output[index] = vec4<f32>(1.0, 0.0, 0.0, 1.0);
}