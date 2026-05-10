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
    #[error("step '{step}': cli.args references '${{{token}}}' but '--{token}' was not provided")]
    UnresolvedCliArgToken { step: String, token: String },
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
/// Order: `[global_args...] [step_args...]`
///
/// Step-level args are scanned for remaining `${...}` tokens after expansion;
/// any that remain unresolved return `Err(WorkflowError::UnresolvedCliArgToken)`.
/// Global args are not scanned (they may contain context variables resolved later).
pub fn assemble_argv(
    global_cli: &CliConfig,
    step: &StepConfig,
    step_name: &str,
    resolved_vars: &HashMap<String, String>,
) -> Result<Vec<String>, WorkflowError> {
    // Expand global args without error checking (global args are out of scope).
    let mut argv: Vec<String> = global_cli
        .args
        .iter()
        .map(|arg| expand_tokens(arg, resolved_vars))
        .collect();

    // Expand step-level args and scan for unresolved tokens.
    if let Some(step_cli) = &step.cli {
        for entry in &step_cli.args {
            let expanded = expand_tokens(entry, resolved_vars);
            // Detect any remaining ${...} tokens in the expanded result.
            if let Some(token) = find_unresolved_token(&expanded) {
                return Err(WorkflowError::UnresolvedCliArgToken {
                    step: step_name.to_string(),
                    token,
                });
            }
            argv.push(expanded);
        }
    }

    Ok(argv)
}

/// Return the name of the first unresolved `${name}` token found in `s`, or `None`.
fn find_unresolved_token(s: &str) -> Option<String> {
    if let Some(start) = s.find("${") {
        let after_open = &s[start + 2..];
        if let Some(end) = after_open.find('}') {
            return Some(after_open[..end].to_string());
        }
    }
    None
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
    step_name: &str,
    resolved_vars: &HashMap<String, String>,
) -> Result<(), WorkflowError> {
    // Verify the CLI binary is available before trying to exec.
    let command = &global_cli.command;
    check_command_available(command)?;

    let args = assemble_argv(global_cli, step, step_name, resolved_vars)?;

    let status = std::process::Command::new(command)
        .args(args)
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

        let argv = assemble_argv(&global, &step, "plan", &vars).unwrap();
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

        let argv = assemble_argv(&global, &step, "plan", &vars).unwrap();
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

    // Criterion 5: check_command_available returns the resolved absolute path on success
    #[test]
    fn command_found_returns_resolved_path() {
        // `sh` is universally available; its resolved path must be an absolute file.
        let result = check_command_available("sh").expect("sh must be available");
        assert!(
            std::path::Path::new(&result).is_absolute(),
            "resolved path must be absolute, got: {result}"
        );
        assert!(
            std::path::Path::new(&result).is_file(),
            "resolved path must point to a file, got: {result}"
        );
    }

    // Criterion 2: run_step launches process with inherited stdin/stdout/stderr
    // and blocks until it exits. We use `sh` with `-c exit 0` so the child exits
    // cleanly — demonstrating the full interactive exec path succeeds.
    #[test]
    fn run_step_inherits_io_and_exits_cleanly() {
        let global = CliConfig {
            command: "sh".to_string(),
            args: vec!["-c".to_string(), "exit 0".to_string()],
        };
        let step = StepConfig {
            description: "test".to_string(),
            args: vec![],
            cli: None,
        };
        let vars = HashMap::new();
        let result = run_step(&global, &step, "test", &vars);
        assert!(
            result.is_ok(),
            "run_step must return Ok for a clean exit: {:?}",
            result
        );
    }

    // Criterion 4: each run_step invocation spawns a fresh child; no state carried between calls
    #[test]
    fn each_invocation_is_independent() {
        let global = CliConfig {
            command: "sh".to_string(),
            args: vec!["-c".to_string(), "exit 0".to_string()],
        };
        let step = StepConfig {
            description: "test".to_string(),
            args: vec![],
            cli: None,
        };
        let vars = HashMap::new();

        let first = run_step(&global, &step, "test", &vars);
        let second = run_step(&global, &step, "test", &vars);

        assert!(first.is_ok(), "first invocation must succeed: {:?}", first);
        assert!(
            second.is_ok(),
            "second invocation must succeed independently: {:?}",
            second
        );
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

        let argv = assemble_argv(&global, &step, "plan", &vars).unwrap();
        assert_eq!(argv, vec!["--model", "claude-opus-4-5"]);
    }

    // ── Slice 58: Runtime error for unresolved step-level cli.args tokens ────────

    /// Unresolved step-level ${token} → UnresolvedCliArgToken error.
    #[test]
    fn unresolved_step_arg_token_returns_error() {
        let global = make_global_cli(vec![]);
        let step = make_step(vec!["${notes}"]);
        let vars: HashMap<String, String> = HashMap::new(); // notes absent

        let result = assemble_argv(&global, &step, "execute", &vars);
        assert!(
            matches!(
                result,
                Err(WorkflowError::UnresolvedCliArgToken {
                    ref step,
                    ref token
                }) if step == "execute" && token == "notes"
            ),
            "expected UnresolvedCliArgToken for absent optional arg, got {:?}",
            result
        );
    }

    /// Error message for unresolved step-level token matches the exact format.
    #[test]
    fn unresolved_cli_arg_token_error_message() {
        let err = WorkflowError::UnresolvedCliArgToken {
            step: "execute".to_string(),
            token: "notes".to_string(),
        };
        let expected =
            "step 'execute': cli.args references '${notes}' but '--notes' was not provided";
        assert_eq!(err.to_string(), expected);
    }

    /// Mixed context var and step arg in same entry expands both when both are in vars.
    #[test]
    fn mixed_context_and_step_arg_expands_both() {
        let global = make_global_cli(vec![]);
        let step = make_step(vec!["--flag", "${base}/${issue}"]);
        let mut vars = HashMap::new();
        vars.insert("base".to_string(), "https://github.com".to_string());
        vars.insert("issue".to_string(), "42".to_string());

        let argv = assemble_argv(&global, &step, "execute", &vars).unwrap();
        assert_eq!(argv, vec!["--flag", "https://github.com/42"]);
    }

    /// Global cli.args with unresolved tokens do NOT trigger UnresolvedCliArgToken.
    #[test]
    fn global_unresolved_token_does_not_trigger_error() {
        let global = make_global_cli(vec!["${global_token}"]);
        let step = StepConfig {
            description: "step with no step-level cli".to_string(),
            args: vec![],
            cli: None, // no step-level cli.args
        };
        let vars: HashMap<String, String> = HashMap::new();

        // Should succeed (no error) — global args are not scanned for unresolved tokens.
        let result = assemble_argv(&global, &step, "plan", &vars);
        assert!(
            result.is_ok(),
            "global unresolved token must not trigger UnresolvedCliArgToken, got {:?}",
            result
        );
    }
}
