use std::io::{stdout, Write};
use std::time::{Duration, Instant};

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode},
    execute,
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};

#[derive(Clone, PartialEq)]
struct Cell {
    content: String,
    is_empty: bool,
}

impl Cell {
    fn new() -> Self {
        Cell {
            content: " ".to_string(),
            is_empty: true,
        }
    }

    fn set_content(&mut self, content: String) {
        self.is_empty = content == " ";
        self.content = content;
    }
}

struct DoubleBuffer {
    current: Vec<Vec<Cell>>,
    next: Vec<Vec<Cell>>,
    width: usize,
    height: usize,
}

impl DoubleBuffer {
    fn new(width: usize, height: usize) -> Self {
        let current = vec![vec![Cell::new(); width]; height];
        let next = vec![vec![Cell::new(); width]; height];

        DoubleBuffer {
            current,
            next,
            width,
            height,
        }
    }

    fn set_cell(&mut self, x: usize, y: usize, content: String) {
        if x < self.width && y < self.height {
            self.next[y][x].set_content(content);
        }
    }

    fn clear_next(&mut self) {
        for row in &mut self.next {
            for cell in row {
                cell.set_content(" ".to_string());
            }
        }
    }

    fn swap_and_get_changes(&mut self) -> Vec<(usize, usize, String)> {
        let mut changes = Vec::new();

        for y in 0..self.height {
            for x in 0..self.width {
                if self.current[y][x] != self.next[y][x] {
                    changes.push((x, y, self.next[y][x].content.clone()));
                }
            }
        }

        std::mem::swap(&mut self.current, &mut self.next);
        changes
    }
}

fn shape_function(dx: f32, dy: f32, t: f32) -> (f32, f32, f32) {
    let distance = (dx * dx + dy * dy).sqrt();
    let angle = dy.atan2(dx);

    // Create colorful patterns based on distance and time
    let r = (0.5 + 0.5 * (distance * 0.1 + t).sin()).clamp(0.0, 1.0);
    let g = (0.5 + 0.5 * (distance * 0.15 + t * 1.5 + angle).sin()).clamp(0.0, 1.0);
    let b = (0.5 + 0.5 * (distance * 0.2 + t * 0.8 - angle).sin()).clamp(0.0, 1.0);

    (r, g, b)
}

fn rgb_to_256_color(r: f32, g: f32, b: f32) -> u8 {
    // Convert 0-1 RGB to 256-color palette (6x6x6 color cube + grayscale)
    let r_idx = (r * 5.0).round() as u8;
    let g_idx = (g * 5.0).round() as u8;
    let b_idx = (b * 5.0).round() as u8;

    // 256-color cube: 16 + 36*r + 6*g + b
    16 + 36 * r_idx + 6 * g_idx + b_idx
}

fn update_buffer(buffer: &mut DoubleBuffer, center_x: u16, center_y: u16, t: f32) {
    buffer.clear_next();

    for y in 0..buffer.height {
        for x in 0..buffer.width {
            let dx = x as f32 - center_x as f32;

            // Get color for top half of cell (y - 0.5)
            let top_dy = ((y as f32 - 0.5) - center_y as f32) * 2.0;
            let (top_r, top_g, top_b) = shape_function(dx, top_dy, t);
            let top_color = rgb_to_256_color(top_r, top_g, top_b);

            // Get color for bottom half of cell (y + 0.5)
            let bottom_dy = ((y as f32 + 0.5) - center_y as f32) * 2.0;
            let (bottom_r, bottom_g, bottom_b) = shape_function(dx, bottom_dy, t);
            let bottom_color = rgb_to_256_color(bottom_r, bottom_g, bottom_b);

            // Create the content string with colors for both halves
            // The ▀ character shows the foreground color on top and background color on bottom
            let content = format!(
                "\x1b[38;5;{}m\x1b[48;5;{}m▀\x1b[0m",
                top_color, bottom_color
            );

            buffer.set_cell(x, y, content);
        }
    }
}

fn main() -> std::io::Result<()> {
    // Enter alternate screen and hide cursor
    execute!(stdout(), EnterAlternateScreen, Hide)?;

    // Enable raw mode for better control
    terminal::enable_raw_mode()?;

    // Clear screen once
    execute!(stdout(), Clear(ClearType::All))?;

    // Get terminal size
    let (width, height) = terminal::size()?;

    // Calculate center position
    let center_x = width / 2;
    let center_y = height / 2;

    let mut stdout = stdout();
    let start_time = Instant::now();

    // Initialize double buffer
    let mut buffer = DoubleBuffer::new(width as usize, height as usize);

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
        let t = start_time.elapsed().as_secs_f32();

        // Update the buffer with new frame data
        update_buffer(&mut buffer, center_x, center_y, t);

        // Get only the changes between frames
        let changes = buffer.swap_and_get_changes();

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
    terminal::disable_raw_mode()?;

    Ok(())
}
