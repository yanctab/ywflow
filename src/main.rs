// src/main.rs
// Entry point for ywflow — parses the CLI, loads config, and dispatches to the
// appropriate handler.  Business logic lives in the modules below; this file
// contains wiring only.

mod cli;
pub mod config;
mod context;
mod input;
mod plugins;
mod workflow;

use anyhow::Result;

fn main() -> Result<()> {
    let config = config::load().ok();
    let cmd = cli::build_command(config.as_ref());
    let matches = cmd.get_matches();

    match matches.subcommand() {
        Some(("setup", _)) => {
            // TODO: delegate to setup handler
            todo!()
        }
        Some((name, _)) => {
            // TODO: delegate to workflow step handler
            eprintln!("Running step: {name}");
            todo!()
        }
        None => {
            // No subcommand given; print help
            cli::build_command(config.as_ref()).print_help()?;
        }
    }

    Ok(())
}
