use std::time::Instant;

use crate::gpu::{ComputePipeline, GpuBuffers, GpuDevice, UniformBuffer, Uniforms};
use crate::threading::{
    DualPerformanceTrackerHandle, ErrorSender, FrameData, SharedFrameBufferHandle,
    SharedUniformsHandle, ThreadError,
};

// AIDEV-NOTE: GPU renderer runs in dedicated thread for continuous compute
pub struct GpuRenderer {
    gpu_device: GpuDevice,
    gpu_buffers: GpuBuffers,
    uniform_buffer: UniformBuffer,
    compute_pipeline: ComputePipeline,
    width: u32,
    height: u32,
    frame_count: u32,
    start_time: Instant,
    last_frame_time: Instant,
}

impl GpuRenderer {
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

        let now = Instant::now();

        Ok(Self {
            gpu_device,
            gpu_buffers,
            uniform_buffer,
            compute_pipeline,
            width,
            height,
            frame_count: 0,
            start_time: now,
            last_frame_time: now,
        })
    }

    // AIDEV-NOTE: Reload shader with new source, called from compute thread
    pub fn reload_shader(&mut self, shader_source: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Validate shader first using existing validation function
        crate::validation::validate_shader(shader_source)?;

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

    // AIDEV-NOTE: Main GPU compute loop - runs continuously without blocking
    pub fn render_frame(
        &mut self,
        shared_uniforms: &SharedUniformsHandle,
    ) -> Result<FrameData, Box<dyn std::error::Error>> {
        // Calculate frame time and delta
        let current_time = Instant::now();
        let delta_time = current_time
            .duration_since(self.last_frame_time)
            .as_secs_f32();
        self.last_frame_time = current_time;

        // Get shared uniform data
        let (cursor, time_paused, paused_time) = {
            let uniforms = shared_uniforms.lock().unwrap();
            (uniforms.cursor, uniforms.time_paused, uniforms.paused_time)
        };

        // Calculate effective time (accounting for pause)
        let effective_time = if time_paused {
            paused_time
        } else {
            self.start_time.elapsed().as_secs_f32()
        };

        // Increment frame count
        self.frame_count += 1;

        // Update uniforms - use doubled height for GPU resolution
        let uniforms = Uniforms::new(
            self.width,
            self.height * 2,
            effective_time,
            cursor,
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

        // Create frame data
        Ok(FrameData {
            gpu_data,
            width: self.width,
        })
    }

    // AIDEV-NOTE: Main GPU thread function - continuous rendering loop
    pub fn run_compute_thread(
        mut self,
        frame_buffer: SharedFrameBufferHandle,
        shared_uniforms: SharedUniformsHandle,
        main_error_sender: ErrorSender,
        terminal_error_sender: ErrorSender,
        performance_tracker: Option<DualPerformanceTrackerHandle>,
    ) {
        loop {
            // Check for shader reload requests
            if let Some(new_shader_source) = {
                let mut uniforms = shared_uniforms.lock().unwrap();
                uniforms.consume_shader_reload()
            } {
                match self.reload_shader(&new_shader_source) {
                    Err(e) => {
                        let error_msg = ThreadError::ShaderCompilationError(e.to_string());
                        let _ = main_error_sender.send(error_msg.clone());
                        let _ = terminal_error_sender.send(error_msg);
                        continue;
                    }
                    Ok(()) => {
                        // Shader reloaded successfully - send signal to clear error state
                        let _ = terminal_error_sender.send(ThreadError::ShaderReloadSuccess);
                    }
                }
            }

            // Render frame
            match self.render_frame(&shared_uniforms) {
                Ok(frame_data) => {
                    // Write frame to shared buffer (may drop frames if terminal is slow)
                    {
                        let mut buffer = frame_buffer.lock().unwrap();
                        buffer.write_frame(frame_data);
                    }

                    // Record GPU frame for performance tracking
                    if let Some(ref tracker) = performance_tracker {
                        let mut perf = tracker.lock().unwrap();
                        perf.record_gpu_frame();
                    }
                }
                Err(e) => {
                    let error_msg = ThreadError::GpuError(e.to_string());
                    let _ = main_error_sender.send(error_msg.clone());
                    let _ = terminal_error_sender.send(error_msg);
                    // Continue running on error - don't crash the GPU thread
                    std::thread::sleep(std::time::Duration::from_millis(16)); // ~60 FPS fallback
                }
            }

            // Small yield to prevent 100% CPU usage
            std::thread::yield_now();
        }
    }
}
