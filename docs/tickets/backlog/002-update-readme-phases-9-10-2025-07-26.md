# Ticket #002: Update README with Phase 9 and 10 Features

**Date:** July 26, 2025
**Priority:** Medium
**Estimated Effort:** 1 hour

## Problem Statement

The README file is outdated and doesn't reflect the significant new features implemented in Phase 9 (WGSL Import System) and Phase 10 (Windowed Rendering Mode). Users need comprehensive documentation of these features to understand the full capabilities of ShaderTUI.

## Description

Phase 9 introduced a powerful import system allowing modular shader development with `// @import "path/to/file.wgsl"` syntax, complete with dependency tracking and hot reload integration. Phase 10 added full windowed rendering mode as an alternative to terminal rendering, with complete feature parity including performance monitoring, hot reload, and interactive controls.

The README should be updated to showcase these major capabilities and provide clear usage examples.

## Acceptance Criteria

- [ ] Add section documenting the WGSL import system with syntax examples
- [ ] Document how import dependency tracking works with hot reload
- [ ] Add comprehensive windowed mode documentation with CLI examples
- [ ] Update feature list to include import system and windowed mode
- [ ] Include screenshots or examples showing both terminal and windowed rendering
- [ ] Document all CLI flags including `--window` and `--perf`
- [ ] Add examples of modular shader development workflow
- [ ] Update installation and usage sections as needed
- [ ] Ensure all example commands are accurate and tested

## Implementation Details

Key sections to update in `README.md`:

1. **Features section**: Add import system and windowed mode to main feature list
2. **Usage section**: Update with new CLI flags and examples
3. **Import System section**: New section explaining:
   ```markdown
   ### WGSL Import System
   ```wgsl
   // @import "utils.wgsl"
   // @import "noise/simplex.wgsl"
   ```
4. **Windowed Mode section**: New section with examples:
   ```bash
   # Basic windowed mode
   shadertui --window example.wgsl
   
   # With performance monitoring
   shadertui --window --perf shader.wgsl
   ```
5. **Examples section**: Add modular shader development workflow

## Definition of Done

- [ ] README comprehensively documents Phase 9 import system
- [ ] README comprehensively documents Phase 10 windowed mode
- [ ] All CLI examples are accurate and tested
- [ ] Feature list is updated with new capabilities
- [ ] Documentation follows consistent formatting and style
- [ ] Code passes linting (cargo clippy)
- [ ] Code passes type checking (cargo build)