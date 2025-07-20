use std::sync::{Arc, Mutex};
use std::time::Instant;

// AIDEV-NOTE: Shared frame buffer for GPU→Terminal communication with frame dropping
#[derive(Debug, Clone)]
pub struct FrameData {
    pub gpu_data: Vec<f32>,
    pub width: u32,
    pub height: u32,
    pub frame_number: u32,
    pub timestamp: Instant,
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

    pub fn reset_drop_counter(&mut self) {
        self.frames_dropped = 0;
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
    TerminalError(String),
    Shutdown,
}

pub type ErrorSender = std::sync::mpsc::Sender<ThreadError>;
pub type ErrorReceiver = std::sync::mpsc::Receiver<ThreadError>;
