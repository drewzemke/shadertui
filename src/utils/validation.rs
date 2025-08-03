// AIDEV-NOTE: Validate shader compilation using naga without GPU device
pub fn validate_shader(shader_source: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Parse WGSL using naga frontend
    let module = naga::front::wgsl::parse_str(shader_source)?;

    // Validate the parsed module
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    validator.validate(&module)?;

    Ok(())
}

// AIDEV-NOTE: Validate user shader for hot reload by injecting into shell and validating complete shader
pub fn validate_user_shader_for_reload(
    user_shader_source: &str,
    shell_type: crate::utils::shader_shell::ShellType,
) -> Result<(), Box<dyn std::error::Error>> {
    // Inject user shader into appropriate shell
    let complete_shader =
        crate::utils::shader_shell::inject_user_shader(user_shader_source, shell_type)?;

    // Validate the complete injected shader
    validate_shader(&complete_shader)?;

    Ok(())
}
