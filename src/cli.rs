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
    use crate::config::{Config, StepConfig};
    use indexmap::IndexMap;

    fn two_step_config() -> Config {
        let mut workflow = IndexMap::new();
        workflow.insert(
            "plan".to_string(),
            StepConfig {
                description: "Plan the work".to_string(),
                args: vec![],
            },
        );
        workflow.insert(
            "execute".to_string(),
            StepConfig {
                description: "Execute the plan".to_string(),
                args: vec![],
            },
        );
        Config { workflow }
    }

    #[test]
    fn no_config_help() {
        let cmd = build_command(None);
        let subcommand_names: Vec<&str> = cmd.get_subcommands().map(|s| s.get_name()).collect();
        assert_eq!(subcommand_names, vec!["setup"]);
    }

    #[test]
    fn with_config_help() {
        let config = two_step_config();
        let cmd = build_command(Some(&config));
        let subcommand_names: Vec<&str> = cmd.get_subcommands().map(|s| s.get_name()).collect();
        assert!(subcommand_names.contains(&"setup"));
        assert!(subcommand_names.contains(&"plan"));
        assert!(subcommand_names.contains(&"execute"));

        let plan_sub = cmd.find_subcommand("plan").unwrap();
        assert_eq!(
            plan_sub.get_about().map(|s| s.to_string()),
            Some("Plan the work".to_string())
        );
        let execute_sub = cmd.find_subcommand("execute").unwrap();
        assert_eq!(
            execute_sub.get_about().map(|s| s.to_string()),
            Some("Execute the plan".to_string())
        );
    }
}
