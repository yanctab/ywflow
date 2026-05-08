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
use config::ConfigError;

fn format_error(err: &anyhow::Error) -> String {
    // Walk the error chain; the first level is the actionable message.
    let msg = err.to_string();
    if msg.contains('\n') {
        // Error already contains hint lines (e.g. WorkflowError::CommandNotFound).
        format!("error: {msg}")
    } else {
        format!("error: {msg}")
    }
}

/// Steps 1 and 2 of the startup sequence:
/// 1. Load config from `start` dir (NotFound → Ok(None), other errors → Err).
/// 2. If config present: check required_env entries.
///
/// Returns `Ok(Some(config))` when config exists and all required env vars are set,
/// `Ok(None)` when no config file is found, or `Err(...)` for any fatal condition.
fn startup_init(start: &std::path::Path) -> Result<Option<config::Config>> {
    let config = match config::load_from(start) {
        Ok(c) => Some(c),
        Err(ConfigError::NotFound) => None,
        Err(e) => anyhow::bail!(e),
    };
    if let Some(cfg) = &config {
        check_required_env(&cfg.required_env)?;
    }
    Ok(config)
}

fn check_required_env(required: &[String]) -> Result<()> {
    for var in required {
        if std::env::var(var).is_err() {
            let msg = format!(
                "required environment variable '{var}' is not set\n  → Add to your shell profile: export {var}=your_key"
            );
            anyhow::bail!(msg);
        }
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{}", format_error(&e));
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    // Steps 1+2: load config and check required env vars (before CLI parsing).
    let config = startup_init(&cwd)?;

    // Step 3: build and parse the CLI.
    let cmd = cli::build_command(config.as_ref());
    let matches = cmd.get_matches();

    // Step 4: dispatch.
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
                        anyhow::bail!("npx exited with status {}", status)
                    }
                })
            } else {
                vec![]
            };

            let missing_skills = cfg.map(plugins::verify_skill_files).unwrap_or_default();

            // Check if the CLI command is available.
            let claude_version = cfg.and_then(|c| {
                workflow::check_command_available(&c.cli.command)
                    .ok()
                    .and_then(|_| {
                        std::process::Command::new(&c.cli.command)
                            .arg("--version")
                            .output()
                            .ok()
                    })
                    .and_then(|out| String::from_utf8(out.stdout).ok())
                    .map(|s| s.trim().to_string())
            });

            plugins::print_ready_summary(
                cfg.is_some(),
                true,
                claude_version.as_deref(),
                &plugin_statuses,
                &missing_skills,
            );
        }
        Some((name, step_matches)) => {
            let cfg = config
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("no ywflow.yaml found"))?;

            let step = cfg
                .workflow
                .get(name)
                .ok_or_else(|| anyhow::anyhow!("unknown step: {name}"))?;

            // Collect argument values from clap matches.
            let mut raw_args: std::collections::HashMap<String, String> =
                std::collections::HashMap::new();
            for arg in &step.args {
                if let Some(val) = step_matches.get_one::<String>(&arg.name) {
                    // Validate the argument type.
                    input::validate(&arg.name, val, &arg.accepts, &input::http_head_check)
                        .map_err(|e| anyhow::anyhow!("{e}"))?;
                    raw_args.insert(arg.name.clone(), val.clone());
                } else if !arg.required {
                    // Optional arg absent → empty string so ${name} expands cleanly.
                    raw_args.insert(arg.name.clone(), String::new());
                }
            }

            // Resolve the full variable map.
            let resolved =
                context::resolve(cfg, name, &raw_args).map_err(|e| anyhow::anyhow!("{e}"))?;

            // Launch the step.
            workflow::run_step(&cfg.cli, step, &resolved).map_err(|e| anyhow::anyhow!("{e}"))?;
        }
        None => {
            cli::build_command(config.as_ref()).print_help()?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_error_has_lowercase_prefix() {
        let err = anyhow::anyhow!("something went wrong");
        let msg = format_error(&err);
        assert!(
            msg.starts_with("error:"),
            "expected 'error:' prefix, got: {msg}"
        );
    }

    #[test]
    fn missing_required_env_message() {
        // Use a var name guaranteed not to be set in the test environment.
        let var = "YWFLOW_TEST_MISSING_VAR_XYZ";
        unsafe {
            std::env::remove_var(var);
        }
        let result = check_required_env(&[var.to_string()]);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains(var),
            "error message should name the missing var: {msg}"
        );
        assert!(
            msg.contains("→"),
            "error message should include a hint: {msg}"
        );
    }

    #[test]
    fn present_required_env_ok() {
        let var = "PATH"; // Always set.
        let result = check_required_env(&[var.to_string()]);
        assert!(result.is_ok());
    }

    /// Criterion 1: exact error message (with "error:" prefix) for missing required env var.
    #[test]
    fn missing_required_env_exact_message() {
        let var = "ANTHROPIC_API_KEY";
        unsafe {
            std::env::remove_var(var);
        }
        let result = check_required_env(&[var.to_string()]);
        assert!(result.is_err());
        let formatted = format_error(&result.unwrap_err());
        let expected = "error: required environment variable 'ANTHROPIC_API_KEY' is not set\n  → Add to your shell profile: export ANTHROPIC_API_KEY=your_key";
        assert_eq!(
            formatted, expected,
            "formatted error message did not match expected.\nGot:      {formatted:?}\nExpected: {expected:?}"
        );
    }

    /// Test case 4: format_error with WorkflowError::CommandNotFound produces exact PRD string.
    #[test]
    fn error_format_with_hint() {
        let err: anyhow::Error = workflow::WorkflowError::CommandNotFound {
            command: "claude".to_string(),
        }
        .into();
        let formatted = format_error(&err);
        let expected = "error: CLI 'claude' not found\n  → Install Claude Code: https://docs.anthropic.com/en/docs/claude-code";
        assert_eq!(
            formatted, expected,
            "formatted error did not match PRD string.\nGot:      {formatted:?}\nExpected: {expected:?}"
        );
    }

    /// Test case 2: no ywflow.yaml present → startup continues without error (non-fatal).
    #[test]
    fn config_not_found_nonfatal() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        // No ywflow.yaml in tmp — startup_init should return Ok(None).
        let result = startup_init(tmp.path());
        assert!(
            result.is_ok(),
            "config not found should be non-fatal, got: {:?}",
            result
        );
        assert!(
            result.unwrap().is_none(),
            "config not found should return None"
        );
    }

    /// Test case 3: malformed YAML present → startup_init returns Err, and format_error
    /// produces a message with the lowercase "error:" prefix.
    #[test]
    fn other_config_error_fatal() {
        use std::fs;
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        // Write an intentionally malformed ywflow.yaml.
        fs::write(tmp.path().join("ywflow.yaml"), b": : invalid yaml {{{{").unwrap();

        let result = startup_init(tmp.path());
        assert!(
            result.is_err(),
            "malformed YAML should be a fatal error, got Ok"
        );
        let formatted = format_error(&result.unwrap_err());
        assert!(
            formatted.starts_with("error:"),
            "fatal config error must produce a lowercase 'error:' prefix, got: {formatted:?}"
        );
    }

    /// Criterion 4: clap "derive" feature is absent from Cargo.toml; binary compiles.
    #[test]
    fn clap_derive_feature_absent() {
        let cargo_toml = include_str!("../Cargo.toml");
        // Locate the clap dependency line and assert "derive" is not in its features.
        let clap_line = cargo_toml
            .lines()
            .find(|l| l.contains("clap"))
            .expect("Cargo.toml must contain a clap dependency");
        assert!(
            !clap_line.contains("derive"),
            "clap must not have the 'derive' feature; found: {clap_line:?}"
        );
    }
}
