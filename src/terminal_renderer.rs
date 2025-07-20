use std::fs;
use std::io::{stdout, Write};
use std::path::Path;
use std::time::{Duration, Instant};

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode},
    execute,
    terminal::{
        self as crossterm_terminal, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen,
    },
};

use crate::file_watcher::FileWatcher;
use crate::terminal::{update_buffer_from_gpu_data, DoubleBuffer};
use crate::threading::{
    DualPerformanceTrackerHandle, ErrorReceiver, ErrorSender, SharedFrameBufferHandle,
    SharedUniformsHandle, ThreadError,
};

// AIDEV-NOTE: Terminal renderer runs in dedicated thread for display and input
pub struct TerminalRenderer {
    terminal_buffer: DoubleBuffer,
    width: u32,
    height: u32,
    error_state: Option<String>,
    displayed_error: Option<String>,
}

impl TerminalRenderer {
    pub fn new(width: u32, height: u32) -> Self {
        let terminal_buffer = DoubleBuffer::new(width as usize, height as usize);

        Self {
            terminal_buffer,
            width,
            height,
            error_state: None,
            displayed_error: None,
        }
    }

    // AIDEV-NOTE: Process latest frame from GPU thread
    fn update_from_frame_buffer(
        &mut self,
        frame_buffer: &SharedFrameBufferHandle,
        perf_enabled: bool,
    ) -> bool {
        let mut buffer = frame_buffer.lock().unwrap();
        if let Some(frame_data) = buffer.read_frame() {
            // Update terminal buffer with GPU data
            if perf_enabled {
                // Skip the top row when performance monitoring is enabled
                self.update_buffer_from_gpu_data_skip_top_row(
                    &frame_data.gpu_data,
                    frame_data.width,
                    frame_data.height,
                );
            } else {
                update_buffer_from_gpu_data(
                    &mut self.terminal_buffer,
                    &frame_data.gpu_data,
                    frame_data.width,
                    frame_data.height,
                );
            }
            true
        } else {
            false
        }
    }

    // AIDEV-NOTE: Update buffer from GPU data but skip row 0 to avoid performance overlay conflict
    fn update_buffer_from_gpu_data_skip_top_row(
        &mut self,
        gpu_data: &[f32],
        gpu_width: u32,
        _gpu_height: u32,
    ) {
        self.terminal_buffer.clear_next();

        // Each terminal cell represents 2 vertical pixels (top and bottom half)
        // Skip y=0 (top row) to preserve performance overlay space
        for y in 1..self.terminal_buffer.height {
            for x in 0..self.terminal_buffer.width {
                // Calculate GPU pixel rows for top and bottom halves of this terminal cell
                let top_pixel_y = y * 2;
                let bottom_pixel_y = y * 2 + 1;

                // Use gpu_width for proper indexing (same logic as original function)
                let top_idx = (top_pixel_y * gpu_width as usize + x) * 4;
                let (top_r, top_g, top_b) = if top_idx + 2 < gpu_data.len() {
                    (
                        gpu_data[top_idx],
                        gpu_data[top_idx + 1],
                        gpu_data[top_idx + 2],
                    )
                } else {
                    (0.0, 0.0, 0.0)
                };

                let bottom_idx = (bottom_pixel_y * gpu_width as usize + x) * 4;
                let (bottom_r, bottom_g, bottom_b) = if bottom_idx + 2 < gpu_data.len() {
                    (
                        gpu_data[bottom_idx],
                        gpu_data[bottom_idx + 1],
                        gpu_data[bottom_idx + 2],
                    )
                } else {
                    (0.0, 0.0, 0.0)
                };

                // Convert to 0-255 range for RGB colors
                let (top_r, top_g, top_b) = self.float_rgb_to_u8(top_r, top_g, top_b);
                let (bottom_r, bottom_g, bottom_b) =
                    self.float_rgb_to_u8(bottom_r, bottom_g, bottom_b);

                // Use ▀ character: foreground = top half, background = bottom half
                let content = format!(
                    "\x1b[38;2;{top_r};{top_g};{top_b}m\x1b[48;2;{bottom_r};{bottom_g};{bottom_b}m▀\x1b[0m"
                );

                self.terminal_buffer.set_cell(x, y, content);
            }
        }
    }

