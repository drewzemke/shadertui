# Guidelines for Claude in `todoistui`

## Project Context  

ShaderTUI is a terminal-based shader development environment that brings GPU-accelerated fragment shaders to the command line.

## The Golden Rules

When unsure about implementation details, ALWAYS ask the developer.  

We optimize for maintainability over cleverness. When in doubt, choose the boring solution.  

## Development Practice

### Habits

- always read `docs/PRD.md` in order to get context before starting any work
- always use the context7 tool to look up external libraries before using them
- always run `cargo fmt` and `cargo clippy` after finishing a chunk of work
- you cannot run the app yourself, you so must always ask the developer to test manually on your behalf
- always add dependencies using `cargo add`, not by writing directly to the package manifest
- follow YAGNI; keep changes focused on the task at hand; do not do more work that is asked

### Anchor comments  

Add specially formatted comments throughout the codebase, where appropriate, for yourself as inline knowledge that can be easily `grep`ped for.  

- Use `AIDEV-NOTE:`, `AIDEV-TODO:`, or `AIDEV-QUESTION:` (all-caps prefix) for comments aimed at AI and developers.  
- **Important:** Before scanning files, always first try to **grep for existing anchors** `AIDEV-*` in relevant subdirectories.  
- **Update relevant anchors** when modifying associated code.  
- **Do not remove `AIDEV-NOTE`s** without explicit human instruction.  
- Make sure to add relevant anchor comments, whenever a file or piece of code is:  
  * too complex, or  
  * very important, or  
  * confusing, or  
  * could have a bug  

 ### Code Conventions

- keep modules small and focused. try to follow the Single Responsibility Principle
- keep non-anchor comments to a minimum, only using them to explain nonobvious steps. comments should explain the code that is present, never use them to explain why/that changes were made

