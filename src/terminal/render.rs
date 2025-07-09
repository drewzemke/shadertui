use crate::terminal::DoubleBuffer;

fn float_rgb_to_u8(r: f32, g: f32, b: f32) -> (u8, u8, u8) {
    let r = (r * 255.0) as u8;
    let g = (g * 255.0) as u8;
    let b = (b * 255.0) as u8;
    (r, g, b)
}

pub fn update_buffer_from_gpu_data(
    buffer: &mut DoubleBuffer,
    gpu_data: &[f32],
    gpu_width: u32,
    _gpu_height: u32,
) {
    buffer.clear_next();

    // Each terminal cell represents 2 vertical pixels (top and bottom half)
    // Terminal height represents the number of character cells
    for y in 0..buffer.height {
        for x in 0..buffer.width {
            // Calculate GPU pixel rows for top and bottom halves of this terminal cell
            let top_pixel_y = y * 2;
            let bottom_pixel_y = y * 2 + 1;

            // AIDEV-NOTE: Critical fix - must use gpu_width (not terminal width) for indexing
            // because GPU buffer is laid out with GPU resolution, not terminal resolution
            // Using vec4 (4 floats) instead of vec3 (3 floats) for proper GPU alignment
            let top_idx = (top_pixel_y * gpu_width as usize + x) * 4;
            let (top_r, top_g, top_b) = if top_idx + 2 < gpu_data.len() {
                (
                    gpu_data[top_idx],
                    gpu_data[top_idx + 1],
                    gpu_data[top_idx + 2],
                )
            } else {
                (0.0, 0.0, 0.0)
            };

            // Get bottom half color - use gpu_width for proper indexing
            let bottom_idx = (bottom_pixel_y * gpu_width as usize + x) * 4;
            let (bottom_r, bottom_g, bottom_b) = if bottom_idx + 2 < gpu_data.len() {
                (
                    gpu_data[bottom_idx],
                    gpu_data[bottom_idx + 1],
                    gpu_data[bottom_idx + 2],
                )
            } else {
                (0.0, 0.0, 0.0)
            };

            // Convert to 0-255 range for RGB colors
            let (top_r, top_g, top_b) = float_rgb_to_u8(top_r, top_g, top_b);
            let (bottom_r, bottom_g, bottom_b) = float_rgb_to_u8(bottom_r, bottom_g, bottom_b);

            // Use ▀ character: foreground = top half, background = bottom half
            // 24-bit RGB: \x1b[38;2;r;g;b;m for foreground, \x1b[48;2;r;g;b;m for background
            let content = format!(
                "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m▀\x1b[0m",
                top_r, top_g, top_b, bottom_r, bottom_g, bottom_b
            );

            buffer.set_cell(x, y, content);
        }
    }
}
