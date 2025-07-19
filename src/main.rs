mod app;
mod cli;
mod event_loop;
mod file_watcher;
mod gpu;
mod terminal;

use cli::Cli;
use event_loop::run_event_loop;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (cli, shader_source) = Cli::parse_and_load()?;
    run_event_loop(cli, shader_source)
}
