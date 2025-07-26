use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Instant;

// AIDEV-NOTE: Shared frame buffer for GPU→Terminal communication with frame dropping
#[derive(Debug, Clone)]
pub struct FrameData {
    pub gpu_data: Vec<f32>,
    pub width: u32,
}

pub struct SharedFrameBuffer {
    // Double buffering: one frame being written by GPU, one being read by terminal
    current_frame: Option<FrameData>,
    next_frame: Option<FrameData>,
    frames_dropped: u64,
}

impl SharedFrameBuffer {
    pub fn new() -> Self {
        Self {
            current_frame: None,
            next_frame: None,
            frames_dropped: 0,
        }
    }

    // AIDEV-NOTE: GPU thread writes new frame, potentially dropping if terminal is slow
    pub fn write_frame(&mut self, frame_data: FrameData) {
        // If there's already a pending frame, we're dropping it
        if self.next_frame.is_some() {
            self.frames_dropped += 1;
        }
        self.next_frame = Some(frame_data);
    }

    // AIDEV-NOTE: Terminal thread reads latest available frame
    pub fn read_frame(&mut self) -> Option<FrameData> {
        // Swap next frame to current if available
        if self.next_frame.is_some() {
            self.current_frame = self.next_frame.take();
        }

        self.current_frame.clone()
    }

    pub fn get_frames_dropped(&self) -> u64 {
        self.frames_dropped
    }
}

// AIDEV-NOTE: Shared uniforms for Terminal→GPU communication
#[derive(Debug, Clone)]
pub struct SharedUniforms {
    pub cursor: [i32; 2],
    pub time_paused: bool,
    pub paused_time: f32,
    pub should_reload_shader: bool,
    pub new_shader_source: Option<String>,
}

impl SharedUniforms {
    pub fn new() -> Self {
        Self {
            cursor: [0, 0],
            time_paused: false,
            paused_time: 0.0,
            should_reload_shader: false,
            new_shader_source: None,
        }
    }

    pub fn move_cursor(&mut self, dx: i32, dy: i32) {
        self.cursor[0] += dx;
        self.cursor[1] += dy;
    }

    pub fn toggle_pause(&mut self, current_time: f32) {
        if self.time_paused {
            self.time_paused = false;
        } else {
            self.time_paused = true;
            self.paused_time = current_time;
        }
    }

    pub fn request_shader_reload(&mut self, shader_source: String) {
        self.should_reload_shader = true;
        self.new_shader_source = Some(shader_source);
    }

    pub fn consume_shader_reload(&mut self) -> Option<String> {
        if self.should_reload_shader {
            self.should_reload_shader = false;
            self.new_shader_source.take()
        } else {
            None
        }
    }
}

// AIDEV-NOTE: Thread-safe wrappers for shared state
pub type SharedFrameBufferHandle = Arc<Mutex<SharedFrameBuffer>>;
pub type SharedUniformsHandle = Arc<Mutex<SharedUniforms>>;

// AIDEV-NOTE: Error types for thread communication
#[derive(Debug, Clone)]
pub enum ThreadError {
    ShaderCompilationError(String),
    ShaderReloadSuccess,
    GpuError(String),
    Shutdown,
}

pub type ErrorSender = std::sync::mpsc::Sender<ThreadError>;
pub type ErrorReceiver = std::sync::mpsc::Receiver<ThreadError>;

// AIDEV-NOTE: Performance monitoring for FPS and frame drop tracking
#[derive(Debug)]
pub struct PerformanceTracker {
    frame_times: VecDeque<Instant>,
    last_fps_calculation: Instant,
    current_fps: f32,
    total_frames_rendered: u64,
    max_frame_history: usize,
}

impl PerformanceTracker {
    pub fn new() -> Self {
        Self {
            frame_times: VecDeque::new(),
            last_fps_calculation: Instant::now(),
            current_fps: 0.0,
            total_frames_rendered: 0,
            max_frame_history: 60, // Track last 60 frames for smooth FPS calculation
        }
    }

    // AIDEV-NOTE: Record a new frame render completion
    pub fn record_frame(&mut self) {
        let now = Instant::now();
        self.frame_times.push_back(now);
        self.total_frames_rendered += 1;

        // Keep only recent frames for FPS calculation
        while self.frame_times.len() > self.max_frame_history {
            self.frame_times.pop_front();
        }

        // Update FPS every 250ms for smooth display
        if now.duration_since(self.last_fps_calculation).as_millis() >= 250 {
            self.update_fps();
            self.last_fps_calculation = now;
        }
    }

    fn update_fps(&mut self) {
        if self.frame_times.len() < 2 {
            self.current_fps = 0.0;
            return;
        }

        let time_span = self
            .frame_times
            .back()
            .unwrap()
            .duration_since(*self.frame_times.front().unwrap())
            .as_secs_f32();

        if time_span > 0.0 {
            self.current_fps = (self.frame_times.len() - 1) as f32 / time_span;
        }
    }

    pub fn get_fps(&self) -> f32 {
        self.current_fps
    }
}

// AIDEV-NOTE: Combined performance tracking for both GPU and Terminal rendering
#[derive(Debug)]
pub struct DualPerformanceTracker {
    pub gpu_tracker: PerformanceTracker,
    pub terminal_tracker: PerformanceTracker,
}

impl DualPerformanceTracker {
    pub fn new() -> Self {
        Self {
            gpu_tracker: PerformanceTracker::new(),
            terminal_tracker: PerformanceTracker::new(),
        }
    }

    pub fn record_gpu_frame(&mut self) {
        self.gpu_tracker.record_frame();
    }

    pub fn record_terminal_frame(&mut self) {
        self.terminal_tracker.record_frame();
    }

    pub fn get_gpu_fps(&self) -> f32 {
        self.gpu_tracker.get_fps()
    }

    pub fn get_terminal_fps(&self) -> f32 {
        self.terminal_tracker.get_fps()
    }
}

pub type DualPerformanceTrackerHandle = Arc<Mutex<DualPerformanceTracker>>;
