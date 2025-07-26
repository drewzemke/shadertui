// Advanced utilities that depend on other utility files
// @import "utils.wgsl"
// @import "math.wgsl"

fn advanced_pattern(uv: vec2<f32>, time: f32) -> vec3<f32> {
    let centered_uv = uv - 0.5;
    let rotated_uv = rotate2d(time * 0.3) * centered_uv;
    
    let domain_warped = rotated_uv + fbm(rotated_uv * 4.0 + time * 0.5) * 0.2;
    let noise1 = fbm(domain_warped * 6.0);
    let noise2 = fbm(domain_warped * 12.0 + vec2<f32>(100.0, 50.0));
    
    let combined = smin(noise1, noise2, 0.3);
    
    return palette(
        combined + time * 0.1,
        vec3<f32>(0.5, 0.5, 0.5),
        vec3<f32>(0.5, 0.5, 0.5),
        vec3<f32>(2.0, 1.0, 0.0),
        vec3<f32>(0.5, 0.20, 0.25)
    );
}