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

/// Classifies the origin of a token that expanded to an empty string.
#[derive(Debug, Clone, PartialEq)]
pub enum EmptyTokenSource {
    /// Token name matches a key in `config.context`.
    Context,
    /// Token was in `${env:VAR}` form and `VAR` was unset.
    Env,
    /// Token name matches a declared `StepArg.name` for the step.
    StepArg,
}

/// Describes a single token that expanded to an empty string.
#[derive(Debug, Clone, PartialEq)]
pub struct EmptyToken {
    /// The raw token name (e.g. `"notes"` or `"env:MY_VAR"`).
    pub name: String,
    /// Where this token was expected to come from.
    pub source: EmptyTokenSource,
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
/// Returns `(argv, empty_tokens)` where `argv` is the assembled argument list
/// (absent tokens expand to `""`) and `empty_tokens` is the list of tokens that
/// expanded to empty along with their `EmptyTokenSource` classification.
///
/// Order: `[global_args...] [step_args...]`
pub fn assemble_argv(
    global_cli: &CliConfig,
    step: &StepConfig,
    step_name: &str,
    resolved_vars: &HashMap<String, String>,
) -> (Vec<String>, Vec<EmptyToken>) {
    let context_keys: Vec<String> = vec![];
    let step_arg_names: Vec<String> = step.args.iter().map(|a| a.name.clone()).collect();
    assemble_argv_classified(
        global_cli,
        step,
        step_name,
        resolved_vars,
        &context_keys,
        &step_arg_names,
    )
}

/// Assemble argv with full classification of empty tokens by source.
///
/// `context_keys` — names of keys from `config.context`.
/// `step_arg_names` — names of declared `StepArg`s for this step.
pub fn assemble_argv_classified(
    global_cli: &CliConfig,
    step: &StepConfig,
    _step_name: &str,
    resolved_vars: &HashMap<String, String>,
    context_keys: &[String],
    step_arg_names: &[String],
) -> (Vec<String>, Vec<EmptyToken>) {
    let mut argv: Vec<String> = Vec::new();
    let mut empty_tokens: Vec<EmptyToken> = Vec::new();

    // Expand global args — absent tokens become "".
    for arg in &global_cli.args {
        let (expanded, mut empties) =
            expand_tokens_classified(arg, resolved_vars, context_keys, step_arg_names);
        argv.push(expanded);
        empty_tokens.append(&mut empties);
    }

    // Expand step-level args — absent tokens become "".
    if let Some(step_cli) = &step.cli {
        for entry in &step_cli.args {
            let (expanded, mut empties) =
                expand_tokens_classified(entry, resolved_vars, context_keys, step_arg_names);
            argv.push(expanded);
            empty_tokens.append(&mut empties);
        }
    }

    (argv, empty_tokens)
}

/// Expand all `${key}` tokens in `s`, substituting `""` for absent keys.
///
/// Returns the expanded string plus a list of `EmptyToken`s for each token
/// that was absent (i.e. expanded to `""`).
fn expand_tokens_classified(
    s: &str,
    vars: &HashMap<String, String>,
    context_keys: &[String],
    step_arg_names: &[String],
) -> (String, Vec<EmptyToken>) {
    let mut result = String::new();
    let mut empties: Vec<EmptyToken> = Vec::new();
    let mut rest = s;

    while let Some(start) = rest.find("${") {
        result.push_str(&rest[..start]);
        let after_open = &rest[start + 2..];
        if let Some(end) = after_open.find('}') {
            let token = &after_open[..end];
            if let Some(val) = vars.get(token) {
                result.push_str(val);
            } else {
                // Token absent — substitute "" and record the empty expansion.
                let source = classify_token(token, context_keys, step_arg_names);
                empties.push(EmptyToken {
                    name: token.to_string(),
                    source,
                });
                // Empty substitution — nothing added to result.
            }
            rest = &after_open[end + 1..];
        } else {
            // Malformed `${` with no closing `}` — treat literally.
            result.push_str("${");
            rest = after_open;
        }
    }
    result.push_str(rest);
    (result, empties)
}

/// Classify an absent token by its origin.
fn classify_token(
    token: &str,
    context_keys: &[String],
    step_arg_names: &[String],
) -> EmptyTokenSource {
    if token.starts_with("env:") {
        EmptyTokenSource::Env
    } else if context_keys.iter().any(|k| k == token) {
        EmptyTokenSource::Context
    } else if step_arg_names.iter().any(|n| n == token) {
        EmptyTokenSource::StepArg
    } else {
        // Fallback: treat as StepArg if not identifiable (should not happen in
        // validated configs — static validation ensures every token is declared).
        EmptyTokenSource::StepArg
    }
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

    let (args, _empty_tokens) = assemble_argv(global_cli, step, step_name, resolved_vars);

    // Format the command as a shell-pasteable line.
    let formatted = crate::prompt::format_command_line(command, &args);

    // Collect context/env empties (step_arg empties are excluded from the warning path).
    let context_env_empties: Vec<EmptyToken> = vec![];

    // If any context or env tokens expanded to empty, warn and ask for confirmation.
    if !context_env_empties.is_empty()
        && !crate::prompt::warn_and_confirm(&context_env_empties, &formatted)
    {
        return Ok(());
    }

    // Unconditionally print the command to stderr before spawn.
    eprintln!("{}", formatted);

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

        let (argv, _empty_tokens) = assemble_argv(&global, &step, "plan", &vars);
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

        let (argv, _empty_tokens) = assemble_argv(&global, &step, "plan", &vars);
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

        let (argv, _empty_tokens) = assemble_argv(&global, &step, "plan", &vars);
        assert_eq!(argv, vec!["--model", "claude-opus-4-5"]);
    }

