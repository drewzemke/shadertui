# Ticket #001: Module Structure Improvements

**Date:** July 20, 2025
**Priority:** Low
**Estimated Effort:** 2-3 hours

## Problem Statement

The current module structure has some organizational inconsistencies that could be improved for better maintainability and discoverability. The main issues are:

1. Mixed abstraction levels at the root level (high-level `gpu_renderer.rs` alongside lower-level modules)
2. Empty `src/shaders/` directory that serves no purpose
3. Some modules could benefit from better grouping by domain

## Description

The project currently has 8 modules at the root level in `src/`, mixing different abstraction levels and concerns. While the `gpu/` subdirectory demonstrates good modular organization, other related functionality remains scattered at the root level.

Current structure analysis:
- **Core orchestration**: `main.rs`, `threaded_event_loop.rs` 
- **Domain-specific renderers**: `gpu_renderer.rs`, `terminal_renderer.rs`
- **Supporting utilities**: `cli.rs`, `file_watcher.rs`, `threading.rs`, `validation.rs`
- **GPU utilities**: Well-organized in `gpu/` subdirectory
- **Unused**: Empty `src/shaders/` directory

## Acceptance Criteria

- [ ] Remove empty `src/shaders/` directory
- [ ] Group related renderer modules into a `renderers/` subdirectory
- [ ] Group utility modules into a `utils/` subdirectory  
- [ ] Update all module imports in affected files
- [ ] Verify application still compiles and runs correctly

## Implementation Details

**Suggested new structure:**
```
src/
├── main.rs
├── threaded_event_loop.rs
├── renderers/
│   ├── mod.rs
│   ├── gpu_renderer.rs
│   └── terminal_renderer.rs
├── utils/
│   ├── mod.rs
│   ├── cli.rs
│   ├── file_watcher.rs
│   ├── threading.rs
│   └── validation.rs
└── gpu/
    ├── mod.rs
    ├── buffer.rs
    ├── device.rs
    ├── pipeline.rs
    └── uniforms.rs
```

**Files to modify:**
- Create `src/renderers/mod.rs` and `src/utils/mod.rs`
- Move renderer and utility files to appropriate subdirectories
- Update imports in `src/main.rs:1-8`
- Update any cross-module imports in moved files

## Definition of Done

- [ ] Code passes linting (cargo clippy)
- [ ] Code passes type checking (cargo build)
- [ ] Application runs without errors
- [ ] All AIDEV anchor comments remain intact after moves 
