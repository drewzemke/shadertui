use std::fs;
use std::path::PathBuf;

use clap::Parser;

use crate::utils::{
    shader_import::process_imports,
    shader_shell::{inject_user_shader, ShellType},
    validation::validate_shader,
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(after_help = "EXAMPLES:
    shadertui example.wgsl                    # Basic usage
    shadertui --perf example.wgsl             # With performance monitoring
    shadertui --max-fps 30 example.wgsl       # Limit terminal refresh to 30 FPS
    shadertui --window example.wgsl           # Render in a window instead of terminal
    shadertui --window --perf shader.wgsl     # Windowed mode with performance monitoring")]
pub struct Cli {
    /// Path to the WGSL shader file
    pub shader_file: PathBuf,

    /// Enable performance monitoring display
    #[arg(short, long)]
    pub perf: bool,

    /// Maximum terminal frame rate (frames per second)
    #[arg(long, value_name = "FPS")]
    pub max_fps: Option<u32>,

    /// Render in a window instead of terminal
    #[arg(short, long)]
    pub window: bool,
}

impl Cli {
    pub fn parse_and_load() -> Result<(Self, String), Box<dyn std::error::Error>> {
        // Parse command line arguments
        let cli = Self::parse();

        // Load shader file with import processing
        let raw_shader_source = match fs::read_to_string(&cli.shader_file) {
            Ok(content) => content,
            Err(e) => {
                eprintln!(
                    "Error reading shader file '{}': {}",
                    cli.shader_file.display(),
                    e
                );
                std::process::exit(1);
            }
        };

        let user_shader_source = match process_imports(&cli.shader_file, &raw_shader_source) {
            Ok((processed, _deps)) => processed,
            Err(e) => {
                eprintln!("Import processing error: {e}");
                std::process::exit(1);
            }
        };

        // Inject user shader into terminal shell for validation (use terminal as default)
        let complete_shader_for_validation =
            match inject_user_shader(&user_shader_source, ShellType::Terminal) {
                Ok(complete) => complete,
                Err(e) => {
                    eprintln!("Shader shell injection error: {e}");
                    std::process::exit(1);
                }
            };

        // Validate the complete injected shader
        if let Err(e) = validate_shader(&complete_shader_for_validation) {
            eprintln!("Shader compilation error: {e}");
            std::process::exit(1);
        }

        // Return the original user shader source (not the injected version)
        // Renderers will do their own injection with appropriate shell type
        Ok((cli, user_shader_source))
    }

    pub fn is_windowed_mode(&self) -> bool {
        self.window
    }
}
