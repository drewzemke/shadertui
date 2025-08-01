# ShaderTUI - Product Requirements Document

## Overview

ShaderTUI is a terminal-based shader development environment that brings GPU-accelerated fragment shaders to the command line. Users can write WGSL compute shaders that render to their terminal using Unicode half-cell characters, with hot reload capabilities for rapid iteration.

## Core Features

### Primary Functionality
- **GPU-accelerated rendering**: Uses wgpu to execute WGSL compute shaders on the GPU
- **Terminal graphics**: Renders to terminal using Unicode half-cell characters (▀) with 24-bit color
- **Hot reload**: Automatically reloads and recompiles shaders when files change
- **Real-time uniforms**: Provides time, resolution, cursor position, frame count, and delta time
- **Interactive controls**: Arrow keys control cursor position, spacebar pauses/resumes time

### Technical Architecture
- **Multi-threaded**: GPU computation and terminal rendering run in separate threads
- **Shared framebuffer**: Threads communicate via `Arc<Mutex<framebuffer>>`
- **Frame dropping**: GPU thread drops frames if terminal rendering is the bottleneck
- **Double buffering**: Terminal rendering uses differential updates for performance

## User Experience

### Command Line Interface
```bash
# Basic usage
shadertui example.wgsl

# With performance monitoring
shadertui --perf example.wgsl
shadertui -p example.wgsl

# With configurable terminal frame rate cap
shadertui --max-fps 30 example.wgsl
```

### Controls
- **Arrow keys**: Move cursor position (affects `uniforms.cursor` as normalized coordinates)
- **Spacebar**: Pause/resume time
- **Q or Ctrl+C**: Exit application

### Shader Interface
Users write complete WGSL compute shaders with this structure:
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
    
    // User shader logic here
    let color = vec3<f32>(uv.x, uv.y, sin(uniforms.time));
    
    let index = id.y * u32(uniforms.resolution.x) + id.x;
    output[index] = color;
}
```

### Performance Monitoring
When enabled via `--perf`, displays in top row of terminal:
- **FPS**: Complete loop time (GPU compute + transfer + terminal render)
- **Frame drops**: Count of dropped frames due to terminal bottleneck

### Error Handling
- **No `unwraps`**: Error cases of `Results` should be meaningfully handled
- **Compilation errors**: Clear screen and display error message in terminal
- **No fallbacks**: Application shows errors directly, encouraging users to fix issues
- **File watching**: Simple file change detection with basic stability check

## Implementation Plan

### Phase 1: Minimal GPU Terminal Rendering
- [x] Set up project structure with Cargo.toml dependencies
- [x] Add required dependencies: `wgpu`, `pollster`, `bytemuck`, `crossterm`
- [x] Create basic wgpu device initialization and GPU context
- [x] Implement basic compute shader pipeline with hardcoded simple shader
- [x] Create GPU buffer for RGB output matching terminal resolution
- [x] Implement GPU→CPU buffer readback
- [x] Port existing terminal rendering to use 24-bit RGB from GPU buffer
- [x] Create simple animation loop with hardcoded time uniform

**Verification**: Run the app and see a GPU-computed animated pattern (like a color gradient or simple sine wave pattern) rendering in the terminal using half-cell characters.

### Phase 2: File-Based Shader Loading
- [x] Add `clap` dependency for command line parsing
- [x] Implement basic CLI that accepts shader file as argument
- [x] Create WGSL file loading and parsing
- [x] Add shader compilation and pipeline creation from file
- [x] Create basic uniform buffer with time and resolution
- [x] Add error handling for shader compilation failures (clear screen, show error)
- [x] Create a few example WGSL shaders for testing

**Verification**: Run `shadertui example.wgsl` and see the shader from the file rendering. Test with broken shader files to confirm error handling works.

### Phase 3: Hot Reload System
- [x] Add `notify` dependency for file watching
- [x] Implement file change detection with stability checking
- [x] Add shader recompilation on file change
- [x] Handle compilation errors during hot reload
- [x] Test hot reload workflow with shader editing
- [x] **Bonus**: Modular code architecture - refactored main.rs into focused modules (app.rs, cli.rs, file_watcher.rs, event_loop.rs)
- [x] **Bonus**: Enhanced error handling - compilation errors stop rendering and display persistently without flickering

**Verification**: Run `shadertui example.wgsl`, edit the shader file in another terminal/editor, and watch changes appear immediately in the running app.

### Phase 4: Complete Uniform System
- [x] Expand uniform buffer to include cursor position, frame count, delta time
- [x] Add input handling for arrow keys to control cursor position
- [x] Add spacebar for pause/resume functionality
- [x] Update example shaders to demonstrate all uniform features

**Verification**: Run the app and verify that arrow keys move cursor (affects shader output), spacebar pauses/resumes animation, and all uniforms are working correctly.

### Phase 5: Multi-threading Architecture
- [x] Design shared framebuffer structure with `Arc<Mutex<>>`
- [x] Implement GPU compute thread with continuous rendering loop
- [x] Implement terminal render thread with frame dropping capability
- [x] Add thread synchronization and communication
- [x] Test threading performance and frame dropping behavior

**Verification**: Run the app and confirm it feels more responsive than the single-threaded version. Performance should be similar or better, with smoother rendering.

### Phase 6: Performance Monitoring
- [x] Add `--perf` flag for performance monitoring
- [x] Implement FPS calculation and tracking
- [x] Add frame drop counting
- [x] Create performance display overlay in terminal top row
- [x] Fix overlay flickering by excluding top row from shader rendering
- [x] Test performance monitoring functionality

**Verification**: Run `shadertui --perf example.wgsl` and see FPS and frame drop metrics in the top row of the terminal.

### Phase 7: Implement FPS Cap
- [x] Add `--max-fps` flag for terminal frame rate cap
- [x] Add help text and usage examples

**Verification**: Run `shadertui --perf --max-fps 10` and verify the capped framerate

### Phase 8: Polish 
- [x] Documentation and usage examples

**Verification**: Run through all example shaders, test in different terminals, and confirm the app works reliably in various environments.

### Phase 9: WGSL Import System
- [x] **Phase 9.1: Basic Import Processing**
  - [x] Add import detection regex to find `// @import "path"` comments
  - [x] Implement recursive file reading and content inlining
  - [x] Add relative path resolution (relative to importing file)
  - [x] Create basic error handling for missing files
