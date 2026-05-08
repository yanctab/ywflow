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
            let cfg = config.as_ref();
            let plugin_statuses = if let Some(c) = cfg {
                plugins::run_plugin_setup(&c.plugins, &plugins::read_installed_plugins, &|pkg| {
                    let status = std::process::Command::new("npx")
                        .args(["claude-plugins", "install", pkg])
                        .status()?;
                    if status.success() {
                        Ok(())
                    } else {
                        Err(anyhow::anyhow!("npx exited with status {}", status))
                    }
                })
            } else {
                vec![]
            };

            let missing_skills = cfg.map(plugins::verify_skill_files).unwrap_or_default();

            plugins::print_ready_summary(
                cfg.is_some(),
                true,
                None,
                &plugin_statuses,
                &missing_skills,
            );
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
