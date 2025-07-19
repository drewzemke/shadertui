use bytemuck::{Pod, Zeroable};

// AIDEV-NOTE: WGSL uniform buffer alignment requirements are strict!
// - vec2<f32> fields must be aligned to 8-byte boundaries
// - The total struct size must be a multiple of 16 bytes for uniforms
// - Field ordering matters: putting vec2<f32> fields together avoids implicit padding
// - Original issue: time:f32 followed by cursor:vec2<f32> created implicit padding
// - Solution: group vec2<f32> fields together, then scalar fields, then explicit padding
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Uniforms {
    pub resolution: [f32; 2], // Terminal resolution (cols, rows*2)  
    pub cursor: [f32; 2],     // Cursor position (x, y)
    pub time: f32,            // Seconds since start
    pub frame: u32,           // Frame number  
    pub delta_time: f32,      // Time since last frame
    pub _padding: f32,        // Ensure 16-byte alignment
}

impl Uniforms {
    pub fn new(
        width: u32,
        height: u32,
        time: f32,
        cursor: [i32; 2],
        frame: u32,
        delta_time: f32,
    ) -> Self {
        Self {
            resolution: [width as f32, height as f32],
            cursor: [cursor[0] as f32, cursor[1] as f32],
            time,
            frame,
            delta_time,
            _padding: 0.0,
        }
    }
}

pub struct UniformBuffer {
    pub buffer: wgpu::Buffer,
}

impl UniformBuffer {
    pub fn new(device: &wgpu::Device) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Uniform Buffer"),
            size: std::mem::size_of::<Uniforms>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self { buffer }
    }

    pub fn update(&self, queue: &wgpu::Queue, uniforms: &Uniforms) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[*uniforms]));
    }
}
