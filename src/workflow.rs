// src/workflow.rs
// Orchestrates workflow step execution; each step launches a full interactive
// Claude Code session and requires human review before advancing.

use crate::config::{CliConfig, StepConfig};
use indexmap::IndexMap;
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

/// Classification of why a token expanded to empty.
#[derive(Debug, Clone, PartialEq)]
pub enum EmptyTokenSource {
    /// The token name is a key in `config.context` but its value was empty or absent.
    Context,
    /// The token was `${env:VAR}` form and `VAR` was unset in the environment.
    Env,
    /// The token name matches a declared `StepArg.name` for this step but was not provided.
    StepArg,
}

/// Records a single token that expanded to an empty string, along with its source.
#[derive(Debug, Clone, PartialEq)]
pub struct EmptyToken {
    pub name: String,
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
/// Order: `[global_args...] [step_args...]`
///
/// Any token that cannot be resolved expands to `""`. The second element of the
/// returned tuple lists every token that expanded to empty, classified by its
/// source (Context, Env, or StepArg).
///
/// `context` is the `config.context` map used to classify empty tokens as
/// `EmptyTokenSource::Context`.
pub fn assemble_argv(
    global_cli: &CliConfig,
    step: &StepConfig,
    _step_name: &str,
    resolved_vars: &HashMap<String, String>,
    context: &IndexMap<String, String>,
) -> (Vec<String>, Vec<EmptyToken>) {
    let context_keys: std::collections::HashSet<&str> =
        context.keys().map(|k| k.as_str()).collect();
    let step_arg_names: Vec<&str> = step.args.iter().map(|a| a.name.as_str()).collect();

    let mut argv: Vec<String> = Vec::new();
    let mut empty_tokens: Vec<EmptyToken> = Vec::new();

    // Expand global args — collect empty tokens from global args too.
    for arg in &global_cli.args {
        let (expanded, mut empties) =
            expand_tokens_classified(arg, resolved_vars, &context_keys, &step_arg_names);
        argv.push(expanded);
        empty_tokens.append(&mut empties);
    }

    // Expand step-level args.
    if let Some(step_cli) = &step.cli {
        for entry in &step_cli.args {
            let (expanded, mut empties) =
                expand_tokens_classified(entry, resolved_vars, &context_keys, &step_arg_names);
            argv.push(expanded);
            empty_tokens.append(&mut empties);
        }
    }

    (argv, empty_tokens)
}

/// Substitute all `${key}` tokens in `s` using `vars`, substituting `""` for absent keys.
/// Returns the expanded string and a list of `EmptyToken` for every token that expanded to empty.
fn expand_tokens_classified(
    s: &str,
    vars: &HashMap<String, String>,
    context_keys: &std::collections::HashSet<&str>,
    step_arg_names: &[&str],
) -> (String, Vec<EmptyToken>) {
    let mut result = String::new();
    let mut empty_tokens: Vec<EmptyToken> = Vec::new();
    let mut rest = s;

    while let Some(start) = rest.find("${") {
        result.push_str(&rest[..start]);
        let after_open = &rest[start + 2..];
        if let Some(end) = after_open.find('}') {
            let token = &after_open[..end];
            match vars.get(token) {
                Some(val) if !val.is_empty() => {
                    result.push_str(val);
                }
                _ => {
                    // Token absent or empty — expand to "" and record the empty expansion.
                    let source = classify_token(token, context_keys, step_arg_names);
                    empty_tokens.push(EmptyToken {
                        name: token.to_string(),
                        source,
                    });
                    // Expand to empty string (push nothing).
                }
            }
            rest = &after_open[end + 1..];
        } else {
            // Unclosed `${` — pass through literally.
            result.push_str("${");
            rest = after_open;
        }
    }
    result.push_str(rest);
    (result, empty_tokens)
}

/// Classify a token by its source for `EmptyToken`.
fn classify_token(
    token: &str,
    context_keys: &std::collections::HashSet<&str>,
    step_arg_names: &[&str],
) -> EmptyTokenSource {
    if token.starts_with("env:") {
        EmptyTokenSource::Env
    } else if context_keys.contains(token) {
        EmptyTokenSource::Context
    } else if step_arg_names.contains(&token) {
        EmptyTokenSource::StepArg
    } else {
        // Default: treat as context if not otherwise classifiable.
        EmptyTokenSource::Context
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
    context: &IndexMap<String, String>,
) -> Result<(), WorkflowError> {
    // Verify the CLI binary is available before trying to exec.
    let command = &global_cli.command;
    check_command_available(command)?;

    let (args, empty_tokens) = assemble_argv(global_cli, step, step_name, resolved_vars, context);

    // Format the command as a shell-pasteable line.
    let formatted = crate::prompt::format_command_line(command, &args);

    // Collect context/env empties (step_arg empties are excluded from the warning path).
    let context_env_empties: Vec<EmptyToken> = empty_tokens
        .into_iter()
        .filter(|t| matches!(t.source, EmptyTokenSource::Context | EmptyTokenSource::Env))
        .collect();

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
    use indexmap::IndexMap;

    fn empty_context() -> IndexMap<String, String> {
        IndexMap::new()
    }

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

        let (argv, _) = assemble_argv(&global, &step, "plan", &vars, &empty_context());
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

        let (argv, _) = assemble_argv(&global, &step, "plan", &vars, &empty_context());
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
        let result = run_step(&global, &step, "test", &vars, &empty_context());
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

        let first = run_step(&global, &step, "test", &vars, &empty_context());
        let second = run_step(&global, &step, "test", &vars, &empty_context());

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

        let (argv, _) = assemble_argv(&global, &step, "plan", &vars, &empty_context());
        assert_eq!(argv, vec!["--model", "claude-opus-4-5"]);
    }

    // AC4: assemble_argv returns (Vec<String>, Vec<EmptyToken>)
    #[test]
    fn assemble_argv_returns_tuple_of_argv_and_empty_tokens() {
        let global = make_global_cli(vec!["--model", "claude-opus-4-5"]);
        let step = make_step(vec!["--flag"]);
        let vars = HashMap::new();

        let (argv, empty_tokens) = assemble_argv(&global, &step, "plan", &vars, &empty_context());
        assert_eq!(argv, vec!["--model", "claude-opus-4-5", "--flag"]);
        assert!(
            empty_tokens.is_empty(),
            "no empty tokens expected when no token placeholders used"
        );
    }

    // ── Slice 58: Migrated tests (formerly asserted UnresolvedCliArgToken) ───────

    /// AC1 + AC10: Absent step-level ${token} expands to "" — no literal in argv.
    #[test]
    fn unresolved_step_arg_token_returns_error() {
        let global = make_global_cli(vec![]);
        let step = make_step(vec!["${notes}"]);
        let vars: HashMap<String, String> = HashMap::new(); // notes absent

        let (argv, _) = assemble_argv(&global, &step, "execute", &vars, &empty_context());
        assert_eq!(
            argv,
            vec![""],
            "absent token must expand to empty string, not literal ${{notes}}"
        );
    }

    /// AC1 + AC10: Migrated — assemble_argv succeeds (no error) for absent tokens.
    #[test]
    fn unresolved_cli_arg_token_error_message() {
        let global = make_global_cli(vec![]);
        let step = make_step(vec!["${notes}"]);
        let vars: HashMap<String, String> = HashMap::new();

        // assemble_argv no longer returns an error for absent tokens.
        let (argv, _) = assemble_argv(&global, &step, "execute", &vars, &empty_context());
        assert_eq!(
            argv,
            vec![""],
            "absent token must produce empty string in argv"
        );
    }

    /// AC2 + AC10: Mixed context var and step arg in same entry expands both when both are in vars.
    #[test]
    fn mixed_context_and_step_arg_expands_both() {
        let global = make_global_cli(vec![]);
        let step = make_step(vec!["--flag", "${base}/${issue}"]);
        let mut vars = HashMap::new();
        vars.insert("base".to_string(), "https://github.com".to_string());
        vars.insert("issue".to_string(), "42".to_string());

        let (argv, _) = assemble_argv(&global, &step, "execute", &vars, &empty_context());
        assert_eq!(argv, vec!["--flag", "https://github.com/42"]);
    }

    /// AC1 + AC10: Global cli.args with absent tokens expand to "" (no error).
    #[test]
    fn global_unresolved_token_does_not_trigger_error() {
        let global = make_global_cli(vec!["${global_token}"]);
        let step = StepConfig {
            description: "step with no step-level cli".to_string(),
            args: vec![],
            cli: None,
        };
        let vars: HashMap<String, String> = HashMap::new();

        // assemble_argv succeeds and expands absent global token to "".
        let (argv, _) = assemble_argv(&global, &step, "plan", &vars, &empty_context());
        assert_eq!(
            argv,
            vec![""],
            "absent global token must expand to empty string"
        );
    }

    // AC5: A token whose name matches a config.context key is classified Context when empty.
    #[test]
    fn absent_context_key_classified_as_context_source() {
        let global = make_global_cli(vec![]);
        let step = make_step(vec!["${myproject}"]);
        let vars: HashMap<String, String> = HashMap::new(); // myproject absent
        let mut context: IndexMap<String, String> = IndexMap::new();
        context.insert("myproject".to_string(), "some_value".to_string());

        let (_, empty_tokens) = assemble_argv(&global, &step, "plan", &vars, &context);
        assert_eq!(empty_tokens.len(), 1);
        assert_eq!(empty_tokens[0].name, "myproject");
        assert_eq!(empty_tokens[0].source, EmptyTokenSource::Context);
    }

    // AC8: A context variable or env var that resolves to non-empty is NOT in Vec<EmptyToken>.
    #[test]
    fn resolved_non_empty_token_not_recorded_in_empty_tokens() {
        let global = make_global_cli(vec![]);
        let step = make_step(vec!["${myproject}", "${env:MY_RESOLVED_VAR}"]);
        let mut vars = HashMap::new();
        vars.insert("myproject".to_string(), "my_project_value".to_string());
        // env:MY_RESOLVED_VAR resolves to non-empty value
        vars.insert(
            "env:MY_RESOLVED_VAR".to_string(),
            "resolved_env_value".to_string(),
        );

        let (argv, empty_tokens) = assemble_argv(&global, &step, "plan", &vars, &empty_context());
        // Both tokens resolved to non-empty values.
        assert_eq!(
            argv,
            vec!["my_project_value", "resolved_env_value"],
            "both tokens must expand to their values"
        );
        assert!(
            empty_tokens.is_empty(),
            "no empty tokens expected when all tokens resolve to non-empty; got: {:?}",
            empty_tokens
        );
    }

    // AC7: A token whose name matches a declared StepArg.name is classified StepArg when empty.
    #[test]
    fn absent_step_arg_token_classified_as_step_arg_source() {
        use crate::config::StepArg;

        let global = make_global_cli(vec![]);
        // Build a step with a declared StepArg named "notes".
        let step = StepConfig {
            description: "Test step".to_string(),
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
        let vars: HashMap<String, String> = HashMap::new(); // notes absent

        let (_, empty_tokens) = assemble_argv(&global, &step, "execute", &vars, &empty_context());
        assert_eq!(empty_tokens.len(), 1);
        assert_eq!(empty_tokens[0].name, "notes");
        assert_eq!(empty_tokens[0].source, EmptyTokenSource::StepArg);
    }

    // AC6: A token in ${env:VAR} form where VAR is unset is classified EmptyTokenSource::Env.
    #[test]
    fn unset_env_var_token_classified_as_env_source() {
        // Ensure the env var is unset.
        unsafe {
            std::env::remove_var("YWFLOW_TEST_UNSET_ENV_VAR_XYZ");
        }
        let global = make_global_cli(vec![]);
        let step = make_step(vec!["${env:YWFLOW_TEST_UNSET_ENV_VAR_XYZ}"]);
        // resolved_vars does not contain env:YWFLOW_TEST_UNSET_ENV_VAR_XYZ
        let vars: HashMap<String, String> = HashMap::new();

        let (_, empty_tokens) = assemble_argv(&global, &step, "plan", &vars, &empty_context());
        assert_eq!(empty_tokens.len(), 1);
        assert_eq!(empty_tokens[0].name, "env:YWFLOW_TEST_UNSET_ENV_VAR_XYZ");
        assert_eq!(empty_tokens[0].source, EmptyTokenSource::Env);
    }

    // AC3: ${cwd} resolves to the current working directory and is never recorded as empty.
    #[test]
    fn cwd_token_resolves_to_working_directory_and_not_recorded_as_empty() {
        let cwd = std::env::current_dir()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let global = make_global_cli(vec![]);
        let step = make_step(vec!["--workdir", "${cwd}"]);
        let mut vars = HashMap::new();
        vars.insert("cwd".to_string(), cwd.clone());

        let (argv, empty_tokens) = assemble_argv(&global, &step, "plan", &vars, &empty_context());
        assert_eq!(
            argv,
            vec!["--workdir", &cwd],
            "${cwd} must expand to the cwd value"
        );
        assert!(
            empty_tokens.iter().all(|t| t.name != "cwd"),
            "${cwd} must not be recorded as an empty token"
        );
    }
}
