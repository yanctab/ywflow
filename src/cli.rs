// src/cli.rs
// CLI argument parsing and dynamic subcommand registration via clap v4.

use crate::config::Config;
use clap::{Arg, Command};

pub fn build_command(config: Option<&Config>) -> Command {
    let setup = Command::new("setup")
        .about("Initialise ywflow in the current project (creates ywflow.yaml scaffold).");

    let mut cmd = Command::new("ywflow")
        .about("Configurable human-in-the-loop workflow runner for Claude Code.")
        .subcommand(setup);

    if let Some(cfg) = config {
        for (name, step) in &cfg.workflow {
            let mut sub = Command::new(name.clone()).about(step.description.clone());
            for arg in &step.args {
                let clap_arg = Arg::new(arg.name.clone())
                    .help(arg.help.clone())
                    .required(arg.required);
                sub = sub.arg(clap_arg);
            }
            cmd = cmd.subcommand(sub);
        }
    }

    cmd
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CliConfig, Config, StepConfig};
    use indexmap::IndexMap;

    fn minimal_config(workflow: IndexMap<String, StepConfig>) -> Config {
        Config {
            required_env: vec![],
            context: IndexMap::new(),
            cli: CliConfig {
                command: "claude".to_string(),
                args: vec![],
            },
            plugins: vec![],
            workflow,
        }
    }

    fn two_step_config() -> Config {
        let mut workflow = IndexMap::new();
        workflow.insert(
            "plan".to_string(),
            StepConfig {
                description: "Plan the work".to_string(),
                args: vec![],
                cli: None,
            },
        );
        workflow.insert(
            "execute".to_string(),
            StepConfig {
                description: "Execute the plan".to_string(),
                args: vec![],
                cli: None,
            },
        );
        minimal_config(workflow)
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

    #[test]
    fn step_args_registered() {
        use crate::config::StepArg;

        let mut workflow = IndexMap::new();
        workflow.insert(
            "plan".to_string(),
            StepConfig {
                description: "Plan the work".to_string(),
                args: vec![StepArg {
                    name: "task".to_string(),
                    accepts: vec![],
                    required: true,
                    help: "The task to plan".to_string(),
                }],
                cli: None,
            },
        );
        let config = minimal_config(workflow);
        let cmd = build_command(Some(&config));

        let plan_sub = cmd.find_subcommand("plan").unwrap();
        let task_arg = plan_sub
            .get_arguments()
            .find(|a| a.get_id() == "task")
            .expect("expected 'task' argument on plan subcommand");
        assert!(
            task_arg.is_required_set(),
            "expected 'task' argument to be required"
        );
    }
}