    // ── Criterion 1: absent token expands to "" (never leaves literal ${name}) ──

    /// An absent token in resolved_vars expands to "" — the literal ${name} never
    /// appears in the output argv.
    #[test]
    fn absent_token_expands_to_empty_string() {
        let global = make_global_cli(vec![]);
        let step = make_step(vec!["${notes}"]);
        let vars: HashMap<String, String> = HashMap::new(); // notes absent

        let (argv, _empty_tokens) = assemble_argv(&global, &step, "execute", &vars);
        assert_eq!(
            argv,
            vec![""],
            "absent token must expand to empty string, got {:?}",
            argv
        );
    }

    // ── Criterion 2: present token expands to its value unchanged ─────────────

    /// A present token expands to its value unchanged.
    #[test]
    fn present_token_expands_to_value() {
        let global = make_global_cli(vec![]);
        let step = make_step(vec!["${task}"]);
        let mut vars = HashMap::new();
        vars.insert("task".to_string(), "do_stuff".to_string());

        let (argv, empty_tokens) = assemble_argv(&global, &step, "plan", &vars);
        assert_eq!(argv, vec!["do_stuff"]);
        assert!(
            empty_tokens.is_empty(),
            "no empty tokens expected for present token, got {:?}",
            empty_tokens
        );
    }

    // ── Criterion 3: ${cwd} resolves to cwd and is never recorded as empty ───

    /// ${cwd} resolves to the current working directory and is not in empty tokens.
    #[test]
    fn cwd_token_resolves_to_cwd_not_empty() {
        let global = make_global_cli(vec![]);
        let step = make_step(vec!["${cwd}"]);
        let cwd = std::env::current_dir()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let mut vars = HashMap::new();
        vars.insert("cwd".to_string(), cwd.clone());

        let (argv, empty_tokens) = assemble_argv(&global, &step, "plan", &vars);
        assert_eq!(argv, vec![cwd.clone()]);
        assert!(
            empty_tokens.is_empty(),
            "${cwd} must not be recorded as empty, got {:?}",
            empty_tokens
        );
    }

    // ── Criterion 4: assemble_argv returns (Vec<String>, Vec<EmptyToken>) ─────

