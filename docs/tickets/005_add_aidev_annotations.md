# Add AIDEV Annotations

## Issue
Several complex functions and GPU-related code lack AIDEV annotations as specified in CLAUDE.md coding practices.

## Specifics
- GPU buffer management code lacks explanatory comments
- Complex render loop logic needs documentation
- Memory layout and alignment decisions should be documented
- Thread safety considerations need explanation

## Solution
1. Add AIDEV-NOTE comments to GPU buffer creation and management
2. Document render loop timing and frame dropping logic
3. Explain memory alignment requirements for uniforms
4. Add comments for complex indexing calculations
5. Document terminal color encoding logic

## Acceptance Criteria
- [ ] All complex GPU operations have AIDEV annotations
- [ ] Memory layout decisions are documented
- [ ] Critical performance optimizations are explained
- [ ] Thread safety considerations are noted where relevant