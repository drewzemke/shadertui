use std::io::{stdout, Write};
use std::time::{Duration, Instant};

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode},
    execute,
    terminal::{
        self as crossterm_terminal, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen,
    },
};

mod gpu;
mod terminal;

use gpu::{ComputePipeline, GpuBuffers, GpuDevice, UniformBuffer, Uniforms};
use terminal::{update_buffer_from_gpu_data, DoubleBuffer};

// AIDEV-NOTE: Main application struct that manages GPU and terminal state
struct App {
    gpu_device: GpuDevice,
    gpu_buffers: GpuBuffers,
    uniform_buffer: UniformBuffer,
    compute_pipeline: ComputePipeline,
    terminal_buffer: DoubleBuffer,
    width: u32,
    height: u32,
}

impl App {
    fn new(width: u32, height: u32) -> Result<Self, Box<dyn std::error::Error>> {
        // Initialize GPU - double the height for half-cell rendering
        let gpu_device = GpuDevice::new_blocking()?;
        let gpu_buffers = GpuBuffers::new(&gpu_device.device, width, height * 2);
        let uniform_buffer = UniformBuffer::new(&gpu_device.device);
        let compute_pipeline =
            ComputePipeline::new(&gpu_device.device, &gpu_buffers, &uniform_buffer);

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

    fn render_frame(
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Enter alternate screen and hide cursor
    execute!(stdout(), EnterAlternateScreen, Hide)?;

    // Enable raw mode for better control
    crossterm_terminal::enable_raw_mode()?;

    // Clear screen once
    execute!(stdout(), Clear(ClearType::All))?;

    // Get terminal size
    let (width, height) = crossterm_terminal::size()?;

    // Initialize the application
    let mut app = match App::new(width as u32, height as u32) {
        Ok(app) => app,
        Err(e) => {
            // Cleanup on error
            execute!(stdout(), Show, LeaveAlternateScreen)?;
            crossterm_terminal::disable_raw_mode()?;
            return Err(e);
        }
    };

    let mut stdout = stdout();
    let start_time = Instant::now();

    // Animation loop
    loop {
        // Check for exit events (non-blocking)
        if event::poll(Duration::from_millis(20))? {
            if let Event::Key(key_event) = event::read()? {
                match key_event.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => break,
                    KeyCode::Char('c')
                        if key_event.modifiers.contains(event::KeyModifiers::CONTROL) =>
                    {
                        break
                    }
                    _ => {}
                }
            }
        }

        // Get current time (seconds since start)
        let time = start_time.elapsed().as_secs_f32();

        // Render frame and get changes
        let changes = match app.render_frame(time) {
            Ok(changes) => changes,
            Err(e) => {
                // Print error and continue
                eprintln!("Render error: {}", e);
                continue;
            }
        };

        // Apply only the changed cells
        for (x, y, content) in changes {
            execute!(stdout, MoveTo(x as u16, y as u16))?;
            stdout.write_all(content.as_bytes())?;
        }

        stdout.flush()?;

        // Wait 20ms before next frame
        std::thread::sleep(Duration::from_millis(20));
    }

    // Cleanup
    execute!(stdout, Show, LeaveAlternateScreen)?;
    crossterm_terminal::disable_raw_mode()?;

    Ok(())
}
