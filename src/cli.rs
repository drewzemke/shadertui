use std::fs;
use std::path::PathBuf;

use clap::Parser;

use crate::app::validate_shader;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Path to the WGSL shader file
    pub shader_file: PathBuf,

    /// Enable performance monitoring display
    #[arg(short, long)]
    pub perf: bool,
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
