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
use clap::{Parser, Subcommand};

/// Configurable human-in-the-loop workflow runner for Claude Code.
#[derive(Parser)]
#[command(name = "ywflow", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialise ywflow in the current project (creates ywflow.yaml scaffold).
    Setup,
    // Workflow steps defined in ywflow.yaml are registered dynamically at
    // runtime — they do not appear as enum variants here.
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Setup => {
            // TODO: delegate to setup handler
            todo!()
        }
    }
}
