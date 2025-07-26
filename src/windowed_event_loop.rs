use std::path::PathBuf;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use crate::renderers::WindowRenderer;
use crate::utils::multi_file_watcher::MultiFileWatcher;
use crate::utils::shader_import::{process_imports, DependencyInfo};
use crate::utils::{get_centered_window_position, get_window_size, Cli};

// AIDEV-NOTE: WindowedApp handles the winit application lifecycle for basic window display
struct WindowedApp {
    window: Option<Arc<Window>>,
    renderer: Option<WindowRenderer>,
    cli: Cli,
    shader_source: String,
    cursor_position: [f32; 2],

    // Hot reload system
    file_watcher: Option<MultiFileWatcher>,
    shader_file_path: PathBuf,
    dependency_info: Option<DependencyInfo>,
    error_state: Option<String>,
}

impl WindowedApp {
    fn new(cli: Cli, shader_source: String) -> Self {
        let (width, height) = get_window_size();
        let shader_file_path = cli.shader_file.clone();

        // Initialize file watcher for hot reload
        let file_watcher = match MultiFileWatcher::new(&shader_file_path) {
            Ok(watcher) => Some(watcher),
            Err(e) => {
                eprintln!("Warning: Could not initialize file watcher: {e}");
                None
            }
        };

        Self {
            window: None,
            renderer: None,
            cli,
            shader_source,
            cursor_position: [width as f32 / 2.0, height as f32 / 2.0],
            file_watcher,
            shader_file_path,
            dependency_info: None,
            error_state: None,
        }
    }

    // AIDEV-NOTE: Update window title with performance metrics if enabled
    fn update_window_title(&self) {
        if let (Some(window), Some(renderer)) = (&self.window, &self.renderer) {
            let title = if let Some(error) = &self.error_state {
                format!("ShaderTUI | Error: {error}")
            } else if self.cli.perf {
                if let Some(fps) = renderer.get_fps() {
                    format!("ShaderTUI | FPS: {fps:.1}")
                } else {
                    "ShaderTUI | FPS: --".to_string()
                }
            } else {
                "ShaderTUI".to_string()
            };
            window.set_title(&title);
        }
    }

