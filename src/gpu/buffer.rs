use wgpu;

pub struct GpuBuffers {
    pub output_buffer: wgpu::Buffer,
    pub readback_buffer: wgpu::Buffer,
    pub size: wgpu::BufferAddress,
}

impl GpuBuffers {
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let buffer_size =
            (width * height * 4 * std::mem::size_of::<f32>() as u32) as wgpu::BufferAddress;

        let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Readback Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Self {
            output_buffer,
            readback_buffer,
            size: buffer_size,
        }
    }

    pub fn copy_to_readback(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.copy_buffer_to_buffer(&self.output_buffer, 0, &self.readback_buffer, 0, self.size);
    }

    pub async fn read_data(
        &self,
        device: &wgpu::Device,
    ) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        let buffer_slice = self.readback_buffer.slice(..);

        // Map the buffer for reading
        let (sender, receiver) = flume::unbounded();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });

        // Poll the device until the buffer is ready
        let _ = device.poll(wgpu::MaintainBase::Wait);

        // Wait for the mapping to complete
        receiver.recv_async().await??;

        // Get the mapped data
        let data = buffer_slice.get_mapped_range();
        let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();

        // Unmap the buffer
        drop(data);
        self.readback_buffer.unmap();

        Ok(result)
    }

    pub fn read_data_blocking(
        &self,
        device: &wgpu::Device,
    ) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        pollster::block_on(self.read_data(device))
    }
}
