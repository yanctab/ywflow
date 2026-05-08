// tests/readme_config.rs
// Integration tests for the README.md Configuration section.
// Each test group corresponds to one acceptance criterion from issue #31.

use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn readme() -> String {
    let path = repo_root().join("README.md");
    std::fs::read_to_string(path).expect("README.md must exist")
}

// Helper: extract only the Configuration section from README.md
// (from "## Configuration" up to the next "##" heading or end-of-file)
fn config_section() -> String {
    let content = readme();
    let start = content
        .find("## Configuration")
        .expect("README.md must contain a '## Configuration' section");
    let rest = &content[start..];
    // Find next top-level "## " heading after the first line
    let after_heading = rest.find('\n').map(|i| i + 1).unwrap_or(rest.len());
    let end = rest[after_heading..]
        .find("\n## ")
        .map(|i| after_heading + i)
        .unwrap_or(rest.len());
    rest[..end].to_string()
}

// ── Criterion 1 ───────────────────────────────────────────────────────────────
// The inline YAML example that uses `steps`, `model`, or `skills` is removed.

#[test]
fn config_section_has_no_steps_key() {
    let section = config_section();
    assert!(
        !section.contains("steps:"),
        "Configuration section must not contain 'steps:' (stale YAML key)"
    );
}

#[test]
fn config_section_has_no_model_key() {
    let section = config_section();
    assert!(
        !section.contains("model:"),
        "Configuration section must not contain 'model:' (stale YAML key)"
    );
}

#[test]
fn config_section_has_no_skills_key() {
    let section = config_section();
    assert!(
        !section.contains("skills:"),
        "Configuration section must not contain 'skills:' (stale YAML key)"
    );
}

// ── Criterion 2 ───────────────────────────────────────────────────────────────
// A top-level key table covers all five keys: required_env, context, cli,
// plugins, workflow; the table notes cli.command as the only required field.

#[test]
fn config_section_table_has_required_env() {
    let section = config_section();
    assert!(
        section.contains("required_env"),
        "Configuration section must include 'required_env' in the top-level key table"
    );
}

#[test]
fn config_section_table_has_context() {
    let section = config_section();
    assert!(
        section.contains("context"),
        "Configuration section must include 'context' in the top-level key table"
    );
}

#[test]
fn config_section_table_has_cli() {
    let section = config_section();
    assert!(
        section.contains("cli"),
        "Configuration section must include 'cli' in the top-level key table"
    );
}

#[test]
fn config_section_table_has_plugins() {
    let section = config_section();
    assert!(
        section.contains("plugins"),
        "Configuration section must include 'plugins' in the top-level key table"
    );
}

#[test]
fn config_section_table_has_workflow() {
    let section = config_section();
    assert!(
        section.contains("workflow"),
        "Configuration section must include 'workflow' in the top-level key table"
    );
}

#[test]
fn config_section_table_notes_cli_command_required() {
    let section = config_section();
    assert!(
        section.contains("cli.command"),
        "Configuration section table must mention cli.command as the only required field"
    );
}

// ── Criterion 3 ───────────────────────────────────────────────────────────────
// The key table is followed by a relative link to docs/config.md.

#[test]
fn config_section_has_link_to_docs_config_md() {
    let section = config_section();
    assert!(
        section.contains("docs/config.md"),
        "Configuration section must include a relative link to docs/config.md"
    );
}

// ── Criterion 4 ───────────────────────────────────────────────────────────────
// A Plugins sub-section with two code block examples: one for source: marketplace
// (with name and package) and one for source: local (with name and path).

#[test]
fn config_section_has_plugins_subsection() {
    let section = config_section();
    assert!(
        section.contains("### Plugins") || section.contains("## Plugins"),
        "Configuration section must include a Plugins sub-section"
    );
}

#[test]
fn config_section_plugins_has_marketplace_example() {
    let section = config_section();
    assert!(
        section.contains("source: marketplace"),
        "Plugins sub-section must include a code block example with 'source: marketplace'"
    );
}

#[test]
fn config_section_plugins_marketplace_has_package_field() {
    let section = config_section();
    // The marketplace example must include a 'package:' field
    assert!(
        section.contains("package:"),
        "Plugins marketplace example must include a 'package:' field"
    );
}

#[test]
fn config_section_plugins_has_local_example() {
    let section = config_section();
    assert!(
        section.contains("source: local"),
        "Plugins sub-section must include a code block example with 'source: local'"
    );
}

#[test]
fn config_section_plugins_local_has_path_field() {
    let section = config_section();
    // The local example must include a 'path:' field
    assert!(
        section.contains("path:"),
        "Plugins local example must include a 'path:' field"
    );
}

