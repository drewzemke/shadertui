@group(0) @binding(0) var<storage, read_write> output: array<vec4<f32>>;
@group(0) @binding(1) var<uniform> uniforms: Uniforms;

struct Uniforms {
    resolution: vec2<f32>,    // Terminal resolution (cols, rows*2)
    cursor: vec2<f32>,       // Cursor position (x, y)
    time: f32,               // Seconds since start
    frame: u32,              // Frame number
    delta_time: f32,         // Time since last frame
    _padding: f32,           // Ensure 16-byte alignment
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
    
    // Cursor position as normalized coordinates
    let cursor_uv = uniforms.cursor / uniforms.resolution;
    
    // Distance from cursor position
    let cursor_dist = distance(uv, cursor_uv);
    
    // Interactive cursor ripple effect
    let ripple = sin(cursor_dist * 30.0 - uniforms.time * 5.0) * exp(-cursor_dist * 3.0);
    
    // Frame-based color cycling
    let frame_factor = f32(uniforms.frame % 360u) / 360.0;
    
    // Delta time visualization (brightness oscillation)
    let delta_brightness = 0.8 + 0.2 * sin(uniforms.delta_time * 100.0);
    
    // Combined color effect
    let color = vec3<f32>(
        0.5 + 0.3 * sin(uv.x * 10.0 + uniforms.time) + ripple * 0.3,
        0.5 + 0.3 * cos(6.28) + ripple * 0.2,
        0.5 + 0.3 * sin(uv.y * 8.0 + uniforms.time * 0.7) + ripple * 0.4
    ) * delta_brightness;
    
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
