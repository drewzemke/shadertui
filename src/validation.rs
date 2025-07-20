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