// ── Criterion 5 ───────────────────────────────────────────────────────────────
// A callout or note states: reserved context keys input, cwd, date cannot be
// used in the context: block.

#[test]
fn config_section_has_reserved_keys_callout() {
    let section = config_section();
    // Must mention all three reserved keys
    assert!(
        section.contains("input") && section.contains("cwd") && section.contains("date"),
        "Configuration section must mention reserved context keys: input, cwd, date"
    );
}

#[test]
fn config_section_reserved_keys_cannot_be_used_in_context() {
    let section = config_section();
    // Must state that reserved keys cannot be used in context:
    let lower = section.to_lowercase();
    assert!(
        lower.contains("reserved") || lower.contains("cannot"),
        "Configuration section must state that reserved keys cannot be used in the context: block"
    );
}

// ── Criterion 6 ───────────────────────────────────────────────────────────────
// A callout or note states: within a step's args, required args must precede optional args.

#[test]
fn config_section_has_arg_order_callout() {
    let section = config_section();
    let lower = section.to_lowercase();
    // Must mention that required args precede optional args
    assert!(
        lower.contains("required") && lower.contains("optional"),
        "Configuration section must note that required args must precede optional args"
    );
    assert!(
        lower.contains("precede") || lower.contains("before") || lower.contains("preceding"),
        "Configuration section must state that required args must precede optional args"
    );
}

// ── Criterion 7 ───────────────────────────────────────────────────────────────
// A callout or note states: config discovery walks up from current working
// directory to filesystem root.

#[test]
fn config_section_has_discovery_callout() {
    let section = config_section();
    let lower = section.to_lowercase();
    // Must mention walking up to find config
    assert!(
        lower.contains("walk") || lower.contains("parent"),
        "Configuration section must mention that config discovery walks up from cwd"
    );
    assert!(
        lower.contains("root"),
        "Configuration section must mention that discovery walks up to filesystem root"
    );
}

// ── Criterion 8 ───────────────────────────────────────────────────────────────
// The existing variable expansion table is retained; the ${task} row is
// corrected to clarify it is an ordinary named step arg value.

#[test]
fn config_section_has_variable_expansion_table() {
    let section = config_section();
    assert!(
        section.contains("### Variable expansion") || section.contains("## Variable expansion"),
        "Configuration section must retain the Variable expansion sub-section"
    );
}

#[test]
fn config_section_variable_table_has_context_row() {
    let section = config_section();
    assert!(
        section.contains("${variable}") || section.contains("`${variable}`"),
        "Variable expansion table must include the '${{variable}}' row for context variables"
    );
}

#[test]
fn config_section_variable_table_has_env_row() {
    let section = config_section();
    assert!(
        section.contains("${env:VAR}") || section.contains("`${env:VAR}`"),
        "Variable expansion table must include the '${{env:VAR}}' row"
    );
}

#[test]
fn config_section_task_row_clarifies_step_arg() {
    let section = config_section();
    // The ${task} or equivalent row must clarify it is a named step arg value, not a special keyword
    assert!(
        section.contains("step arg")
            || section.contains("named arg")
            || section.contains("step's args"),
        "Variable expansion table must clarify that named step arg values (like ${{task}}) are ordinary step args, not special keywords"
    );
}

// ── Criterion 9 ───────────────────────────────────────────────────────────────
// No occurrences of `steps`, `--task`, `model`, or `skills` remain in the
// updated section.

#[test]
fn config_section_has_no_task_flag() {
    let section = config_section();
    assert!(
        !section.contains("--task"),
        "Configuration section must not contain '--task' flag reference"
    );
}

// Note: steps:, model:, skills: already tested in Criterion 1 above.

// ── Criterion 10 ──────────────────────────────────────────────────────────────
// Step-level cli.args behaviour is documented: appended after global cli.args,
// not a replacement.

#[test]
fn config_section_documents_step_cli_args_appended() {
    let section = config_section();
    assert!(
        section.contains("appended") || section.contains("append"),
        "Configuration section must document that step-level cli.args are appended after global cli.args"
    );
}

#[test]
fn config_section_documents_step_cli_args_not_replacement() {
    let section = config_section();
    let lower = section.to_lowercase();
    // Must convey it is not a replacement (e.g. "not a replacement", "not replace", "in addition")
    assert!(
        lower.contains("not a replacement")
            || lower.contains("not replace")
            || lower.contains("in addition")
            || lower.contains("after global"),
        "Configuration section must clarify that step-level cli.args are not a replacement for global cli.args"
    );
}