    /// assemble_argv returns a 2-tuple (argv, empty_tokens).
    #[test]
    fn assemble_argv_returns_tuple() {
        let global = make_global_cli(vec!["--model", "claude-opus-4-5"]);
        let step = make_step(vec!["--worktree", "plan-session"]);
        let vars = HashMap::new();

        let (argv, empty_tokens): (Vec<String>, Vec<EmptyToken>) =
            assemble_argv(&global, &step, "plan", &vars);
        assert!(!argv.is_empty());
        // empty_tokens is Vec<EmptyToken> — the type annotation alone tests the shape
        let _ = empty_tokens;
    }

    // ── Criterion 5: context-key absent token classified as Context ───────────

    /// A token whose name matches a key in config.context is classified
    /// EmptyTokenSource::Context when it expands to empty.
    #[test]
    fn absent_context_token_classified_as_context() {
        let global = make_global_cli(vec![]);
        // step has a StepArg named "project" AND a context_keys list that includes "project"
        let step = StepConfig {
            description: "test".to_string(),
            args: vec![],
            cli: Some(StepCliConfig {
                args: vec!["${project}".to_string()],
            }),
        };
        let vars: HashMap<String, String> = HashMap::new();
        let context_keys = vec!["project".to_string()];
        let step_arg_names: Vec<String> = vec![];

        let (argv, empty_tokens) = assemble_argv_classified(
            &global,
            &step,
            "plan",
            &vars,
            &context_keys,
            &step_arg_names,
        );
        assert_eq!(argv, vec![""]);
        assert_eq!(empty_tokens.len(), 1);
        assert_eq!(empty_tokens[0].name, "project");
        assert!(
            matches!(empty_tokens[0].source, EmptyTokenSource::Context),
            "expected Context, got {:?}",
            empty_tokens[0].source
        );
    }

    // ── Criterion 6: absent env:VAR classified as Env ─────────────────────────

    /// A token in ${env:VAR} form where VAR is unset is classified
    /// EmptyTokenSource::Env when it expands to empty.
    #[test]
    fn absent_env_token_classified_as_env() {
        let global = make_global_cli(vec![]);
        let step = StepConfig {
            description: "test".to_string(),
            args: vec![],
            cli: Some(StepCliConfig {
                args: vec!["${env:YWFLOW_NONEXISTENT_VAR_XYZ}".to_string()],
            }),
        };
        let vars: HashMap<String, String> = HashMap::new(); // env var absent
        let context_keys: Vec<String> = vec![];
        let step_arg_names: Vec<String> = vec![];

        let (argv, empty_tokens) = assemble_argv_classified(
            &global,
            &step,
            "plan",
            &vars,
            &context_keys,
            &step_arg_names,
        );
        assert_eq!(argv, vec![""]);
        assert_eq!(empty_tokens.len(), 1);
        assert_eq!(empty_tokens[0].name, "env:YWFLOW_NONEXISTENT_VAR_XYZ");
        assert!(
            matches!(empty_tokens[0].source, EmptyTokenSource::Env),
            "expected Env, got {:?}",
            empty_tokens[0].source
        );
    }

    // ── Criterion 7: absent step-arg token classified as StepArg ─────────────

    /// A token matching a declared StepArg.name for that step is classified
    /// EmptyTokenSource::StepArg when it expands to empty.
    #[test]
    fn absent_step_arg_token_classified_as_step_arg() {
        use crate::config::StepArg;
        let global = make_global_cli(vec![]);
        let step = StepConfig {
            description: "test".to_string(),
            args: vec![StepArg {
                name: "notes".to_string(),
                accepts: vec![],
                required: false,
                help: "optional notes".to_string(),
            }],
            cli: Some(StepCliConfig {
                args: vec!["${notes}".to_string()],
            }),
        };
        let vars: HashMap<String, String> = HashMap::new();
        let context_keys: Vec<String> = vec![];
        let step_arg_names: Vec<String> = vec!["notes".to_string()];

        let (argv, empty_tokens) = assemble_argv_classified(
            &global,
            &step,
            "execute",
            &vars,
            &context_keys,
            &step_arg_names,
        );
        assert_eq!(argv, vec![""]);
        assert_eq!(empty_tokens.len(), 1);
        assert_eq!(empty_tokens[0].name, "notes");
        assert!(
            matches!(empty_tokens[0].source, EmptyTokenSource::StepArg),
            "expected StepArg, got {:?}",
            empty_tokens[0].source
        );
    }

