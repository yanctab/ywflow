// src/context.rs
// Builds and manages the variable-expansion context used across workflow steps.
// Expansion order: context variables, then runtime variables (${task}), then ${env:X}.

#![allow(dead_code)]

use std::collections::{HashMap, HashSet};
use thiserror::Error;

use crate::config::Config;

#[derive(Debug, Error, PartialEq)]
pub enum ContextError {
    #[error("circular reference involving variable '{0}'")]
    Circular(String),
    #[error("undefined variable(s) after expansion: {}", .0.join(", "))]
    Undefined(Vec<String>),
}

pub fn resolve(
    config: &Config,
    step_name: &str,
    step_args: &HashMap<String, String>,
) -> Result<HashMap<String, String>, ContextError> {
    // Pass 1: expand context variables against each other (cycle-safe).
    let mut ctx = pass1_context(&config.context)?;

    // Pass 2: inject runtime variables (cwd, date) and step args, overwriting context.
    pass2_runtime(&mut ctx, step_name, step_args, config);

    // Pass 3: resolve ${env:VAR} tokens from the process environment.
    pass3_env(&mut ctx);

    // Any unresolved ${...} tokens after all passes → Undefined error.
    let mut unresolved: Vec<String> = ctx
        .values()
        .flat_map(|v| collect_tokens(v))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    unresolved.sort();
    if !unresolved.is_empty() {
        return Err(ContextError::Undefined(unresolved));
    }

    Ok(ctx)
}

// ── Pass 1 ────────────────────────────────────────────────────────────────────

/// Expand all context key/values against each other using iterative substitution
/// with cycle detection via a "visiting" set.
fn pass1_context(
    context: &indexmap::IndexMap<String, String>,
) -> Result<HashMap<String, String>, ContextError> {
    // Build a mutable working map seeded from the raw context.
    let mut map: HashMap<String, String> = context
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    // We expand each key by repeatedly substituting tokens until stable.
    // Cycle detection: track keys currently being expanded on the call stack.
    let keys: Vec<String> = context.keys().cloned().collect();
    let mut resolved: HashSet<String> = HashSet::new();

    for key in &keys {
        if !resolved.contains(key) {
            let mut visiting: HashSet<String> = HashSet::new();
            expand_key(key, &mut map, &mut visiting, &mut resolved)?;
        }
    }

    Ok(map)
}

fn expand_key(
    key: &str,
    map: &mut HashMap<String, String>,
    visiting: &mut HashSet<String>,
    resolved: &mut HashSet<String>,
) -> Result<(), ContextError> {
    if resolved.contains(key) {
        return Ok(());
    }
    if visiting.contains(key) {
        return Err(ContextError::Circular(key.to_string()));
    }

    visiting.insert(key.to_string());

    let value = match map.get(key) {
        Some(v) => v.clone(),
        None => {
            visiting.remove(key);
            return Ok(());
        }
    };

    let tokens = collect_context_tokens(&value);
    for token in tokens {
        if map.contains_key(&token) {
            expand_key(&token, map, visiting, resolved)?;
        }
        // If the token is not a context key we leave it for later passes.
    }

    // Re-fetch the value now that dependencies are expanded.
    let value = map.get(key).cloned().unwrap_or_default();
    let expanded = substitute_context_tokens(&value, map);
    map.insert(key.to_string(), expanded);

    visiting.remove(key);
    resolved.insert(key.to_string());
    Ok(())
}

/// Collect the names inside `${name}` tokens (excluding `env:` prefix).
fn collect_context_tokens(s: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut rest = s;
    while let Some(start) = rest.find("${") {
        let after_open = &rest[start + 2..];
        if let Some(end) = after_open.find('}') {
            let token = &after_open[..end];
            if !token.starts_with("env:") {
                tokens.push(token.to_string());
            }
            rest = &after_open[end + 1..];
        } else {
            break;
        }
    }
    tokens
}

/// Replace `${key}` with `map[key]` where the key exists in map.
fn substitute_context_tokens(s: &str, map: &HashMap<String, String>) -> String {
    let mut result = String::new();
    let mut rest = s;
    while let Some(start) = rest.find("${") {
        result.push_str(&rest[..start]);
        let after_open = &rest[start + 2..];
        if let Some(end) = after_open.find('}') {
            let token = &after_open[..end];
            if !token.starts_with("env:")
                && let Some(val) = map.get(token)
            {
                result.push_str(val);
                rest = &after_open[end + 1..];
                continue;
            }
            // Leave unresolved tokens as-is for later passes.
            result.push_str("${");
            result.push_str(token);
            result.push('}');
            rest = &after_open[end + 1..];
        } else {
            result.push_str("${");
            rest = after_open;
        }
    }
    result.push_str(rest);
    result
}

// ── Pass 2 ────────────────────────────────────────────────────────────────────

fn pass2_runtime(
    ctx: &mut HashMap<String, String>,
    _step_name: &str,
    step_args: &HashMap<String, String>,
    _config: &Config,
) {
    // Inject cwd.
    let cwd = std::env::current_dir()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    ctx.insert("cwd".to_string(), cwd);

    // Inject date in ISO-8601 format (YYYY-MM-DD) using chrono.
    let date = chrono::Local::now().format("%Y-%m-%d").to_string();
    ctx.insert("date".to_string(), date);

    // Inject named step args, overwriting any same-named context variable.
    for (k, v) in step_args {
        ctx.insert(k.clone(), v.clone());
    }

    // Expand any remaining ${...} tokens in context values now that runtime
    // vars are available.
    let keys: Vec<String> = ctx.keys().cloned().collect();
    for key in keys {
        let val = ctx.get(&key).cloned().unwrap_or_default();
        let expanded = substitute_context_tokens(&val, ctx);
        ctx.insert(key, expanded);
    }
}

