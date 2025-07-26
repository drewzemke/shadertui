// This creates a circular dependency with test_circular_a.wgsl
// @import "test_circular_a.wgsl"

fn function_from_b(uv: vec2<f32>) -> vec3<f32> {
    return vec3<f32>(uv.x, uv.y, 0.5);
}