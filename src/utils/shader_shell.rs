use std::error::Error;
use std::fmt;

// AIDEV-NOTE: Shell templates for different rendering modes
const TERMINAL_SHELL: &str = include_str!("../shaders/terminal_shell.wgsl");
const WINDOW_SHELL: &str = include_str!("../shaders/window_shell.wgsl");
const WINDOW_DISPLAY_SHADER: &str = include_str!("../shaders/window_display.wgsl");

const USER_INJECTION_MARKER: &str = "// USER_SHADER_INJECTION_POINT";

#[derive(Debug, Clone, Copy)]
pub enum ShellType {
    Terminal,
    Window,
}

#[derive(Debug)]
pub enum ShaderShellError {
    MissingComputeColorFunction,
    InjectionMarkerNotFound,
}

impl fmt::Display for ShaderShellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShaderShellError::MissingComputeColorFunction => {
                write!(f, "User shader must contain 'fn compute_color(coords: vec2<f32>) -> vec3<f32>' function")
            }
            ShaderShellError::InjectionMarkerNotFound => {
                write!(f, "Shell template is missing injection marker")
            }
        }
    }
}

impl Error for ShaderShellError {}

// AIDEV-NOTE: Validate that user shader contains required compute_color function
pub fn validate_user_shader(user_shader: &str) -> Result<(), ShaderShellError> {
    // Check for compute_color function signature
    if !user_shader.contains("fn compute_color(coords: vec2<f32>) -> vec3<f32>") {
        return Err(ShaderShellError::MissingComputeColorFunction);
    }
    Ok(())
}

// AIDEV-NOTE: Inject user shader code into the appropriate shell template
pub fn inject_user_shader(
    user_shader: &str,
    shell_type: ShellType,
) -> Result<String, ShaderShellError> {
    // First validate the user shader
    validate_user_shader(user_shader)?;

    // Get the appropriate shell template
    let shell_template = match shell_type {
        ShellType::Terminal => TERMINAL_SHELL,
        ShellType::Window => WINDOW_SHELL,
    };

    // Check that the injection marker exists
    if !shell_template.contains(USER_INJECTION_MARKER) {
        return Err(ShaderShellError::InjectionMarkerNotFound);
    }

    // Replace the injection marker with user code
    let complete_shader = shell_template.replace(USER_INJECTION_MARKER, user_shader);

    Ok(complete_shader)
}

// AIDEV-NOTE: Get the window display shader for the render pipeline
pub fn get_window_display_shader() -> &'static str {
    WINDOW_DISPLAY_SHADER
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_user_shader_valid() {
        let valid_shader = r#"
            fn compute_color(coords: vec2<f32>) -> vec3<f32> {
                let uv = coords / uniforms.resolution;
                return vec3<f32>(uv.x, uv.y, 0.5);
            }
        "#;
        assert!(validate_user_shader(valid_shader).is_ok());
    }

    #[test]
    fn test_validate_user_shader_missing_function() {
        let invalid_shader = r#"
            fn some_other_function() -> f32 {
                return 1.0;
            }
        "#;
        assert!(matches!(
            validate_user_shader(invalid_shader),
            Err(ShaderShellError::MissingComputeColorFunction)
        ));
    }

    #[test]
    fn test_inject_user_shader_terminal() {
        let user_shader = r#"
            fn compute_color(coords: vec2<f32>) -> vec3<f32> {
                let uv = coords / uniforms.resolution;
                return vec3<f32>(uv.x, uv.y, 0.5);
            }
        "#;

        let result = inject_user_shader(user_shader, ShellType::Terminal);
        assert!(result.is_ok());

        let complete_shader = result.unwrap();
        assert!(complete_shader.contains("@group(0) @binding(0) var<storage, read_write> output"));
        assert!(complete_shader.contains("fn compute_color(coords: vec2<f32>) -> vec3<f32>"));
        assert!(!complete_shader.contains(USER_INJECTION_MARKER));
    }

    #[test]
    fn test_inject_user_shader_window() {
        let user_shader = r#"
            fn compute_color(coords: vec2<f32>) -> vec3<f32> {
                let uv = coords / uniforms.resolution;
                return vec3<f32>(uv.x, uv.y, 0.5);
            }
        "#;

        let result = inject_user_shader(user_shader, ShellType::Window);
        assert!(result.is_ok());

        let complete_shader = result.unwrap();
        assert!(complete_shader
            .contains("@group(0) @binding(0) var output_texture: texture_storage_2d"));
        assert!(complete_shader.contains("fn compute_color(coords: vec2<f32>) -> vec3<f32>"));
        assert!(!complete_shader.contains(USER_INJECTION_MARKER));
    }
}