// ── Pass 3 ────────────────────────────────────────────────────────────────────

fn pass3_env(ctx: &mut HashMap<String, String>) {
    let keys: Vec<String> = ctx.keys().cloned().collect();
    for key in keys {
        let val = ctx.get(&key).cloned().unwrap_or_default();
        let expanded = substitute_env_tokens(&val);
        ctx.insert(key, expanded);
    }
}

fn substitute_env_tokens(s: &str) -> String {
    let mut result = String::new();
    let mut rest = s;
    while let Some(start) = rest.find("${") {
        result.push_str(&rest[..start]);
        let after_open = &rest[start + 2..];
        if let Some(end) = after_open.find('}') {
            let token = &after_open[..end];
            if let Some(var_name) = token.strip_prefix("env:") {
                match std::env::var(var_name) {
                    Ok(val) => {
                        result.push_str(&val);
                        rest = &after_open[end + 1..];
                        continue;
                    }
                    Err(_) => {
                        // Leave as-is; will become Undefined if still present.
                        result.push_str("${");
                        result.push_str(token);
                        result.push('}');
                    }
                }
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

// ── Utility ───────────────────────────────────────────────────────────────────

/// Collect all `${...}` token names (including env: prefix) from a value string.
fn collect_tokens(s: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut rest = s;
    while let Some(start) = rest.find("${") {
        let after_open = &rest[start + 2..];
        if let Some(end) = after_open.find('}') {
            tokens.push(after_open[..end].to_string());
            rest = &after_open[end + 1..];
        } else {
            break;
        }
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::parse_and_validate;

    fn make_config(context_yaml: &str, workflow_yaml: &str) -> Config {
        let yaml = format!(
            r#"
cli:
  command: claude
context:
{}
workflow:
{}
"#,
            context_yaml, workflow_yaml
        );
        parse_and_validate(&yaml).expect("valid config")
    }

    #[test]
    fn context_var_in_step_arg() {
        // A context var referenced inside another context var resolves correctly.
        let config = make_config(
            "  base: hello\n  greeting: ${base}_world",
            r#"  plan:
    description: "Plan"
    args:
      - name: task
        required: true
        help: "Task""#,
        );
        let step_args: HashMap<String, String> =
            [("task".to_string(), "do_stuff".to_string())].into();
        let result = resolve(&config, "plan", &step_args).unwrap();
        assert_eq!(result.get("greeting"), Some(&"hello_world".to_string()));
    }

    #[test]
    fn circular_reference() {
        // Circular references between context vars produce ContextError::Circular.
        let yaml = r#"
cli:
  command: claude
context:
  a: ${b}
  b: ${a}
workflow:
  plan:
    description: "Plan"
"#;
        let config = parse_and_validate(yaml).unwrap();
        let step_args: HashMap<String, String> = HashMap::new();
        let result = resolve(&config, "plan", &step_args);
        assert!(
            matches!(result, Err(ContextError::Circular(_))),
            "expected Circular, got {:?}",
            result
        );
    }

    #[test]
    fn runtime_cwd() {
        // Resolved map contains key `cwd` equal to the process working directory.
        let config = make_config("  project: myproject", "  plan:\n    description: \"Plan\"");
        let step_args: HashMap<String, String> = HashMap::new();
        let result = resolve(&config, "plan", &step_args).unwrap();
        let expected_cwd = std::env::current_dir()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        assert_eq!(result.get("cwd"), Some(&expected_cwd));
    }

    #[test]
    fn runtime_date() {
        // Resolved map contains key `date` in ISO-8601 format (YYYY-MM-DD).
        let config = make_config("  project: myproject", "  plan:\n    description: \"Plan\"");
        let step_args: HashMap<String, String> = HashMap::new();
        let result = resolve(&config, "plan", &step_args).unwrap();
        let date = result.get("date").expect("date key present");
        // Must match YYYY-MM-DD
        assert!(
            date.len() == 10
                && date.chars().nth(4) == Some('-')
                && date.chars().nth(7) == Some('-'),
            "date not in ISO-8601 format: {date}"
        );
    }

    #[test]
    fn named_step_arg() {
        // A step arg value passed in appears in the resolved map under its name.
        let config = make_config(
            "  project: myproject",
            r#"  plan:
    description: "Plan"
    args:
      - name: task
        required: true
        help: "Task""#,
        );
        let step_args: HashMap<String, String> =
            [("task".to_string(), "implement_feature".to_string())].into();
        let result = resolve(&config, "plan", &step_args).unwrap();
        assert_eq!(result.get("task"), Some(&"implement_feature".to_string()));
    }

    #[test]
    fn optional_missing() {
        // An optional step arg not supplied does not produce an error and is absent.
        let config = make_config(
            "  project: myproject",
            r#"  plan:
    description: "Plan"
    args:
      - name: task
        required: true
        help: "Task"
      - name: url
        required: false
        help: "Optional URL""#,
        );
        // Supply only the required arg, not the optional one.
        let step_args: HashMap<String, String> =
            [("task".to_string(), "do_stuff".to_string())].into();
        let result = resolve(&config, "plan", &step_args).unwrap();
        assert!(
            !result.contains_key("url"),
            "optional absent arg should not be in map"
        );
    }
}
