@group(0) @binding(0) var output_texture: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(1) var<uniform> uniforms: Uniforms;

struct Uniforms {
    resolution: vec2<f32>,    // Window resolution (width, height)
    cursor: vec2<f32>,       // Cursor position (x, y)
    time: f32,               // Seconds since start
    frame: u32,              // Frame number
    delta_time: f32,         // Time since last frame
    _padding: f32,           // Ensure 16-byte alignment
}

// USER_SHADER_INJECTION_POINT

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let coords = vec2<f32>(f32(id.x), f32(id.y));
    
    // Skip if we're outside the bounds
    if (coords.x >= uniforms.resolution.x || coords.y >= uniforms.resolution.y) {
        return;
    }
    
    // Create normalized coordinates (0-1)
    let uv = coords / uniforms.resolution;
    
    // Call user's compute_color function
    let final_color = compute_color(uv);
    
    // Write to texture
    textureStore(output_texture, vec2<i32>(i32(coords.x), i32(coords.y)), vec4<f32>(final_color, 1.0));
}