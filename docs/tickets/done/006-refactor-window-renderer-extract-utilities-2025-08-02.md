# Ticket #006: Refactor WindowRenderer Extract Utilities

**Date:** August 2, 2025
**Priority:** Medium
**Estimated Effort:** 2-3 hours

## Problem Statement

The `src/renderers/window_renderer.rs` file has grown to 676 lines and violates the Single Responsibility Principle. It handles GPU device setup, surface configuration, pipeline creation, texture management, time tracking, input state, performance monitoring, and hot reloading - all in a single struct. This creates several code smells:

- **Code duplication**: Storage texture creation, sampler creation, and bind group creation are repeated across `new()`, `resize()`, and `reload_shader()` methods
- **High complexity**: The file is difficult to navigate and understand
- **Poor testability**: Individual components cannot be unit tested in isolation
- **Maintenance burden**: Changes to GPU resource creation require touching multiple locations

## Description

Refactor `WindowRenderer` by extracting utility modules into a dedicated `src/renderers/window/` directory. This will improve maintainability, reduce code duplication, enable better testing, and follow the Single Responsibility Principle. The main renderer will become a coordinator that delegates to focused utility modules.

## Acceptance Criteria

- [ ] Create `src/renderers/window/` module directory
- [ ] Extract GPU resource management into `src/renderers/window/gpu_resources.rs`
- [ ] Extract pipeline creation into `src/renderers/window/pipeline_factory.rs`
- [ ] Extract surface management into `src/renderers/window/surface_manager.rs`
- [ ] Extract renderer state into `src/renderers/window/window_state.rs`
- [ ] Refactor `WindowRenderer` to use the extracted utilities
- [ ] Eliminate code duplication in texture/sampler/bind group creation
- [ ] Reduce `window_renderer.rs` from 676+ lines to approximately 200 lines
- [ ] Preserve all existing functionality (no behavioral changes)
- [ ] Update module exports in `src/renderers/window/mod.rs`

## Implementation Details

### File Structure
```
src/renderers/window/
├── mod.rs                 # Module exports
├── gpu_resources.rs       # GPU resource creation utilities
├── pipeline_factory.rs    # Pipeline creation logic
├── surface_manager.rs     # Surface configuration management
└── window_state.rs        # Renderer state management
```

### GpuResourceManager (`src/renderers/window/gpu_resources.rs`)
```rust
pub struct GpuResourceManager {
    device: Arc<wgpu::Device>,
}

impl GpuResourceManager {
    pub fn create_storage_texture(&self, width: u32, height: u32) -> wgpu::Texture { ... }
    pub fn create_sampler(&self) -> wgpu::Sampler { ... }
    pub fn create_compute_bind_group(&self, ...) -> wgpu::BindGroup { ... }
    pub fn create_render_bind_group(&self, ...) -> wgpu::BindGroup { ... }
}
```

### PipelineFactory (`src/renderers/window/pipeline_factory.rs`)
```rust
pub struct PipelineFactory;

impl PipelineFactory {
    pub fn create_compute_pipeline(device: &wgpu::Device, shader: &str) -> Result<...> { ... }
    pub fn create_render_pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> Result<...> { ... }
}
```

### SurfaceManager (`src/renderers/window/surface_manager.rs`)
```rust
pub struct SurfaceManager {
    surface: wgpu::Surface<'static>,
    adapter: wgpu::Adapter,
}

impl SurfaceManager {
    pub fn configure(&self, device: &wgpu::Device, width: u32, height: u32) { ... }
    pub fn get_optimal_format(&self) -> wgpu::TextureFormat { ... }
}
```

### WindowState (`src/renderers/window/window_state.rs`)
```rust
pub struct WindowState {
    pub cursor_position: [f32; 2],
    pub is_paused: bool,
    pub paused_time: f32,
    pub frame_count: u32,
    pub start_time: Instant,
    pub last_frame_time: Instant,
}
```

### Refactored WindowRenderer Structure
```rust
pub struct WindowRenderer {
    surface_manager: SurfaceManager,
    resource_manager: GpuResourceManager,
    compute_pipeline: wgpu::ComputePipeline,
    render_pipeline: wgpu::RenderPipeline,
    compute_bind_group: wgpu::BindGroup,
    render_bind_group: wgpu::BindGroup,
    uniform_buffer: UniformBuffer,
    state: WindowState,
    performance_tracker: Option<PerformanceTracker>,
    width: u32,
    height: u32,
}
```

## Definition of Done

- [ ] All new utility modules are implemented with proper documentation
- [ ] `WindowRenderer` is refactored to use extracted utilities
- [ ] All existing AIDEV-NOTE comments are preserved and updated as needed
- [ ] Code duplication is eliminated (storage texture, sampler, bind group creation)
- [ ] File size of `window_renderer.rs` is reduced to ~200 lines
- [ ] All existing functionality works unchanged (windowed mode operates normally)
- [ ] Code passes linting (`cargo clippy`)
- [ ] Code passes type checking (`cargo build`)
- [ ] Manual testing confirms windowed mode still works (hot reload, resize, input, etc.)