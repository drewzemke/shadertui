# Refactor Module Structure

## Issue
The current module structure has some organizational issues that could be improved for better maintainability and single responsibility.

## Specifics
- `src/main.rs` contains the App struct which should be in its own module
- GPU-related functionality is well-modularized but could use better separation
- Terminal rendering logic is split but could be consolidated

## Solution
1. Move App struct to `src/app.rs` or `src/lib.rs`
2. Consider creating a unified `src/renderer.rs` that coordinates GPU and terminal
3. Ensure each module has a single, clear responsibility
4. Add proper module documentation

## Acceptance Criteria
- [ ] Main.rs only contains main function and basic setup
- [ ] App logic is in its own module
- [ ] Module boundaries are clear and logical
- [ ] Each module follows single responsibility principle