mod app;
mod cli;
mod event_loop;
mod file_watcher;
mod gpu;
mod gpu_renderer;
mod terminal;
mod terminal_renderer;
mod threaded_event_loop;
mod threading;

use cli::Cli;
use threaded_event_loop::run_threaded_event_loop;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (cli, shader_source) = Cli::parse_and_load()?;
    run_threaded_event_loop(cli, shader_source)
}
