# Improve Error Handling

## Issue
Current error handling is minimal and doesn't provide clear feedback. The PRD specifies that errors should clear the screen and show helpful messages.

## Specifics
- PRD.md lines 74-77 specify error handling requirements
- Current implementation prints to stderr and continues
- Missing: clear screen on error, better error messages, graceful recovery

## Solution
1. Add error formatting function for clear display
2. Implement screen clearing on GPU/shader errors
3. Add better error context and suggestions
4. Improve error message formatting for terminal display
5. Add graceful fallback behavior where possible

## Acceptance Criteria
- [ ] Errors clear screen and display helpful messages
- [ ] Error messages are formatted appropriately for terminal
- [ ] GPU initialization errors are handled gracefully
- [ ] Shader compilation errors show clear feedback