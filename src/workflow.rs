// src/workflow.rs
// Orchestrates workflow step execution; each step launches a full interactive
// Claude Code session and requires human review before advancing.

use crate::config::{CliConfig, StepConfig};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorkflowError {
    #[error(
        "CLI '{command}' not found\n  → Install Claude Code: https://docs.anthropic.com/en/docs/claude-code"
    )]
    CommandNotFound { command: String },
    #[error("exec failed: {0}")]
    Exec(#[from] std::io::Error),
}

/// Verify that `command` is on the system PATH by checking it exists via `which`.
/// Returns the resolved binary path on success.
pub fn check_command_available(command: &str) -> Result<String, WorkflowError> {
    // Use `which` semantics: walk PATH and return the first matching executable.
    std::env::var("PATH")
        .unwrap_or_default()
        .split(':')
        .map(std::path::Path::new)
        .find_map(|dir| {
            let candidate = dir.join(command);
            if candidate.is_file() {
                Some(candidate.to_string_lossy().into_owned())
            } else {
                None
            }
        })
        .ok_or_else(|| WorkflowError::CommandNotFound {
            command: command.to_string(),
        })
}

/// Assemble the final argv for a step, expanding all `${variable}` tokens.
///
/// Order: `<cli.command> [global_args...] [step_args...]`
pub fn assemble_argv(
    global_cli: &CliConfig,
    step: &StepConfig,
    resolved_vars: &HashMap<String, String>,
) -> Vec<String> {
    let step_args = step
        .cli
        .as_ref()
        .map(|c| c.args.clone())
        .unwrap_or_default();

    global_cli
        .args
        .iter()
        .chain(step_args.iter())
        .map(|arg| expand_tokens(arg, resolved_vars))
        .collect()
}

/// Substitute all `${key}` tokens in `s` using `vars`.
fn expand_tokens(s: &str, vars: &HashMap<String, String>) -> String {
    let mut result = String::new();
    let mut rest = s;
    while let Some(start) = rest.find("${") {
        result.push_str(&rest[..start]);
        let after_open = &rest[start + 2..];
        if let Some(end) = after_open.find('}') {
            let token = &after_open[..end];
            if let Some(val) = vars.get(token) {
                result.push_str(val);
            } else {
                result.push_str("${");
                result.push_str(token);
                result.push('}');
            }
            rest = &after_open[end + 1..];
        } else {
            result.push_str("${");
            rest = after_open;
        }
    }
    result.push_str(rest);
    result
}

/// Launch the step as a fully interactive child process.
///
/// stdin/stdout/stderr are inherited from the parent so the user gets a real terminal.
/// This call blocks until the child process exits.
pub fn run_step(
    global_cli: &CliConfig,
    step: &StepConfig,
    resolved_vars: &HashMap<String, String>,
) -> Result<(), WorkflowError> {
    // Verify the CLI binary is available before trying to exec.
    let command = &global_cli.command;
    check_command_available(command)?;

    let args = assemble_argv(global_cli, step, resolved_vars);

    let status = std::process::Command::new(command)
        .args(&args)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()?
        .wait()?;

    if !status.success() {
        // Non-zero exit from the child is not an error in ywflow — the human
        // controls what happens next. We propagate the exit code though.
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CliConfig, StepCliConfig, StepConfig};

    fn make_global_cli(args: Vec<&str>) -> CliConfig {
        CliConfig {
            command: "claude".to_string(),
            args: args.into_iter().map(|s| s.to_string()).collect(),
        }
    }

    fn make_step(step_args: Vec<&str>) -> StepConfig {
        StepConfig {
            description: "Test step".to_string(),
            args: vec![],
            cli: Some(StepCliConfig {
                args: step_args.into_iter().map(|s| s.to_string()).collect(),
            }),
        }
    }

    // Criterion: global args + step args concatenated with tokens expanded
    #[test]
    fn assembles_correct_argv() {
        let global = make_global_cli(vec!["--model", "claude-opus-4-5"]);
        let step = make_step(vec!["--worktree", "plan-session", "/plan-skill"]);
        let vars = HashMap::new();

        let argv = assemble_argv(&global, &step, &vars);
        assert_eq!(
            argv,
            vec![
                "--model",
                "claude-opus-4-5",
                "--worktree",
                "plan-session",
                "/plan-skill"
            ]
        );
    }

    // Criterion: ${task} with task=foo in vars → foo in assembled argv
    #[test]
    fn tokens_expanded() {
        let global = make_global_cli(vec!["--model", "${model}"]);
        let step = make_step(vec!["${task}"]);
        let mut vars = HashMap::new();
        vars.insert("model".to_string(), "claude-opus-4-5".to_string());
        vars.insert("task".to_string(), "do_stuff".to_string());

        let argv = assemble_argv(&global, &step, &vars);
        assert_eq!(argv, vec!["--model", "claude-opus-4-5", "do_stuff"]);
    }

    // Criterion: check_command_available returns error for nonexistent binary
    #[test]
    fn command_not_found() {
        let result = check_command_available("nonexistent_binary_xyz_ywflow");
        assert!(
            matches!(result, Err(WorkflowError::CommandNotFound { ref command }) if command == "nonexistent_binary_xyz_ywflow"),
            "expected CommandNotFound, got {:?}",
            result
        );
    }

    // Criterion 3: CommandNotFound error message is exactly the specified string
    #[test]
    fn command_not_found_exact_message() {
        let err = WorkflowError::CommandNotFound {
            command: "claude".to_string(),
        };
        let expected = "CLI 'claude' not found\n  \u{2192} Install Claude Code: https://docs.anthropic.com/en/docs/claude-code";
        assert_eq!(
            err.to_string(),
            expected,
            "error message must match exactly"
        );
    }

    // Criterion: check_command_available returns Ok for a binary that exists
    #[test]
    fn command_found() {
        // `sh` is available on all POSIX systems.
        let result = check_command_available("sh");
        assert!(result.is_ok(), "expected sh to be found, got {:?}", result);
    }

    // Criterion: step with no step-level cli.args uses only global args
    #[test]
    fn no_step_args_uses_only_global() {
        let global = make_global_cli(vec!["--model", "claude-opus-4-5"]);
        let step = StepConfig {
            description: "No extra args".to_string(),
            args: vec![],
            cli: None,
        };
        let vars = HashMap::new();

        let argv = assemble_argv(&global, &step, &vars);
        assert_eq!(argv, vec!["--model", "claude-opus-4-5"]);
    }
}
