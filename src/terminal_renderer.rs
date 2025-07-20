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
    ErrorReceiver, ErrorSender, SharedFrameBufferHandle, SharedUniformsHandle, ThreadError,
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
    fn update_from_frame_buffer(&mut self, frame_buffer: &SharedFrameBufferHandle) -> bool {
        let mut buffer = frame_buffer.lock().unwrap();
        if let Some(frame_data) = buffer.read_frame() {
            // Update terminal buffer with GPU data
            update_buffer_from_gpu_data(
                &mut self.terminal_buffer,
                &frame_data.gpu_data,
                frame_data.width,
                frame_data.height,
            );
            true
        } else {
            false
        }
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

    // AIDEV-NOTE: Main terminal thread function - handles input, file watching, and display
    pub fn run_terminal_thread(
        mut self,
        frame_buffer: SharedFrameBufferHandle,
        shared_uniforms: SharedUniformsHandle,
        error_sender: ErrorSender,
        error_receiver: ErrorReceiver,
        shader_file: &Path,
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
            if self.update_from_frame_buffer(&frame_buffer) {
                // Get changes for rendering
                let changes = self.terminal_buffer.swap_and_get_changes();

                // Apply only the changed cells
                for (x, y, content) in changes {
                    execute!(stdout, MoveTo(x as u16, y as u16))?;
                    stdout.write_all(content.as_bytes())?;
                }

                stdout.flush()?;
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
