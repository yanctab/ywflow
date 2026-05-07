// src/cli.rs
// CLI argument parsing and dynamic subcommand registration via clap v4.

use crate::config::Config;
use clap::Command;

pub fn build_command(config: Option<&Config>) -> Command {
    let setup = Command::new("setup")
        .about("Initialise ywflow in the current project (creates ywflow.yaml scaffold).");

    let mut cmd = Command::new("ywflow")
        .about("Configurable human-in-the-loop workflow runner for Claude Code.")
        .subcommand(setup);

    if let Some(cfg) = config {
        for (name, step) in &cfg.workflow {
            let sub = Command::new(name.clone()).about(step.description.clone());
            cmd = cmd.subcommand(sub);
        }
    }

    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_config_help() {
        let cmd = build_command(None);
        let subcommand_names: Vec<&str> = cmd.get_subcommands().map(|s| s.get_name()).collect();
        assert_eq!(subcommand_names, vec!["setup"]);
    }
}
