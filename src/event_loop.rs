use std::fs;
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

use crate::app::App;
use crate::cli::Cli;
use crate::file_watcher::FileWatcher;

pub fn run_event_loop(cli: Cli, shader_source: String) -> Result<(), Box<dyn std::error::Error>> {
    // Get terminal size before initializing GPU
    let (width, height) = crossterm_terminal::size()?;

    // Initialize the application BEFORE entering alternate screen
    // This way, any shader compilation errors will display cleanly
    let mut app = match App::new(width as u32, height as u32, &shader_source) {
        Ok(app) => app,
        Err(e) => {
            eprintln!("Shader compilation error: {e}");
            std::process::exit(1);
        }
    };

    // Set up file watcher
    let mut file_watcher = FileWatcher::new(&cli.shader_file)?;

    // Only enter alternate screen after successful initialization
    execute!(stdout(), EnterAlternateScreen, Hide)?;

    // Enable raw mode for better control
    crossterm_terminal::enable_raw_mode()?;

    // Clear screen once
    execute!(stdout(), Clear(ClearType::All))?;

    let mut stdout = stdout();
    let start_time = Instant::now();
    let mut error_state: Option<String> = None;
    let mut displayed_error: Option<String> = None;

    // Animation loop
    loop {
        // Check for file changes
        if file_watcher.check_for_changes() {
            error_state = handle_file_change(&mut app, &cli.shader_file, &mut stdout)?;
        }

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

        // If we're in an error state, display error only if it changed
        if let Some(ref error_msg) = error_state {
            // Only redraw if this is a new error or we haven't displayed it yet
            if displayed_error.as_ref() != Some(error_msg) {
                execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;
                stdout.write_all(format!("{error_msg}\nPress 'q' to quit").as_bytes())?;
                stdout.flush()?;
                displayed_error = Some(error_msg.clone());
            }
            std::thread::sleep(Duration::from_millis(20));
            continue;
        } else {
            // Clear displayed error when we exit error state
            displayed_error = None;
        }

        // Get current time (seconds since start)
        let time = start_time.elapsed().as_secs_f32();

        // Render frame and get changes
        let changes = match app.render_frame(time) {
            Ok(changes) => changes,
            Err(e) => {
                // Print error and continue
                eprintln!("Render error: {e}");
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

fn handle_file_change(
    app: &mut App,
    shader_file: &std::path::Path,
    stdout: &mut std::io::Stdout,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    // Attempt to reload the shader
    match fs::read_to_string(shader_file) {
        Ok(new_shader_source) => {
            match app.reload_shader(&new_shader_source) {
                Ok(()) => {
                    // Clear screen and continue rendering with new shader
                    execute!(stdout, Clear(ClearType::All))?;
                    Ok(None) // No error, clear error state
                }
                Err(e) => {
                    // Return error state - shader will stop rendering
                    Ok(Some(format!("Shader reload error: {e}")))
                }
            }
        }
        Err(e) => {
            // Return file read error state
            Ok(Some(format!("File read error: {e}")))
        }
    }
}
