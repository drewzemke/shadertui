# Ticket #003: Fix Y-Axis Inconsistency Between Terminal and Window Modes

**Date:** July 26, 2025
**Priority:** High
**Estimated Effort:** 2-3 hours

## Problem Statement

Terminal mode and windowed mode render shaders with flipped Y-axes relative to each other. This creates inconsistent behavior when switching between rendering modes - the same shader appears vertically flipped depending on whether it's rendered in terminal or window mode. This breaks the expectation of consistent visual output across both rendering targets.

## Description

The coordinate system inconsistency likely stems from different Y-axis conventions:
- **Terminal mode**: Uses terminal coordinate system where Y=0 is at top, increasing downward
- **Windowed mode**: Uses standard graphics coordinate system where Y=0 is at bottom, increasing upward

This affects both the visual output of shaders and cursor position mapping, creating a confusing user experience when developing shaders that should look identical across both modes.

## Acceptance Criteria

- [ ] Same shader renders identically in both terminal and windowed mode (no Y-axis flip)
- [ ] Cursor position mapping works consistently across both modes
- [ ] Arrow key controls work identically in both modes (up moves cursor up in both)
- [ ] Mouse position in windowed mode maps correctly to equivalent terminal coordinates
- [ ] UV coordinate system is consistent between modes
- [ ] Existing example shaders render identically in both modes
- [ ] No regression in existing functionality

## Implementation Details

Investigation needed to determine the root cause:

1. **Check coordinate systems in key files:**
   - `src/renderers/terminal_renderer.rs`: How UV coordinates are calculated for terminal output
   - `src/renderers/window_renderer.rs`: How the compute→render pipeline handles coordinates
   - `src/gpu/uniforms.rs`: How cursor position is passed to shaders

2. **Likely fixes:**
   - **Option A**: Flip Y-coordinates in windowed mode uniform updates
   - **Option B**: Flip Y-coordinates in terminal mode rendering
   - **Option C**: Adjust texture coordinate mapping in windowed render shader

3. **Key areas to examine:**
   - Cursor position calculation in `update_cursor_position()` methods
   - UV coordinate calculation in both renderers
   - The vertex shader in `window_renderer.rs` that samples from storage texture

4. **Testing approach:**
   - Create test shader with clear directional indicators (gradient, cursor ripple)
   - Verify cursor position matches between modes
   - Test with existing example shaders

## Definition of Done

- [ ] Visual output is identical between terminal and windowed modes
- [ ] Cursor controls work consistently across both modes
- [ ] All existing example shaders render consistently
- [ ] No regressions in cursor position mapping
- [ ] Arrow key and mouse controls work intuitively in both modes
- [ ] Code passes linting (cargo clippy)
- [ ] Code passes type checking (cargo build)
- [ ] Comprehensive testing with multiple shader examples 
