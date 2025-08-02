# Ticket #004: Color Space Mismatch Between Renderers

**Date:** August 2, 2025
**Priority:** Medium
**Estimated Effort:** 2-3 hours

## Problem Statement

The terminal renderer and windowed renderer produce different color output when running the same shader. This suggests there's a color space conversion issue or different color space assumptions between the two rendering paths that need to be identified and corrected for consistent visual output.

## Description

When the same shader is rendered in both terminal mode and windowed mode, the colors appear different. This inconsistency creates a poor developer experience as shaders don't look the same across different output modes. The issue is likely related to:

- Different color space assumptions (sRGB vs linear RGB)
- Missing gamma correction in one or both renderers
- Different color format handling between terminal and GPU output
- Incorrect color space conversions during the rendering pipeline

## Acceptance Criteria

- [ ] Investigate color space handling in terminal renderer
- [ ] Investigate color space handling in windowed renderer  
- [ ] Identify the specific differences causing color mismatch
- [ ] Implement consistent color space handling across both renderers
- [ ] Verify same shader produces visually identical colors in both modes
- [ ] Test with multiple shaders to ensure fix is comprehensive

## Implementation Details

### Files to investigate:
- Terminal renderer implementation (likely in `src/` directory)
- Windowed renderer implementation  
- Color conversion/processing code
- GPU texture format specifications

### Investigation steps:
1. Compare color space assumptions in both renderers
2. Check if gamma correction is applied consistently
3. Verify RGB format handling (8-bit vs float, linear vs sRGB)
4. Look for color space conversion functions
5. Test with known color values to identify differences

### Potential solutions:
- Add explicit color space conversions
- Implement consistent gamma correction
- Standardize on a single color space (likely sRGB)
- Add color space conversion utilities

## Definition of Done

- [ ] Color output is visually identical between terminal and windowed renderers
- [ ] Code includes clear documentation of color space handling
- [ ] Test shaders demonstrate consistent color reproduction
- [ ] Code passes linting (`cargo clippy`)
- [ ] Code passes type checking (`cargo build`)