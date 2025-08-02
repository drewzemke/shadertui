// Example user shader using the new shell architecture
//
// Available uniforms (automatically provided by shell):
//   uniforms.resolution: vec2<f32>  - Screen resolution (width, height in pixels)  
//   uniforms.cursor: vec2<f32>      - Cursor position (x, y in pixels)
//   uniforms.time: f32              - Time since start (seconds)
//   uniforms.frame: u32             - Frame number since start
//   uniforms.delta_time: f32        - Time since last frame (seconds)
//
// Your compute_color function receives:
//   uv: vec2<f32> - Normalized coordinates (0.0 to 1.0)

fn compute_color(uv: vec2<f32>) -> vec3<f32> {
    // Cursor position as normalized coordinates
    let cursor_uv = uniforms.cursor / uniforms.resolution;
    
    // Distance from cursor position
    let cursor_dist = distance(uv, cursor_uv);
    
    // Interactive cursor ripple effect
    let ripple = sin(cursor_dist * 30.0 - uniforms.time * 5.0) * exp(-cursor_dist * 3.0);
    
    // Frame-based color cycling
    let frame_factor = f32(uniforms.frame % 360u) / 360.0;
    
    // Combined color effect
    let color = vec3<f32>(
        0.5 + 0.3 * sin(uv.x * 10.0 + uniforms.time) + ripple * 0.3,
        0.5 + 0.3 * cos(6.28) + ripple * 0.2,
        0.5 + 0.3 * sin(uv.y * 8.0 + uniforms.time * 0.7) + ripple * 0.4
    );
    
    // Return clamped color
    return clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));
}
