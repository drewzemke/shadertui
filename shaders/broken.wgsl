// Test shader demonstrating compilation error handling (intentionally broken)
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
    // This is intentionally broken - missing variable declaration
    let coords = undefined_variable;
    return vec3<f32>(1.0, 0.0, 0.0);
}