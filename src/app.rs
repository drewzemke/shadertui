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
    cursor: [i32; 2],
    frame_count: u32,
    last_frame_time: std::time::Instant,
    time_paused: bool,
    paused_time: f32,
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
            cursor: [0, 0],
            frame_count: 0,
            last_frame_time: std::time::Instant::now(),
            time_paused: false,
            paused_time: 0.0,
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

    // AIDEV-NOTE: Move cursor position with arrow keys
    pub fn move_cursor(&mut self, dx: i32, dy: i32) {
        self.cursor[0] += dx;
        self.cursor[1] += dy;
    }

    // AIDEV-NOTE: Toggle time pause state
    pub fn toggle_pause(&mut self, current_time: f32) {
        if self.time_paused {
            // Resume: reset start time to account for paused duration
            self.time_paused = false;
        } else {
            // Pause: store current time
            self.time_paused = true;
            self.paused_time = current_time;
        }
    }

    pub fn render_frame(
        &mut self,
        start_time: std::time::Instant,
    ) -> Result<Vec<(usize, usize, String)>, Box<dyn std::error::Error>> {
        // Calculate frame time and delta
        let current_time = std::time::Instant::now();
        let delta_time = current_time
            .duration_since(self.last_frame_time)
            .as_secs_f32();
        self.last_frame_time = current_time;

        // Calculate effective time (accounting for pause)
        let effective_time = if self.time_paused {
            self.paused_time
        } else {
            start_time.elapsed().as_secs_f32()
        };

        // Increment frame count
        self.frame_count += 1;

        // Update uniforms - use doubled height for GPU resolution
        let uniforms = Uniforms::new(
            self.width,
            self.height * 2,
            effective_time,
            self.cursor,
            self.frame_count,
            delta_time,
        );
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
