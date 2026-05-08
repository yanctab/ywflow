// tests/docs_examples.rs
// Integration tests verifying the docs/examples/ documentation files exist
// and contain the required content per issue #29 acceptance criteria.

use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

// ── Criterion 1 ──────────────────────────────────────────────────────────────
// docs/examples/minimal.md exists and contains the simplest valid ywflow.yaml:
// only cli.command set, one context variable, no plugins, one step with no args;
// accompanied by a brief prose explanation of each field shown.

#[test]
fn minimal_md_exists() {
    let path = repo_root().join("docs/examples/minimal.md");
    assert!(
        path.exists(),
        "docs/examples/minimal.md must exist at {:?}",
        path
    );
}

#[test]
fn minimal_md_contains_cli_command() {
    let content = std::fs::read_to_string(repo_root().join("docs/examples/minimal.md"))
        .expect("docs/examples/minimal.md must be readable");
    assert!(
        content.contains("command: claude"),
        "minimal.md must include 'command: claude' in the YAML example"
    );
}

#[test]
fn minimal_md_has_one_context_variable() {
    let content = std::fs::read_to_string(repo_root().join("docs/examples/minimal.md"))
        .expect("docs/examples/minimal.md must be readable");
    assert!(
        content.contains("context:"),
        "minimal.md must include a 'context:' section with one variable"
    );
}

#[test]
fn minimal_md_has_no_plugins_section() {
    let content = std::fs::read_to_string(repo_root().join("docs/examples/minimal.md"))
        .expect("docs/examples/minimal.md must be readable");
    assert!(
        !content.contains("plugins:"),
        "minimal.md must not include a 'plugins:' section (simplest valid config has no plugins)"
    );
}

#[test]
fn minimal_md_has_one_step_with_no_args() {
    let content = std::fs::read_to_string(repo_root().join("docs/examples/minimal.md"))
        .expect("docs/examples/minimal.md must be readable");
    assert!(
        content.contains("workflow:"),
        "minimal.md must include a 'workflow:' section with one step"
    );
    assert!(
        !content.contains("args:"),
        "minimal.md must not include 'args:' — the step must have no args"
    );
}

#[test]
fn minimal_md_has_prose_explanation() {
    let content = std::fs::read_to_string(repo_root().join("docs/examples/minimal.md"))
        .expect("docs/examples/minimal.md must be readable");
    // Must have at least one paragraph of prose (not just YAML)
    let prose_lines: Vec<&str> = content
        .lines()
        .filter(|l| {
            !l.trim_start().starts_with('#')
                && !l.trim_start().starts_with('-')
                && !l.trim().is_empty()
                && !l.trim_start().starts_with("```")
                && !l.contains(':')
        })
        .collect();
    assert!(
        prose_lines.len() >= 3,
        "minimal.md must have at least 3 lines of prose explanation, found: {:?}",
        prose_lines
    );
}

// ── Criterion 2 ──────────────────────────────────────────────────────────────
// docs/examples/minimal.md notes that cli.command is the only required field
// and all other top-level fields default to empty/absent via #[serde(default)].

#[test]
fn minimal_md_notes_cli_command_is_required() {
    let content = std::fs::read_to_string(repo_root().join("docs/examples/minimal.md"))
        .expect("docs/examples/minimal.md must be readable");
    assert!(
        content.to_lowercase().contains("only required"),
        "minimal.md must state that cli.command is the only required field"
    );
}

#[test]
fn minimal_md_mentions_serde_default() {
    let content = std::fs::read_to_string(repo_root().join("docs/examples/minimal.md"))
        .expect("docs/examples/minimal.md must be readable");
    assert!(
        content.contains("serde(default)") || content.contains("#[serde(default)]"),
        "minimal.md must mention serde(default) to explain why other fields are optional"
    );
}
