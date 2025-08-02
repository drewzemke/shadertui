# Ticket #005: Implement Shader Shell Architecture

**Date:** August 02, 2025
**Priority:** High
**Estimated Effort:** 2-3 hours

## Problem Statement

The current system requires users to write specific compute shaders that are compatible with the terminal renderer, then uses fragile regex transformations to adapt them for the windowed renderer. This approach is brittle because:

1. Regex replacements in `src/renderers/window_renderer.rs:194-220` depend on exact shader syntax
2. Any deviation from expected patterns breaks the transformation
3. Users must understand low-level GPU buffer management instead of focusing on shader logic
4. Different output formats between terminal (storage buffer) and window (texture storage) create complexity

## Description

Replace the current regex-based shader transformation with a "shell shader" architecture. Create two internal shell shaders (one for terminal, one for window) that handle the renderer-specific boilerplate, while users only need to implement a simple color computation function.

This provides a clean, stable API where users write `fn compute_color(uv: vec2<f32>) -> vec3<f32>` and the shells handle all the complexity of storage buffers, texture writes, coordinate systems, and workgroup management.

## Acceptance Criteria

- [ ] Create terminal shell shader that calls user's `compute_color` function
- [ ] Create window shell shader that calls user's `compute_color` function  
- [ ] Remove regex transformation logic from `src/renderers/window_renderer.rs:194-220`
- [ ] Update shader compilation to inject user code into appropriate shell
- [ ] Add compile-time validation for required `compute_color` function signature
- [ ] Ensure both renderers produce identical visual output
- [ ] Update existing example shader to use new format
- [ ] Verify import system (`// @import`) works with new architecture

## Implementation Details

### Shell Structure
Both shells will have this structure:
```wgsl
// Renderer-specific declarations (storage buffer OR texture)
@group(0) @binding(1) var<uniform> uniforms: Uniforms;

// User code injection point
${USER_SHADER_CODE}

// Renderer-specific main function that calls compute_color(uv)
```

### Files to Modify
- `src/renderers/window_renderer.rs` - Remove `adapt_shader_for_texture()`, add shell injection
- `src/renderers/gpu_renderer.rs` - Update to use terminal shell
- Create shell templates (possibly as string constants or embedded files)
- Update shader compilation pipeline to inject user code
- `shaders/example.wgsl` - Convert to new format

### Validation Approach
During compilation, check that user shader contains:
- `fn compute_color(uv: vec2<f32>) -> vec3<f32>` function signature
- No conflicting global declarations

## Definition of Done

- [ ] Code passes linting (`cargo clippy`)
- [ ] Code passes type checking (`cargo build`)
- [ ] Both terminal and window renderers work with new shell system
- [ ] No more regex transformations in codebase
- [ ] Example shader converted and working identically in both modes
- [ ] Manual testing confirms visual output is identical between renderers 
