mod cli;
mod file_watcher;
mod gpu;
mod gpu_renderer;
mod terminal_renderer;
mod threaded_event_loop;
mod threading;
mod validation;

use cli::Cli;
use threaded_event_loop::run_threaded_event_loop;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (cli, shader_source) = Cli::parse_and_load()?;
    run_threaded_event_loop(cli, shader_source)
}
