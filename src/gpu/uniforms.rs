use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Uniforms {
    pub resolution: [f32; 2],
    pub time: f32,
    pub _padding: f32, // Ensure 16-byte alignment
}

impl Uniforms {
    pub fn new(width: u32, height: u32, time: f32) -> Self {
        Self {
            resolution: [width as f32, height as f32],
            time,
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
