mod gpu;
mod renderers;
mod threaded_event_loop;
mod utils;

use threaded_event_loop::run_threaded_event_loop;
use utils::Cli;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (cli, shader_source) = Cli::parse_and_load()?;
    run_threaded_event_loop(cli, shader_source)
}