    // AIDEV-NOTE: Helper function for RGB conversion
    fn float_rgb_to_u8(&self, r: f32, g: f32, b: f32) -> (u8, u8, u8) {
        let r = (r * 255.0) as u8;
        let g = (g * 255.0) as u8;
        let b = (b * 255.0) as u8;
        (r, g, b)
    }

    // AIDEV-NOTE: Handle file change and request shader reload
    fn handle_file_change(
        shader_file: &Path,
        shared_uniforms: &SharedUniformsHandle,
    ) -> Option<String> {
        match fs::read_to_string(shader_file) {
            Ok(new_shader_source) => {
                // Request shader reload via shared uniforms
                {
                    let mut uniforms = shared_uniforms.lock().unwrap();
                    uniforms.request_shader_reload(new_shader_source);
                }
                None // No error, reload requested
            }
            Err(e) => Some(format!("File read error: {e}")),
        }
    }

    // AIDEV-NOTE: Format performance overlay string for top row display
    fn format_performance_overlay(
        performance_tracker: &Option<DualPerformanceTrackerHandle>,
        frame_buffer: &SharedFrameBufferHandle,
    ) -> Option<String> {
        if let Some(ref tracker) = performance_tracker {
            let (gpu_fps, term_fps, frames_dropped) = {
                let perf = tracker.lock().unwrap();
                let frame_buf = frame_buffer.lock().unwrap();
                (
                    perf.get_gpu_fps(),
                    perf.get_terminal_fps(),
                    frame_buf.get_frames_dropped(),
                )
            };
            Some(format!(
                "GPU: {gpu_fps:.1} | Term: {term_fps:.1} | Dropped: {frames_dropped}"
            ))
        } else {
            None
        }
    }

