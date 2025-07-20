use std::sync::{Arc, Mutex};
use std::thread;

use crate::cli::Cli;
use crate::gpu_renderer::GpuRenderer;
use crate::terminal_renderer::TerminalRenderer;
use crate::threading::{ErrorReceiver, SharedFrameBuffer, SharedUniforms, ThreadError};

// AIDEV-NOTE: Multi-threaded event loop with independent GPU and Terminal threads
pub fn run_threaded_event_loop(
    cli: Cli,
    shader_source: String,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get terminal size
    let (width, height) = crossterm::terminal::size()?;

    // Create shared state
    let frame_buffer = Arc::new(Mutex::new(SharedFrameBuffer::new()));
    let shared_uniforms = Arc::new(Mutex::new(SharedUniforms::new()));

    // Create error communication channels
    let (main_error_sender, main_error_receiver): (_, ErrorReceiver) = std::sync::mpsc::channel();
    let (terminal_error_sender, terminal_error_receiver): (_, ErrorReceiver) =
        std::sync::mpsc::channel();

    // Initialize GPU renderer BEFORE starting threads to catch early shader errors
    let gpu_renderer = match GpuRenderer::new(width as u32, height as u32, &shader_source) {
        Ok(renderer) => renderer,
        Err(e) => {
            eprintln!("Shader compilation error: {e}");
            std::process::exit(1);
        }
    };

    // Clone handles for threads
    let gpu_frame_buffer = Arc::clone(&frame_buffer);
    let gpu_shared_uniforms = Arc::clone(&shared_uniforms);
    let gpu_main_error_sender = main_error_sender.clone();
    let gpu_terminal_error_sender = terminal_error_sender.clone();

    let terminal_frame_buffer = Arc::clone(&frame_buffer);
    let terminal_shared_uniforms = Arc::clone(&shared_uniforms);
    let terminal_main_error_sender = main_error_sender.clone();

    // Spawn GPU compute thread
    let _gpu_thread = thread::spawn(move || {
        gpu_renderer.run_compute_thread(
            gpu_frame_buffer,
            gpu_shared_uniforms,
            gpu_main_error_sender,
            gpu_terminal_error_sender,
        );
    });

    // Spawn Terminal render thread
    let shader_file_path = cli.shader_file.clone();
    let terminal_thread = thread::spawn(move || {
        let terminal_renderer = TerminalRenderer::new(width as u32, height as u32);
        if let Err(e) = terminal_renderer.run_terminal_thread(
            terminal_frame_buffer,
            terminal_shared_uniforms,
            terminal_main_error_sender,
            terminal_error_receiver,
            &shader_file_path,
        ) {
            eprintln!("Terminal thread error: {e}");
        }
    });

    // Main thread handles error coordination and shutdown
    loop {
        match main_error_receiver.recv() {
            Ok(ThreadError::Shutdown) => {
                // User requested quit - threads will naturally exit
                break;
            }
            Ok(ThreadError::ShaderCompilationError(_)) => {
                // Shader compilation errors are now handled by the terminal thread
                // and displayed in the UI, so we just continue here
            }
            Ok(ThreadError::ShaderReloadSuccess) => {
                // Shader reload success is handled by the terminal thread
                // and clears the error state, so we just continue here
            }
            Ok(ThreadError::GpuError(_)) => {
                // GPU errors are now handled by the terminal thread
                // and displayed in the UI, so we just continue here
            }
            Ok(ThreadError::TerminalError(msg)) => {
                // Terminal error is more serious - exit
                eprintln!("Terminal error: {msg}");
                break;
            }
            Err(_) => {
                // Channel closed - threads have exited
                break;
            }
        }
    }

    // Wait for threads to finish (they should exit naturally on shutdown signal)
    // Note: GPU thread runs in infinite loop, so we don't join it
    // The process exit will clean it up
    let _ = terminal_thread.join();

    Ok(())
}