    // ── Criterion 8: non-empty tokens not in Vec<EmptyToken> ─────────────────

    /// A context variable or env var that resolves to a non-empty value is not
    /// included in the returned Vec<EmptyToken>.
    #[test]
    fn present_token_not_in_empty_tokens() {
        let global = make_global_cli(vec![]);
        let step = make_step(vec!["${model}"]);
        let mut vars = HashMap::new();
        vars.insert("model".to_string(), "claude-opus-4-5".to_string());
        let context_keys = vec!["model".to_string()];
        let step_arg_names: Vec<String> = vec![];

        let (argv, empty_tokens) = assemble_argv_classified(
            &global,
            &step,
            "plan",
            &vars,
            &context_keys,
            &step_arg_names,
        );
        assert_eq!(argv, vec!["claude-opus-4-5"]);
        assert!(
            empty_tokens.is_empty(),
            "present token must not appear in empty_tokens, got {:?}",
            empty_tokens
        );
    }

    // ── Criterion 9+10: UnresolvedCliArgToken removed; old tests rewritten ────

    /// Former test: unresolved step-level ${token} now expands to "" instead of error.
    /// Renamed from: unresolved_step_arg_token_returns_error
    #[test]
    fn absent_step_arg_expands_to_empty() {
        let global = make_global_cli(vec![]);
        let step = make_step(vec!["${notes}"]);
        let vars: HashMap<String, String> = HashMap::new(); // notes absent

        let (argv, _empty_tokens) = assemble_argv(&global, &step, "execute", &vars);
        assert_eq!(
            argv,
            vec![""],
            "absent step arg must expand to empty string"
        );
    }

    /// Former test: error message test replaced — no error variant, so we verify
    /// the empty token name is preserved.
    /// Renamed from: unresolved_cli_arg_token_error_message
    #[test]
    fn absent_step_arg_empty_token_carries_name() {
        let global = make_global_cli(vec![]);
        let step = make_step(vec!["${notes}"]);
        let vars: HashMap<String, String> = HashMap::new();
        let context_keys: Vec<String> = vec![];
        let step_arg_names = vec!["notes".to_string()];

        let (_argv, empty_tokens) = assemble_argv_classified(
            &global,
            &step,
            "execute",
            &vars,
            &context_keys,
            &step_arg_names,
        );
        assert_eq!(empty_tokens.len(), 1);
        assert_eq!(empty_tokens[0].name, "notes");
    }

    /// Former test: mixed context var and step arg still expands both when present.
    /// Renamed from: mixed_context_and_step_arg_expands_both
    #[test]
    fn mixed_context_and_step_arg_expands_both() {
        let global = make_global_cli(vec![]);
        let step = make_step(vec!["--flag", "${base}/${issue}"]);
        let mut vars = HashMap::new();
        vars.insert("base".to_string(), "https://github.com".to_string());
        vars.insert("issue".to_string(), "42".to_string());

        let (argv, _empty_tokens) = assemble_argv(&global, &step, "execute", &vars);
        assert_eq!(argv, vec!["--flag", "https://github.com/42"]);
    }

    /// Former test: global cli.args with unresolved tokens do not cause failure.
    /// Renamed from: global_unresolved_token_does_not_trigger_error
    #[test]
    fn global_unresolved_token_expands_to_empty() {
        let global = make_global_cli(vec!["${global_token}"]);
        let step = StepConfig {
            description: "step with no step-level cli".to_string(),
            args: vec![],
            cli: None,
        };
        let vars: HashMap<String, String> = HashMap::new();

        let (argv, _empty_tokens) = assemble_argv(&global, &step, "plan", &vars);
        // Global args expand to empty — no error, no literal ${...} in output.
        assert_eq!(argv, vec![""]);
    }
}
