use crate::gpu::{ComputePipeline, GpuBuffers, GpuDevice, UniformBuffer, Uniforms};
use crate::terminal::{update_buffer_from_gpu_data, DoubleBuffer};

// AIDEV-NOTE: Main application struct that manages GPU and terminal state
pub struct App {
    gpu_device: GpuDevice,
    gpu_buffers: GpuBuffers,
    uniform_buffer: UniformBuffer,
    compute_pipeline: ComputePipeline,
    terminal_buffer: DoubleBuffer,
    width: u32,
    height: u32,
}

impl App {
    pub fn new(
        width: u32,
        height: u32,
        shader_source: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Initialize GPU - double the height for half-cell rendering
        let gpu_device = GpuDevice::new_blocking()?;
        let gpu_buffers = GpuBuffers::new(&gpu_device.device, width, height * 2);
        let uniform_buffer = UniformBuffer::new(&gpu_device.device);
        let compute_pipeline = ComputePipeline::new(
            &gpu_device.device,
            &gpu_buffers,
            &uniform_buffer,
            shader_source,
        )?;

        // Initialize terminal buffer
        let terminal_buffer = DoubleBuffer::new(width as usize, height as usize);

        Ok(Self {
            gpu_device,
            gpu_buffers,
            uniform_buffer,
            compute_pipeline,
            terminal_buffer,
            width,
            height,
        })
    }

    // AIDEV-NOTE: Reload shader with new source, handling validation and compilation
    pub fn reload_shader(&mut self, shader_source: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Validate shader first
        validate_shader(shader_source)?;

        // Create new compute pipeline
        let new_pipeline = ComputePipeline::new(
            &self.gpu_device.device,
            &self.gpu_buffers,
            &self.uniform_buffer,
            shader_source,
        )?;

        // Replace the old pipeline
        self.compute_pipeline = new_pipeline;
        Ok(())
    }

    pub fn render_frame(
        &mut self,
        time: f32,
    ) -> Result<Vec<(usize, usize, String)>, Box<dyn std::error::Error>> {
        // Update uniforms - use doubled height for GPU resolution
        let uniforms = Uniforms::new(self.width, self.height * 2, time);
        self.uniform_buffer
            .update(&self.gpu_device.queue, &uniforms);

        // Create command encoder
        let mut encoder =
            self.gpu_device
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        // Dispatch the compute shader - use doubled height
        self.compute_pipeline
            .dispatch(&mut encoder, self.width, self.height * 2);

        // Copy output to readback buffer
        self.gpu_buffers.copy_to_readback(&mut encoder);

        // Submit commands
        self.gpu_device.queue.submit(Some(encoder.finish()));

        // Read back the GPU data
        let gpu_data = self
            .gpu_buffers
            .read_data_blocking(&self.gpu_device.device)?;

        // Update terminal buffer with GPU data - pass doubled height
        update_buffer_from_gpu_data(
            &mut self.terminal_buffer,
            &gpu_data,
            self.width,
            self.height * 2,
        );

        // Get changes for rendering
        Ok(self.terminal_buffer.swap_and_get_changes())
    }
}

// AIDEV-NOTE: Validate shader compilation using naga without GPU device
pub fn validate_shader(shader_source: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Parse WGSL using naga frontend
    let module = naga::front::wgsl::parse_str(shader_source)?;

    // Validate the parsed module
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    validator.validate(&module)?;

    Ok(())
}
