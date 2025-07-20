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
use crate::threading::{
    DualPerformanceTrackerHandle, ErrorReceiver, ErrorSender, SharedFrameBufferHandle,
    SharedUniformsHandle, ThreadError,
};

// AIDEV-NOTE: Terminal renderer runs in dedicated thread for display and input
pub struct TerminalRenderer {
    width: u32,
    height: u32,
    error_state: Option<String>,
    displayed_error: Option<String>,
}

impl TerminalRenderer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            error_state: None,
            displayed_error: None,
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

    // AIDEV-NOTE: Build complete screen directly from GPU data for maximum performance
    fn build_full_screen_from_gpu_data(
        &self,
        frame_data: &crate::threading::FrameData,
        performance_tracker: &Option<DualPerformanceTrackerHandle>,
        frame_buffer: &SharedFrameBufferHandle,
    ) -> String {
        let mut screen_content = String::new();
        let gpu_data = &frame_data.gpu_data;
        let gpu_width = frame_data.width;

        // Handle performance overlay if enabled - reserve first row
        if let Some(perf_text) = Self::format_performance_overlay(performance_tracker, frame_buffer)
        {
            // Create performance overlay on first row
            let clear_line = " ".repeat(self.width as usize - perf_text.len());
            screen_content.push_str(&perf_text);
            screen_content.push_str(&clear_line);
        }

        // Determine starting row for GPU data (skip row 0 if performance monitoring enabled)
        let start_row = if performance_tracker.is_some() { 1 } else { 0 };

        // Build each terminal row from GPU data
        for term_y in start_row..self.height as usize {
            for term_x in 0..self.width as usize {
                // Calculate GPU pixel rows for top and bottom halves of this terminal cell
                let top_pixel_y = term_y * 2;
                let bottom_pixel_y = term_y * 2 + 1;

                // Get top half color
                let top_idx = (top_pixel_y * gpu_width as usize + term_x) * 4;
                let (top_r, top_g, top_b) = if top_idx + 2 < gpu_data.len() {
                    (
                        gpu_data[top_idx],
                        gpu_data[top_idx + 1],
                        gpu_data[top_idx + 2],
                    )
                } else {
                    (0.0, 0.0, 0.0)
                };

                // Get bottom half color
                let bottom_idx = (bottom_pixel_y * gpu_width as usize + term_x) * 4;
                let (bottom_r, bottom_g, bottom_b) = if bottom_idx + 2 < gpu_data.len() {
                    (
                        gpu_data[bottom_idx],
                        gpu_data[bottom_idx + 1],
                        gpu_data[bottom_idx + 2],
                    )
                } else {
                    (0.0, 0.0, 0.0)
                };

                // Convert to 0-255 range
                let (top_r, top_g, top_b) = self.float_rgb_to_u8(top_r, top_g, top_b);
                let (bottom_r, bottom_g, bottom_b) =
                    self.float_rgb_to_u8(bottom_r, bottom_g, bottom_b);

                // Create styled character: ▀ with top color as foreground, bottom as background
                // Optimize: use push_str with pre-built components instead of format!
                screen_content.push_str("\x1b[38;2;");
                screen_content.push_str(&top_r.to_string());
                screen_content.push(';');
                screen_content.push_str(&top_g.to_string());
                screen_content.push(';');
                screen_content.push_str(&top_b.to_string());
                screen_content.push_str("m\x1b[48;2;");
                screen_content.push_str(&bottom_r.to_string());
                screen_content.push(';');
                screen_content.push_str(&bottom_g.to_string());
                screen_content.push(';');
                screen_content.push_str(&bottom_b.to_string());
                screen_content.push_str("m▀\x1b[0m");
            }
        }

        screen_content
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
        max_fps: Option<u32>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Set up file watcher
        let mut file_watcher = FileWatcher::new(shader_file)?;

        // Enter alternate screen and setup terminal
        execute!(stdout(), EnterAlternateScreen, Hide)?;
        crossterm_terminal::enable_raw_mode()?;
        execute!(stdout(), Clear(ClearType::All))?;

        let mut stdout = stdout();
        let start_time = Instant::now();

        // Calculate frame time for FPS limiting
        let frame_time = max_fps.map(|fps| Duration::from_millis(1000 / fps as u64));
        let mut last_frame_time = Instant::now();

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
                }
            }

            // Check for input events (non-blocking)
            if event::poll(Duration::from_millis(1))? {
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

            // Update from latest GPU frame and render full screen
            if let Some(frame_data) = {
                let mut buffer = frame_buffer.lock().unwrap();
                buffer.read_frame()
            } {
                // Build complete screen content directly from GPU data
                let screen_content = self.build_full_screen_from_gpu_data(
                    &frame_data,
                    &performance_tracker,
                    &frame_buffer,
                );

                // Single write operation for the entire screen
                execute!(stdout, MoveTo(0, 0))?;
                stdout.write_all(screen_content.as_bytes())?;
                stdout.flush()?;

                // Record terminal frame for performance tracking
                if let Some(ref tracker) = performance_tracker {
                    let mut perf = tracker.lock().unwrap();
                    perf.record_terminal_frame();
                }
            }

            // Apply FPS limiting if max_fps is specified
            if let Some(target_frame_time) = frame_time {
                let elapsed = last_frame_time.elapsed();
                if elapsed < target_frame_time {
                    std::thread::sleep(target_frame_time - elapsed);
                }
                last_frame_time = Instant::now();
            }
        }

        // Cleanup
        execute!(stdout, Show, LeaveAlternateScreen)?;
        crossterm_terminal::disable_raw_mode()?;

        Ok(())
    }
}
