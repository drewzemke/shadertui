use winit::dpi::PhysicalPosition;
use winit::event_loop::ActiveEventLoop;

const WINDOW_WIDTH: u32 = 1280;
const WINDOW_HEIGHT: u32 = 800;

// AIDEV-NOTE: Calculate centered window position using the active event loop
pub fn get_centered_window_position(event_loop: &ActiveEventLoop) -> PhysicalPosition<i32> {
    if let Some(monitor) = event_loop.primary_monitor() {
        let monitor_size = monitor.size();
        let x = (monitor_size.width as i32 - WINDOW_WIDTH as i32) / 2;
        let y = (monitor_size.height as i32 - WINDOW_HEIGHT as i32) / 2;
        PhysicalPosition::new(x.max(0), y.max(0))
    } else {
        // Fallback to a reasonable default if monitor detection fails
        PhysicalPosition::new(100, 100)
    }
}

pub fn get_window_size() -> (u32, u32) {
    (WINDOW_WIDTH, WINDOW_HEIGHT)
}