    // AIDEV-NOTE: Handle file changes and attempt shader reload
    fn handle_file_change(&mut self) -> bool {
        if let Some(file_watcher) = &mut self.file_watcher {
            if let Some(_changed_file) = file_watcher.check_for_changes() {
                match std::fs::read_to_string(&self.shader_file_path) {
                    Ok(raw_shader_source) => {
                        match process_imports(&self.shader_file_path, &raw_shader_source) {
                            Ok((processed_shader_source, deps)) => {
                                // Update dependency tracking
                                if let Err(e) = file_watcher.update_watched_files(&deps.all_files) {
                                    eprintln!("Warning: Could not update watched files: {e}");
                                }
                                self.dependency_info = Some(deps);

                                // Attempt shader reload
                                if let Some(renderer) = &mut self.renderer {
                                    match renderer.reload_shader(&processed_shader_source) {
                                        Ok(()) => {
                                            self.error_state = None;
                                            println!("Shader reloaded successfully");
                                            return true;
                                        }
                                        Err(e) => {
                                            let error_msg = format!("Compilation error: {e}");
                                            self.error_state = Some(error_msg.clone());
                                            eprintln!("{error_msg}");
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                let error_msg = format!("Import error: {e}");
                                self.error_state = Some(error_msg.clone());
                                eprintln!("{error_msg}");
                            }
                        }
                    }
                    Err(e) => {
                        let error_msg = format!("File read error: {e}");
                        self.error_state = Some(error_msg.clone());
                        eprintln!("{error_msg}");
                    }
                }
            }
        }
        false
    }
}

impl ApplicationHandler for WindowedApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let (width, height) = get_window_size();
        let position = get_centered_window_position(event_loop);

        let window_attributes = Window::default_attributes()
            .with_title("ShaderTUI")
            .with_inner_size(PhysicalSize::new(width, height))
            .with_position(position)
            .with_resizable(true);

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        // Create wgpu instance and surface
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window.clone()).unwrap();
        let window_size = window.inner_size();

        // Create renderer with the surface and shader
        match WindowRenderer::new(
            instance,
            surface,
            (window_size.width, window_size.height),
            &self.shader_source,
            self.cli.perf,
        ) {
            Ok(mut renderer) => {
                println!("Successfully initialized WindowRenderer");

                // Set initial cursor position
                renderer.update_cursor_position(self.cursor_position[0], self.cursor_position[1]);

                self.renderer = Some(renderer);
                self.window = Some(window);

                // Initialize dependency tracking for the initial shader
                match std::fs::read_to_string(&self.shader_file_path) {
                    Ok(raw_shader_source) => {
                        match process_imports(&self.shader_file_path, &raw_shader_source) {
                            Ok((_processed_shader_source, deps)) => {
                                if let Some(file_watcher) = &mut self.file_watcher {
                                    if let Err(e) =
                                        file_watcher.update_watched_files(&deps.all_files)
                                    {
                                        eprintln!(
                                            "Warning: Could not initialize watched files: {e}"
                                        );
                                    }
                                }
                                self.dependency_info = Some(deps);
                            }
                            Err(e) => {
                                eprintln!("Warning: Could not process initial imports: {e}");
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Warning: Could not read initial shader file: {e}");
                    }
                }

                // Request initial redraw
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            Err(e) => {
                let error_msg = format!("Failed to create WindowRenderer: {e}");
                eprintln!("{error_msg}");
                self.error_state = Some(error_msg);

                // Still set up the window but without renderer
                self.window = Some(window);

                // Try to display error in window title
                self.update_window_title();

                // Exit after a short delay to allow error display
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                println!("Window close requested, exiting...");
                event_loop.exit();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key_code),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                match key_code {
                    KeyCode::KeyQ => {
                        println!("Q pressed, exiting...");
                        event_loop.exit();
                    }
                    KeyCode::Escape => {
                        println!("Escape pressed, exiting...");
                        event_loop.exit();
                    }
                    KeyCode::Space => {
                        if let Some(renderer) = &mut self.renderer {
                            renderer.toggle_pause();
                        }
                    }
                    KeyCode::ArrowUp => {
                        // Arrow up should move cursor up in window coords (decrease Y)
                        self.cursor_position[1] = (self.cursor_position[1] - 10.0).max(0.0);
                        if let Some(renderer) = &mut self.renderer {
                            renderer.update_cursor_position(
                                self.cursor_position[0],
                                self.cursor_position[1],
                            );
                        }
                    }
                    KeyCode::ArrowDown => {
                        // Arrow down should move cursor down in window coords (increase Y)
                        if let Some(window) = &self.window {
                            let size = window.inner_size();
                            self.cursor_position[1] =
                                (self.cursor_position[1] + 10.0).min(size.height as f32 - 1.0);
                        }
                        if let Some(renderer) = &mut self.renderer {
                            renderer.update_cursor_position(
                                self.cursor_position[0],
                                self.cursor_position[1],
                            );
                        }
                    }
                    KeyCode::ArrowLeft => {
                        self.cursor_position[0] = (self.cursor_position[0] - 10.0).max(0.0);
                        if let Some(renderer) = &mut self.renderer {
                            renderer.update_cursor_position(
                                self.cursor_position[0],
                                self.cursor_position[1],
                            );
                        }
                    }
                    KeyCode::ArrowRight => {
                        if let Some(window) = &self.window {
                            let size = window.inner_size();
                            self.cursor_position[0] =
                                (self.cursor_position[0] + 10.0).min(size.width as f32 - 1.0);
                        }
                        if let Some(renderer) = &mut self.renderer {
                            renderer.update_cursor_position(
                                self.cursor_position[0],
                                self.cursor_position[1],
                            );
                        }
                    }
                    _ => {}
                }

                // Request redraw after input to see immediate changes
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                // Mouse position as alternative cursor control
                self.cursor_position = [position.x as f32, position.y as f32];
                if let Some(renderer) = &mut self.renderer {
                    renderer
                        .update_cursor_position(self.cursor_position[0], self.cursor_position[1]);
                }

                // Request redraw for mouse movement
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    match renderer.resize(size.width, size.height) {
                        Ok(()) => {
                            // Clear any previous resize errors on successful resize
                            if self
                                .error_state
                                .as_ref()
                                .is_some_and(|e| e.contains("Resize error"))
                            {
                                self.error_state = None;
                            }

                            // Update cursor bounds for new window size
                            self.cursor_position[0] =
                                self.cursor_position[0].min(size.width as f32);
                            self.cursor_position[1] =
                                self.cursor_position[1].min(size.height as f32);
                            renderer.update_cursor_position(
                                self.cursor_position[0],
                                self.cursor_position[1],
                            );

                            self.update_window_title();
                        }
                        Err(e) => {
                            let error_msg = format!("Resize error: {e}");
                            eprintln!("{error_msg}");
                            self.error_state = Some(error_msg);
                            self.update_window_title();
                        }
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                // Render the shader to the window surface
                if let Some(renderer) = &mut self.renderer {
                    match renderer.render() {
                        Ok(()) => {
                            // Clear any previous render errors on successful render
                            if self
                                .error_state
                                .as_ref()
                                .is_some_and(|e| e.contains("Render error"))
                            {
                                self.error_state = None;
                            }
                            // Update window title with performance metrics after successful render
                            self.update_window_title();
                        }
                        Err(e) => {
                            let error_msg = format!("Render error: {e}");
                            eprintln!("{error_msg}");

                            // Check for specific surface/GPU errors that might require special handling
                            let error_str = e.to_string();
                            if error_str.contains("Surface")
                                || error_str.contains("Lost")
                                || error_str.contains("Outdated")
                            {
                                // Surface-related error - might need to recreate surface
                                self.error_state =
                                    Some("Surface error - try resizing window".to_string());
                            } else {
                                self.error_state = Some(error_msg);
                            }

                            self.update_window_title();
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Check for file changes and hot reload
        if self.handle_file_change() {
            // Update window title to reflect any error state changes
            self.update_window_title();

            // Request redraw after successful shader reload
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }

        // Continuously request redraws for animation
        if let Some(window) = &self.window {
            window.request_redraw();
        }

        // Use Poll mode for continuous animation updates
        event_loop.set_control_flow(ControlFlow::Poll);
    }
}

pub fn run_windowed_event_loop(
    cli: Cli,
    shader_source: String,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting ShaderTUI in windowed mode...");
    println!("Window will display at 1280x800 pixels, centered on screen");
    println!("Controls:");
    println!("  Arrow keys: Move cursor position");
    println!("  Spacebar: Pause/resume animation");
    println!("  Q or Escape: Exit");
    println!("  Mouse: Move cursor (alternative to arrow keys)");

    let event_loop = EventLoop::new()?;
    let mut app = WindowedApp::new(cli, shader_source);

    event_loop.run_app(&mut app)?;
    Ok(())
}
