// src/config.rs
// Deserialises ywflow.yaml into typed structs; exposes the top-level Config type.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PluginSource {
    Marketplace,
    Local,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Plugin {
    pub name: String,
    pub source: PluginSource,
    #[serde(default)]
    pub package: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CliConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AcceptsType {
    File,
    Url,
    String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StepArg {
    pub name: String,
    #[serde(default)]
    pub accepts: Vec<AcceptsType>,
    pub required: bool,
    pub help: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StepCliConfig {
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StepConfig {
    pub description: String,
    #[serde(default)]
    pub args: Vec<StepArg>,
    #[serde(default)]
    pub cli: Option<StepCliConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    #[serde(default)]
    pub required_env: Vec<String>,
    #[serde(default)]
    pub context: IndexMap<String, String>,
    pub cli: CliConfig,
    #[serde(default)]
    pub plugins: Vec<Plugin>,
    pub workflow: IndexMap<String, StepConfig>,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("no ywflow.yaml found in current directory or any parent directory")]
    NotFound,
    #[error("parse error: {0}")]
    Parse(#[from] serde_yaml::Error),
    #[error(
        "arg order violation in step '{step}': optional arg '{arg}' appears before required arg '{after}'"
    )]
    ArgOrder {
        step: String,
        arg: String,
        after: String,
    },
    #[error("schema error: {0}")]
    Schema(String),
}

const RESERVED_CONTEXT_KEYS: &[&str] = &["input", "cwd", "date"];

pub fn load() -> Result<Config, ConfigError> {
    let start = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    load_from(&start)
}

pub fn load_from(start: &std::path::Path) -> Result<Config, ConfigError> {
    let yaml_path = walk_up(start).ok_or(ConfigError::NotFound)?;
    let content = std::fs::read_to_string(&yaml_path).map_err(|_| ConfigError::NotFound)?;
    parse_and_validate(&content)
}

fn walk_up(start: &std::path::Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        let candidate = current.join("ywflow.yaml");
        if candidate.exists() {
            return Some(candidate);
        }
        if !current.pop() {
            return None;
        }
    }
}

pub fn parse_and_validate(yaml: &str) -> Result<Config, ConfigError> {
    let config: Config = serde_yaml::from_str(yaml)?;
    validate(&config)?;
    Ok(config)
}

fn validate(config: &Config) -> Result<(), ConfigError> {
    // Check reserved context keys
    for key in config.context.keys() {
        if RESERVED_CONTEXT_KEYS.contains(&key.as_str()) {
            return Err(ConfigError::Schema(format!(
                "context key '{}' is reserved",
                key
            )));
        }
    }

    // Check arg order in each workflow step
    for (step_name, step) in &config.workflow {
        let mut last_optional: Option<&str> = None;
        for arg in &step.args {
            if arg.required {
                if let Some(opt_name) = last_optional {
                    return Err(ConfigError::ArgOrder {
                        step: step_name.clone(),
                        arg: opt_name.to_string(),
                        after: arg.name.clone(),
                    });
                }
            } else {
                last_optional = Some(&arg.name);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn valid_yaml() -> &'static str {
        r#"
required_env:
  - GITHUB_TOKEN
context:
  project: myproject
cli:
  command: claude
  args:
    - --model
    - claude-opus-4-5
plugins:
  - name: my-plugin
    source: marketplace
    package: my-org/my-plugin
workflow:
  plan:
    description: "Plan the work"
    args:
      - name: task
        required: true
        help: "The task to plan"
      - name: url
        accepts:
          - url
        required: false
        help: "Optional reference URL"
    cli:
      args:
        - --extra-flag
"#
    }

    #[test]
    fn walk_parent_dirs() {
        let parent = TempDir::new().unwrap();
        let child = parent.path().join("subdir").join("deep");
        fs::create_dir_all(&child).unwrap();

        let yaml_path = parent.path().join("ywflow.yaml");
        fs::write(&yaml_path, valid_yaml()).unwrap();

        // Use load_from to exercise the full walk-up path through the public interface
        let result = load_from(&child);
        assert!(
            result.is_ok(),
            "should find ywflow.yaml in parent dir: {:?}",
            result
        );
    }

    #[test]
    fn valid_config() {
        let config = parse_and_validate(valid_yaml()).unwrap();
        assert_eq!(config.required_env, vec!["GITHUB_TOKEN"]);
        assert_eq!(
            config.context.get("project"),
            Some(&"myproject".to_string())
        );
        assert_eq!(config.cli.command, "claude");
        assert_eq!(config.cli.args, vec!["--model", "claude-opus-4-5"]);
        assert_eq!(config.plugins.len(), 1);
        assert_eq!(config.plugins[0].name, "my-plugin");
        assert!(config.workflow.contains_key("plan"));
        let plan = &config.workflow["plan"];
        assert_eq!(plan.description, "Plan the work");
        assert_eq!(plan.args.len(), 2);
        assert!(plan.args[0].required);
        assert!(!plan.args[1].required);
        assert_eq!(plan.cli.as_ref().unwrap().args, vec!["--extra-flag"]);
    }

    #[test]
    fn missing_required_field() {
        let yaml_without_cli = r#"
required_env: []
context: {}
plugins: []
workflow:
  plan:
    description: "Plan"
"#;
        let result = parse_and_validate(yaml_without_cli);
        assert!(
            matches!(result, Err(ConfigError::Parse(_))),
            "expected ConfigError::Parse, got {:?}",
            result
        );
    }

    #[test]
    fn arg_order_violation() {
        let yaml = r#"
cli:
  command: claude
workflow:
  plan:
    description: "Plan"
    args:
      - name: optional_arg
        required: false
        help: "An optional arg"
      - name: required_arg
        required: true
        help: "A required arg that comes after optional"
"#;
        let result = parse_and_validate(yaml);
        assert!(
            matches!(
                result,
                Err(ConfigError::ArgOrder {
                    ref step,
                    ref arg,
                    ref after
                }) if step == "plan" && arg == "optional_arg" && after == "required_arg"
            ),
            "expected ConfigError::ArgOrder, got {:?}",
            result
        );
    }

    #[test]
    fn not_found() {
        // Build an isolated directory tree with no ywflow.yaml anywhere.
        // We keep the tree entirely inside a TempDir so no ancestor up to /tmp
        // or / can interfere (those paths are guaranteed to have no ywflow.yaml
        // in a normal CI/dev environment).
        let tmp = TempDir::new().unwrap();
        let deep = tmp.path().join("a").join("b").join("c");
        fs::create_dir_all(&deep).unwrap();

        let result = load_from(&deep);
        assert!(
            matches!(result, Err(ConfigError::NotFound)),
            "expected ConfigError::NotFound, got {:?}",
            result
        );
        let err = ConfigError::NotFound;
        assert_eq!(
            err.to_string(),
            "no ywflow.yaml found in current directory or any parent directory"
        );
    }

    #[test]
    fn reserved_context_key() {
        let yaml = r#"
cli:
  command: claude
context:
  cwd: /some/path
workflow:
  plan:
    description: "Plan"
"#;
        let result = parse_and_validate(yaml);
        assert!(
            matches!(result, Err(ConfigError::Schema(_))),
            "expected ConfigError::Schema, got {:?}",
            result
        );
    }

    #[test]
    fn accepts_type_string_deserialises_from_yaml_token() {
        let yaml = r#"
cli:
  command: claude
workflow:
  step:
    description: "A step"
    args:
      - name: task
        accepts:
          - string
        required: true
        help: "The task"
"#;
        let config = parse_and_validate(yaml).unwrap();
        let step = &config.workflow["step"];
        assert_eq!(step.args[0].accepts, vec![AcceptsType::String]);
    }

    #[test]
    fn repo_ywflow_yaml_at_repo_root() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let yaml_path = manifest_dir.join("ywflow.yaml");
        assert!(
            yaml_path.exists(),
            "ywflow.yaml must exist at the repository root ({}), not under docs/ or src/",
            yaml_path.display()
        );
        // Confirm it is NOT nested under docs/ or src/
        let under_docs = manifest_dir.join("docs").join("ywflow.yaml").exists();
        let under_src = manifest_dir.join("src").join("ywflow.yaml").exists();
        assert!(!under_docs, "ywflow.yaml must not be under docs/");
        assert!(!under_src, "ywflow.yaml must not be under src/");
    }

    #[test]
    fn repo_ywflow_yaml_workflow_step_shapes() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let yaml_path = manifest_dir.join("ywflow.yaml");
        if !yaml_path.exists() {
            return;
        }
        let content = fs::read_to_string(&yaml_path).expect("read ywflow.yaml");
        let config = parse_and_validate(&content).unwrap();

        // plan: exactly one required arg accepting string
        let plan = config.workflow.get("plan").expect("plan step must exist");
        assert_eq!(
            plan.args.len(),
            1,
            "plan step must have exactly one arg, found: {:?}",
            plan.args
        );
        let plan_arg = &plan.args[0];
        assert!(plan_arg.required, "plan arg must be required");
        assert!(
            plan_arg.accepts.contains(&AcceptsType::String),
            "plan arg must accept string, found: {:?}",
            plan_arg.accepts
        );

        // breakdown: exactly one required arg accepting file or url
        let breakdown = config
            .workflow
            .get("breakdown")
            .expect("breakdown step must exist");
        assert_eq!(
            breakdown.args.len(),
            1,
            "breakdown must have exactly one arg"
        );
        let bd_arg = &breakdown.args[0];
        assert!(bd_arg.required, "breakdown arg must be required");
        let accepts_file = bd_arg.accepts.contains(&AcceptsType::File);
        let accepts_url = bd_arg.accepts.contains(&AcceptsType::Url);
        assert!(
            accepts_file || accepts_url,
            "breakdown arg must accept file or url"
        );

        // execute: one required arg followed by one optional arg
        let execute = config
            .workflow
            .get("execute")
            .expect("execute step must exist");
        assert_eq!(execute.args.len(), 2, "execute must have exactly two args");
        assert!(
            execute.args[0].required,
            "execute first arg must be required"
        );
        assert!(
            !execute.args[1].required,
            "execute second arg must be optional"
        );
    }

    #[test]
    fn repo_ywflow_yaml_plugins_has_marketplace_and_local() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let yaml_path = manifest_dir.join("ywflow.yaml");
        if !yaml_path.exists() {
            return;
        }
        let content = fs::read_to_string(&yaml_path).expect("read ywflow.yaml");
        let config = parse_and_validate(&content).unwrap();
        let has_marketplace = config
            .plugins
            .iter()
            .any(|p| p.source == PluginSource::Marketplace);
        let has_local = config
            .plugins
            .iter()
            .any(|p| p.source == PluginSource::Local);
        assert!(
            has_marketplace,
            "ywflow.yaml plugins must include at least one marketplace plugin"
        );
        assert!(
            has_local,
            "ywflow.yaml plugins must include at least one local plugin"
        );
    }

    #[test]
    fn repo_ywflow_yaml_cli_has_command_and_args() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let yaml_path = manifest_dir.join("ywflow.yaml");
        if !yaml_path.exists() {
            return;
        }
        let content = fs::read_to_string(&yaml_path).expect("read ywflow.yaml");
        let config = parse_and_validate(&content).unwrap();
        assert!(
            !config.cli.command.is_empty(),
            "ywflow.yaml cli must specify a command"
        );
        assert!(
            !config.cli.args.is_empty(),
            "ywflow.yaml cli must specify at least one global arg"
        );
    }

    #[test]
    fn repo_ywflow_yaml_context_has_env_expansion() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let yaml_path = manifest_dir.join("ywflow.yaml");
        if !yaml_path.exists() {
            return;
        }
        let content = fs::read_to_string(&yaml_path).expect("read ywflow.yaml");
        let config = parse_and_validate(&content).unwrap();
        let has_env_expansion = config.context.values().any(|v| v.contains("${env:"));
        assert!(
            has_env_expansion,
            "ywflow.yaml context must have at least one value using ${{env:...}} expansion"
        );
    }

    #[test]
    fn repo_ywflow_yaml_parses() {
        // Locate the ywflow.yaml committed to the repo root (two levels up from src/).
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let yaml_path = manifest_dir.join("ywflow.yaml");
        if !yaml_path.exists() {
            // Acceptable when running outside the repo tree.
            return;
        }
        let content = fs::read_to_string(&yaml_path).expect("read ywflow.yaml");
        let result = parse_and_validate(&content);
        assert!(
            result.is_ok(),
            "ywflow.yaml at repo root must parse without errors: {:?}",
            result
        );
        let config = result.unwrap();
        assert!(
            !config.required_env.is_empty(),
            "ywflow.yaml must have at least one required_env entry"
        );
        assert!(
            config.context.len() >= 2,
            "ywflow.yaml must have at least two context variables"
        );
        assert!(
            config.plugins.len() >= 2,
            "ywflow.yaml must have at least two plugins"
        );
        assert!(
            config.workflow.contains_key("plan"),
            "ywflow.yaml must define a 'plan' step"
        );
        assert!(
            config.workflow.contains_key("breakdown"),
            "ywflow.yaml must define a 'breakdown' step"
        );
        assert!(
            config.workflow.contains_key("execute"),
            "ywflow.yaml must define an 'execute' step"
        );
    }
}
