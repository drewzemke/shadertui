// Test shader demonstrating multiple imports
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

// @import "utils.wgsl"
// @import "math.wgsl"

fn compute_color(uv: vec2<f32>) -> vec3<f32> {
    let centered_uv = uv - 0.5;
    
    let rotated_uv = rotate2d(uniforms.time * 0.5) * centered_uv;
    let noise_value = fbm(rotated_uv * 6.0 + uniforms.time * 0.3);
    
    return palette(
        noise_value + uniforms.time * 0.2,
        vec3<f32>(0.5, 0.5, 0.5),
        vec3<f32>(0.5, 0.5, 0.5),
        vec3<f32>(1.0, 1.0, 1.0),
        vec3<f32>(0.0, 0.33, 0.67)
    );
}