- [x] **Phase 9.2: Dependency Tracking**  
  - [x] Track dependency chains for each shader file
  - [x] Implement circular dependency detection and error reporting
  - [x] Add file modification time tracking for all dependencies
- [x] **Phase 9.3: Hot Reload Integration**
  - [x] Extend file watcher to monitor all dependency files
  - [x] Trigger shader recompilation when any dependency changes
  - [x] Update dependency tracking when main shader imports change

**Verification**: Create `utils.wgsl` with `hash`, `noise`, `fbm` functions. Update `example2.wgsl` to use `// @import "utils.wgsl"` instead of inline functions. Verify hot reload works when editing either file.

### Phase 10: Windowed Rendering Mode

- [x] **Phase 10.1: Basic Window Creation and CLI Integration**
  - [x] Add `winit` dependency to Cargo.toml
  - [x] Add `--window` / `-w` CLI flag to enable window mode
  - [x] Create basic window with correct initial size (1280x800 pixels, centered)
  - [x] Add simple window event loop that can open/close window
  - [x] Update main.rs to route to window mode vs terminal mode based on CLI flag
  - [x] Console shows informational messages when windowed mode starts

  **Verification**: Run `shadertui --window example.wgsl` and confirm a window opens at 1280x800 centered, closes with standard window controls.

- [x] **Phase 10.2: WindowRenderer with wgpu Surface Integration**
  - [x] Create `WindowRenderer` struct in `src/renderers/window_renderer.rs`
  - [x] Implement wgpu surface creation from winit window (no framebuffer needed)
  - [x] Create two-stage pipeline: compute shader writes to storage texture, render shader displays it
  - [x] Add uniform buffer integration for window mode with hardcoded values
  - [x] Test basic shader rendering in window

  **Verification**: ✅ Window displays actual shader output with pixel-level rendering (not terminal characters). Shader renders correctly with hardcoded uniforms.

- [x] **Phase 10.3: Window Event Handling and Controls**
  - [x] Implement time uniform with real-time updates and frame counting
  - [x] Implement window resize handling with automatic uniform resolution updates
  - [x] Add keyboard input handling for existing controls (arrows, spacebar, Q/Escape)
  - [x] Integrate mouse position tracking as alternate means to move cursor uniform
  - [x] Handle window close events properly
  - [x] Implement proper Y-axis coordinate flipping for intuitive cursor control
  - [x] Add bounds checking for cursor movement within window dimensions

  **Verification**: ✅ Arrow keys control cursor position in shader with correct directional mapping, spacebar pauses/resumes animation, Q/Escape exits, window resizing updates shader resolution correctly, mouse movement provides real-time cursor control with proper coordinate transformation.

