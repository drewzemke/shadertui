mod gpu;
mod renderers;
mod threaded_event_loop;
mod utils;
mod windowed_event_loop;

use threaded_event_loop::run_threaded_event_loop;
use utils::Cli;
use windowed_event_loop::run_windowed_event_loop;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (cli, shader_source) = Cli::parse_and_load()?;

    if cli.is_windowed_mode() {
        run_windowed_event_loop(cli, shader_source)
    } else {
        run_threaded_event_loop(cli, shader_source)
    }
}