    // AIDEV-NOTE: Main terminal thread function - handles input, file watching, and display
    pub fn run_terminal_thread(
        mut self,
        frame_buffer: SharedFrameBufferHandle,
        shared_uniforms: SharedUniformsHandle,
        error_sender: ErrorSender,
        error_receiver: ErrorReceiver,
        shader_file: &Path,
        performance_tracker: Option<DualPerformanceTrackerHandle>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Set up file watcher
        let mut file_watcher = FileWatcher::new(shader_file)?;

        // Enter alternate screen and setup terminal
        execute!(stdout(), EnterAlternateScreen, Hide)?;
        crossterm_terminal::enable_raw_mode()?;
        execute!(stdout(), Clear(ClearType::All))?;

        let mut stdout = stdout();
        let start_time = Instant::now();

        // Terminal rendering loop
        loop {
            // Check for file changes
            if file_watcher.check_for_changes() {
                if let Some(error_msg) = Self::handle_file_change(shader_file, &shared_uniforms) {
                    self.error_state = Some(error_msg);
                } else {
                    // Clear error state on successful reload request
                    self.error_state = None;
                }
            }

            // Check for thread errors (non-blocking)
            if let Ok(thread_error) = error_receiver.try_recv() {
                match thread_error {
                    ThreadError::ShaderCompilationError(msg) => {
                        self.error_state = Some(format!("Shader compilation error: {msg}"));
                    }
                    ThreadError::ShaderReloadSuccess => {
                        // Clear error state on successful shader reload
                        self.error_state = None;
                    }
                    ThreadError::GpuError(msg) => {
                        self.error_state = Some(format!("GPU error: {msg}"));
                    }
                    ThreadError::Shutdown => {
                        break;
                    }
                    ThreadError::TerminalError(_) => {
                        // This shouldn't happen since we're the terminal thread
                    }
                }
            }

            // Check for input events (non-blocking)
            if event::poll(Duration::from_millis(16))? {
                // ~60 FPS input polling
                if let Event::Key(key_event) = event::read()? {
                    match key_event.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => {
                            let _ = error_sender.send(ThreadError::Shutdown);
                            break;
                        }
                        KeyCode::Char('c')
                            if key_event.modifiers.contains(event::KeyModifiers::CONTROL) =>
                        {
                            let _ = error_sender.send(ThreadError::Shutdown);
                            break;
                        }
                        KeyCode::Up => {
                            let mut uniforms = shared_uniforms.lock().unwrap();
                            uniforms.move_cursor(0, -1);
                        }
                        KeyCode::Down => {
                            let mut uniforms = shared_uniforms.lock().unwrap();
                            uniforms.move_cursor(0, 1);
                        }
                        KeyCode::Left => {
                            let mut uniforms = shared_uniforms.lock().unwrap();
                            uniforms.move_cursor(-1, 0);
                        }
                        KeyCode::Right => {
                            let mut uniforms = shared_uniforms.lock().unwrap();
                            uniforms.move_cursor(1, 0);
                        }
                        KeyCode::Char(' ') => {
                            let current_time = start_time.elapsed().as_secs_f32();
                            let mut uniforms = shared_uniforms.lock().unwrap();
                            uniforms.toggle_pause(current_time);
                        }
                        _ => {}
                    }
                }
            }

            // Check for thread errors
            // This is handled by the main thread coordination

            // If we're in an error state, display error only if it changed
            if let Some(ref error_msg) = self.error_state {
                // Only redraw if this is a new error or we haven't displayed it yet
                if self.displayed_error.as_ref() != Some(error_msg) {
                    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;
                    stdout.write_all(format!("{error_msg}\nPress 'q' to quit").as_bytes())?;
                    stdout.flush()?;
                    self.displayed_error = Some(error_msg.clone());
                }
                std::thread::sleep(Duration::from_millis(16));
                continue;
            } else {
                // Clear displayed error when we exit error state
                self.displayed_error = None;
            }

            // Update from latest GPU frame
            if self.update_from_frame_buffer(&frame_buffer, performance_tracker.is_some()) {
                // Get changes for rendering
                let changes = self.terminal_buffer.swap_and_get_changes();

                // Apply only the changed cells
                for (x, y, content) in changes {
                    execute!(stdout, MoveTo(x as u16, y as u16))?;
                    stdout.write_all(content.as_bytes())?;
                }

                // Draw performance overlay on top row if enabled - after all other changes
                if let Some(perf_text) =
                    Self::format_performance_overlay(&performance_tracker, &frame_buffer)
                {
                    execute!(stdout, MoveTo(0, 0))?;
                    // Clear the entire top row with black background first
                    let clear_line =
                        format!("\x1b[48;2;0;0;0m{}\x1b[0m", " ".repeat(self.width as usize));
                    stdout.write_all(clear_line.as_bytes())?;
                    execute!(stdout, MoveTo(0, 0))?;
                    // Use white text on black background to make it stand out
                    let styled_perf =
                        format!("\x1b[38;2;255;255;255m\x1b[48;2;0;0;0m{perf_text}\x1b[0m");
                    stdout.write_all(styled_perf.as_bytes())?;
                }

                stdout.flush()?;

                // Record terminal frame for performance tracking
                if let Some(ref tracker) = performance_tracker {
                    let mut perf = tracker.lock().unwrap();
                    perf.record_terminal_frame();
                }
            }

            // Target ~60 FPS for terminal updates
            std::thread::sleep(Duration::from_millis(16));
        }

        // Cleanup
        execute!(stdout, Show, LeaveAlternateScreen)?;
        crossterm_terminal::disable_raw_mode()?;

        Ok(())
    }
}