- [x] **Phase 10.4: Performance Monitoring and Hot Reload Integration**
  - [x] Add performance metrics display to window title bar (FPS, frame drops)
  - [x] Integrate existing file watcher system with window mode
  - [x] Ensure hot reload works identical to terminal mode
  - [x] Add window-specific error handling for surface/GPU issues
  - [x] Test complete workflow with shader editing
  - [x] Enhanced shader adaptation with regex-based pattern matching for robust buffer-to-texture conversion

  **Verification**: ✅ Run `shadertui --window --perf example.wgsl`, confirm performance metrics in title bar, edit shader file and see changes hot reload instantly, error handling works for broken shaders. All windowed mode features now have complete feature parity with terminal mode.

## Example Usage Scenarios

### Basic Shader Development
```bash
# Create a simple animated shader
echo 'compute shader code' > rainbow.wgsl
shadertui rainbow.wgsl
# Edit rainbow.wgsl in another terminal/editor
# Watch changes appear immediately in shadertui
```

### Performance Analysis
```bash
# Monitor performance while developing
shadertui --perf complex_shader.wgsl
# Top row shows: "FPS: 60 | Dropped: 0"
```

### Frame Rate Control
```bash
# Limit terminal updates for battery savings
shadertui --max-fps 30 battery_friendly.wgsl
```

### Windowed Development
```bash
# Render shader in a resizable window for better visual quality
shadertui --window example.wgsl
shadertui -w example.wgsl

# Combine window mode with performance monitoring
shadertui --window --perf complex_shader.wgsl

# Hot reload works with import system in windowed mode
shadertui -w shaders/test_basic_import.wgsl
# Edit imported files and see changes instantly

# All controls work in windowed mode:
# - Arrow keys: Move cursor position
# - Spacebar: Pause/resume animation  
# - Q or Escape: Exit
# - Mouse: Move cursor (alternative to arrow keys)
# - Window resize: Automatically updates shader resolution
```

## Potential Development Issues

### GPU Integration Challenges
- **Device initialization**: Different platforms (Metal, Vulkan, DirectX) may have varying setup requirements
- **Shader compilation**: WGSL compilation errors may be cryptic or platform-specific
- **Buffer management**: GPU memory allocation and deallocation edge cases
- **Performance variability**: Different GPUs may have wildly different performance characteristics

### Threading Complexity
- **Synchronization bugs**: Potential deadlocks or race conditions in framebuffer access
- **Frame dropping logic**: Complex timing interactions between GPU and terminal threads
- **Resource cleanup**: Proper shutdown of GPU resources across threads

### Terminal Compatibility
- **Color support detection**: Some terminals may not support 24-bit color as expected
- **Unicode support**: Half-cell characters may not render correctly in all terminals
- **Performance variation**: Terminal rendering speed varies significantly between applications

### File System Integration
- **File watching reliability**: `notify` crate may have platform-specific behaviors
- **Editor compatibility**: Different editors may write files in ways that trigger multiple events
- **Path handling**: Cross-platform path resolution and file access permissions

### Performance Bottlenecks
- **GPU-CPU transfer**: Readback latency may be higher than expected
- **Terminal rendering**: Terminal update speed may be slower than GPU computation
- **Memory allocation**: Frequent buffer allocations may cause performance issues

## Future Enhancements

### Language Support
- [ ] Add GLSL fragment shader support with automatic translation
- [ ] Support for multiple shader languages in same project
- [ ] Shader language auto-detection from file extensions

### Advanced GPU Features
- [ ] Auto-detection and optimization of workgroup sizes per GPU
- [ ] Support for multiple render passes and ping-pong buffers
- [ ] Texture loading and binding support
- [ ] Compute shader debugging tools

### User Experience Improvements
- [ ] Shader template system for common patterns
- [ ] Live uniform editing via keyboard shortcuts
- [ ] Shader validation and better error messages

### Performance Optimizations
- [ ] Lock-free ring buffer for thread communication
- [ ] Adaptive resolution scaling based on performance
- [ ] GPU timing profiling and optimization suggestions
- [ ] Memory usage monitoring and optimization

### Platform Features
- [ ] Support for lower-color terminals (256-color fallback)
- [ ] Terminal resize handling

### Advanced Controls
- [ ] Custom uniform parameters via command line
- [ ] Keyboard shortcuts for shader parameters
- [ ] Time controls (speed up, slow down, scrub)
- [ ] Screenshot/recording functionality
