use std::time::Instant;

// AIDEV-NOTE: Extracted window state management from WindowRenderer for better organization
pub struct WindowState {
    pub cursor_position: [f32; 2],
    pub is_paused: bool,
    pub paused_time: f32,
    pub frame_count: u32,
    pub start_time: Instant,
    pub last_frame_time: Instant,
}

impl WindowState {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            cursor_position: [0.0, 0.0],
            is_paused: false,
            paused_time: 0.0,
            frame_count: 0,
            start_time: now,
            last_frame_time: now,
        }
    }

    // AIDEV-NOTE: Public methods for controlling renderer state from event loop
    pub fn update_cursor_position(&mut self, x: f32, y: f32, height: u32) {
        // Store cursor in pixel coordinates, flipping Y axis (window Y=0 at top, shader Y=0 at bottom)
        self.cursor_position = [x, height as f32 - y];
        // println!(
        //     "Updated cursor: ({x:.3}, {y:.3}) -> flipped: ({:.3}, {:.3})",
        //     self.cursor_position[0], self.cursor_position[1]
        // );
    }

    pub fn toggle_pause(&mut self) {
        if self.is_paused {
            // Resume: adjust start time to account for pause duration
            let pause_duration = Instant::now().duration_since(self.last_frame_time);
            self.start_time += pause_duration;
            self.is_paused = false;
        } else {
            // Pause: store current time
            self.paused_time = Instant::now().duration_since(self.start_time).as_secs_f32();
            self.is_paused = true;
        }
    }

    pub fn get_current_time(&self) -> f32 {
        if self.is_paused {
            self.paused_time
        } else {
            Instant::now().duration_since(self.start_time).as_secs_f32()
        }
    }

    pub fn update_frame_timing(&mut self) -> f32 {
        let current_time = Instant::now();
        let delta_time = current_time
            .duration_since(self.last_frame_time)
            .as_secs_f32();
        self.last_frame_time = current_time;
        self.frame_count += 1;
        delta_time
    }
}
