# ShaderTUI

A terminal-based shader development environment that brings GPU-accelerated fragment shaders to the command line.

## Features

- **GPU-accelerated rendering**: Execute WGSL compute shaders on the GPU, rendered to your terminal
- **Hot reload**: Automatically reloads and recompiles shaders when files change
- **Real-time uniforms**: Time, resolution, cursor position, frame count, and delta time
- **Interactive controls**: Arrow keys control cursor, spacebar pauses/resumes time
- **Performance monitoring**: FPS tracking and frame drop counting
- **Frame rate control**: Configurable terminal refresh rate
- **WGSL import system**: Modular shader development with `// @import "path"` syntax
- **Windowed mode**: Full GPU rendering in an OS window as alternative to terminal

## Installation

```bash
cargo install --git https://github.com/drewzemke/shadertui
```

## Usage

```bash
# Basic usage
shadertui example.wgsl

# With performance monitoring
shadertui --perf example.wgsl

# Limit terminal refresh rate
shadertui --max-fps 30 example.wgsl

# Windowed mode
shadertui --window example.wgsl

# Combined options
shadertui --window --perf shader.wgsl
```

### Controls

- **Arrow keys**: Move cursor position (mouse also works in windowed mode)
- **Spacebar**: Pause/resume time
- **Q/Escape or Ctrl+C**: Exit

### Shader Format

Write WGSL compute shaders with this structure:

```wgsl
@group(0) @binding(0) var<storage, read_write> output: array<vec4<f32>>;
@group(0) @binding(1) var<uniform> uniforms: Uniforms;

struct Uniforms {
    resolution: vec2<f32>,    // Terminal resolution (cols, rows*2)
    time: f32,               // Seconds since start
    cursor: vec2<f32>,       // Normalized cursor position (0-1)
    frame: u32,              // Frame number
    delta_time: f32,         // Time since last frame
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let coords = vec2<f32>(f32(id.x), f32(id.y));
    let uv = coords / uniforms.resolution;
    
    // Your shader code here
    let color = vec3<f32>(uv.x, uv.y, sin(uniforms.time));
    
    let index = id.y * u32(uniforms.resolution.x) + id.x;
    output[index] = vec4<f32>(color, 1.0);
}
```

### WGSL Import System

Create modular shaders using import statements:

```wgsl
// @import "utils.wgsl"
// @import "noise/perlin.wgsl"

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let uv = get_uv(id); // function from utils.wgsl
    let noise_val = perlin_noise(uv); // function from perlin.wgsl
    // ...
}
```

Import paths are relative to the importing file. The system tracks dependencies and triggers hot reload when any imported file changes.

### Windowed Mode

Render shaders in a resizable window instead of terminal:

```bash
# Basic windowed rendering
shadertui --window shader.wgsl

# With performance monitoring in title bar
shadertui --window --perf shader.wgsl
```

Windowed mode provides the same interactive controls and hot reload functionality as terminal mode, with full pixel-level rendering quality.

## Future Considerations

- GLSL fragment shader support 
- GPU stuff: multiple render passes, texture loading, etc.
- Live uniform editing
- Terminal resize handling
- Screenshots/recording

## License

MIT
