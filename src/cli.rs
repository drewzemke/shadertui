use std::fs;
use std::path::PathBuf;

use clap::Parser;

use crate::validation::validate_shader;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(after_help = "EXAMPLES:
    shadertui example.wgsl                    # Basic usage
    shadertui --perf example.wgsl             # With performance monitoring
    shadertui --max-fps 30 example.wgsl       # Limit terminal refresh to 30 FPS
    shadertui --perf --max-fps 10 shader.wgsl # Performance monitoring with 10 FPS cap")]
pub struct Cli {
    /// Path to the WGSL shader file
    pub shader_file: PathBuf,

    /// Enable performance monitoring display
    #[arg(short, long)]
    pub perf: bool,

    /// Maximum terminal frame rate (frames per second)
    #[arg(long, value_name = "FPS")]
    pub max_fps: Option<u32>,
}

impl Cli {
    pub fn parse_and_load() -> Result<(Self, String), Box<dyn std::error::Error>> {
        // Parse command line arguments
        let cli = Self::parse();

        // Load shader file
        let shader_source = match fs::read_to_string(&cli.shader_file) {
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

        // Validate shader compilation before proceeding
        if let Err(e) = validate_shader(&shader_source) {
            eprintln!("Shader compilation error: {e}");
            std::process::exit(1);
        }

        Ok((cli, shader_source))
    }
}
