@group(0) @binding(0) var<storage, read_write> output: array<vec4<f32>>;
@group(0) @binding(1) var<uniform> uniforms: Uniforms;

struct Uniforms {
    resolution: vec2<f32>,
    cursor: vec2<f32>,
    time: f32,
    frame: u32,
    delta_time: f32,
    _padding: f32,
}

// @import "utils.wgsl"
// @import "math.wgsl"

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let coords = vec2<f32>(f32(id.x), f32(id.y));
    
    if (coords.x >= uniforms.resolution.x || coords.y >= uniforms.resolution.y) {
        return;
    }
    
    let uv = coords / uniforms.resolution;
    let centered_uv = uv - 0.5;
    
    let rotated_uv = rotate2d(uniforms.time * 0.5) * centered_uv;
    let noise_value = fbm(rotated_uv * 6.0 + uniforms.time * 0.3);
    
    let color = palette(
        noise_value + uniforms.time * 0.2,
        vec3<f32>(0.5, 0.5, 0.5),
        vec3<f32>(0.5, 0.5, 0.5),
        vec3<f32>(1.0, 1.0, 1.0),
        vec3<f32>(0.0, 0.33, 0.67)
    );
    
    let index = u32(coords.y * uniforms.resolution.x + coords.x);
    output[index] = vec4<f32>(color, 1.0);
}