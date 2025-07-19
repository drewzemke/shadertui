@group(0) @binding(0) var<storage, read_write> output: array<vec4<f32>>;
@group(0) @binding(1) var<uniform> uniforms: Uniforms;

struct Uniforms {
    resolution: vec2<f32>,
    time: f32,
    _padding: f32,
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let coords = vec2<f32>(f32(id.x), f32(id.y));
    
    // Skip if we're outside the bounds
    if (coords.x >= uniforms.resolution.x || coords.y >= uniforms.resolution.y) {
        return;
    }
    
    // Create normalized coordinates (0-1)
    let uv = coords / uniforms.resolution;
    
    // Different pattern - spiral colors
    let center = vec2<f32>(0.5, 0.5);
    let dist = distance(uv, center);
    let angle = atan2(uv.y - center.y, uv.x - center.x);
    
    let color = vec3<f32>(
        0.5 + 0.5 * sin(dist * 20.0 + uniforms.time),
        0.5 + 0.5 * cos(angle * 5.0 + uniforms.time * 0.5),
        0.5 + 0.5 * sin(uniforms.time * 2.0)
    );
    
    // Clamp to [0, 1] range
    let final_color = vec3<f32>(
        clamp(color.r, 0.0, 1.0),
        clamp(color.g, 0.0, 1.0),
        clamp(color.b, 0.0, 1.0)
    );
    
    // Write to output buffer
    let index = u32(coords.y * uniforms.resolution.x + coords.x);
    output[index] = vec4<f32>(final_color, 1.0);
}