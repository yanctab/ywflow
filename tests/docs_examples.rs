// tests/docs_examples.rs
// Integration tests verifying the docs/examples/ documentation files exist
// and contain the required content per issue #29 acceptance criteria.
// Also covers docs/config.md content requirements per issue #39.

use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

// ── Issue #39 AC4: docs/config.md lists "string" as a valid accepts token ────

#[test]
fn config_md_accepts_field_lists_string_token() {
    let path = repo_root().join("docs/config.md");
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("docs/config.md must be readable at {:?}", path));
    assert!(
        content.contains("\"string\"") || content.contains("`string`"),
        "docs/config.md accepts field documentation must list \"string\" as a valid token"
    );
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

// ── Criterion 3 ──────────────────────────────────────────────────────────────
// docs/examples/full.md exists and contains the complete annotated YAML from
// the repo-root ywflow.yaml exactly, with inline comments or surrounding prose
// explaining every field.

#[test]
fn full_md_exists() {
    let path = repo_root().join("docs/examples/full.md");
    assert!(
        path.exists(),
        "docs/examples/full.md must exist at {:?}",
        path
    );
}

#[test]
fn full_md_contains_required_env_section() {
    let content = std::fs::read_to_string(repo_root().join("docs/examples/full.md"))
        .expect("docs/examples/full.md must be readable");
    assert!(
        content.contains("ANTHROPIC_API_KEY"),
        "full.md must include 'ANTHROPIC_API_KEY' from the repo-root ywflow.yaml"
    );
}

#[test]
fn full_md_contains_complete_context_section() {
    let content = std::fs::read_to_string(repo_root().join("docs/examples/full.md"))
        .expect("docs/examples/full.md must be readable");
    assert!(
        content.contains("planning_model: claude-opus-4-5"),
        "full.md must include 'planning_model: claude-opus-4-5' from the repo-root ywflow.yaml"
    );
    assert!(
        content.contains("api_key: ${env:ANTHROPIC_API_KEY}"),
        "full.md must include 'api_key: ${{env:ANTHROPIC_API_KEY}}' from the repo-root ywflow.yaml"
    );
    assert!(
        content.contains("project_dir: ${cwd}"),
        "full.md must include 'project_dir: ${{cwd}}' from the repo-root ywflow.yaml"
    );
}

#[test]
fn full_md_contains_plugins_section() {
    let content = std::fs::read_to_string(repo_root().join("docs/examples/full.md"))
        .expect("docs/examples/full.md must be readable");
    assert!(
        content.contains("yanct-claude-plugin"),
        "full.md must include 'yanct-claude-plugin' from the repo-root ywflow.yaml"
    );
    assert!(
        content.contains("local-skills"),
        "full.md must include 'local-skills' from the repo-root ywflow.yaml"
    );
}

#[test]
fn full_md_explains_every_field_with_prose() {
    let content = std::fs::read_to_string(repo_root().join("docs/examples/full.md"))
        .expect("docs/examples/full.md must be readable");
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
        prose_lines.len() >= 5,
        "full.md must have at least 5 lines of prose explanation for every field, found: {:?}",
        prose_lines
    );
}

// ── Criterion 4 ──────────────────────────────────────────────────────────────
// docs/examples/full.md explains the three steps (plan, breakdown, execute)
// and their arg definitions.

#[test]
fn full_md_explains_plan_step() {
    let content = std::fs::read_to_string(repo_root().join("docs/examples/full.md"))
        .expect("docs/examples/full.md must be readable");
    assert!(
        content.contains("plan:") || content.contains("**plan**") || content.contains("### plan"),
        "full.md must include the 'plan' step"
    );
    assert!(
        content.contains("/prd-skill"),
        "full.md must include '/prd-skill' from the plan step cli args"
    );
}

#[test]
fn full_md_explains_breakdown_step_with_args() {
    let content = std::fs::read_to_string(repo_root().join("docs/examples/full.md"))
        .expect("docs/examples/full.md must be readable");
    assert!(
        content.contains("breakdown:")
            || content.contains("**breakdown**")
            || content.contains("### breakdown"),
        "full.md must include the 'breakdown' step"
    );
    assert!(
        content.contains("prd"),
        "full.md must document the 'prd' arg of the breakdown step"
    );
}

#[test]
fn full_md_explains_execute_step_with_args() {
    let content = std::fs::read_to_string(repo_root().join("docs/examples/full.md"))
        .expect("docs/examples/full.md must be readable");
    assert!(
        content.contains("execute:")
            || content.contains("**execute**")
            || content.contains("### execute"),
        "full.md must include the 'execute' step"
    );
    assert!(
        content.contains("issue"),
        "full.md must document the 'issue' arg of the execute step"
    );
    assert!(
        content.contains("notes"),
        "full.md must document the optional 'notes' arg of the execute step"
    );
}

// ── Criterion 5 ──────────────────────────────────────────────────────────────
// docs/examples/prd-workflow.md exists and narrates the plan->breakdown->execute
// sequence end-to-end.

#[test]
fn prd_workflow_md_exists() {
    let path = repo_root().join("docs/examples/prd-workflow.md");
    assert!(
        path.exists(),
        "docs/examples/prd-workflow.md must exist at {:?}",
        path
    );
}

#[test]
fn prd_workflow_md_narrates_all_three_steps() {
    let content = std::fs::read_to_string(repo_root().join("docs/examples/prd-workflow.md"))
        .expect("docs/examples/prd-workflow.md must be readable");
    assert!(
        content.contains("plan"),
        "prd-workflow.md must narrate the 'plan' step"
    );
    assert!(
        content.contains("breakdown"),
        "prd-workflow.md must narrate the 'breakdown' step"
    );
    assert!(
        content.contains("execute"),
        "prd-workflow.md must narrate the 'execute' step"
    );
}

// ── Criterion 6 ──────────────────────────────────────────────────────────────
// docs/examples/prd-workflow.md states that each step launches
// `claude [global_args...] [step_args...]` as a blocking interactive child process.

#[test]
fn prd_workflow_md_states_blocking_child_process() {
    let content = std::fs::read_to_string(repo_root().join("docs/examples/prd-workflow.md"))
        .expect("docs/examples/prd-workflow.md must be readable");
    assert!(
        content.contains("blocking") || content.contains("block"),
        "prd-workflow.md must state that the child process is launched as blocking"
    );
    assert!(
        content.contains("interactive"),
        "prd-workflow.md must state that the child process is interactive"
    );
}

#[test]
fn prd_workflow_md_shows_argv_pattern() {
    let content = std::fs::read_to_string(repo_root().join("docs/examples/prd-workflow.md"))
        .expect("docs/examples/prd-workflow.md must be readable");
    assert!(
        content.contains("global_args") || content.contains("[global args]"),
        "prd-workflow.md must document the `claude [global_args...] [step_args...]` invocation pattern"
    );
    assert!(
        content.contains("step_args") || content.contains("[step args]"),
        "prd-workflow.md must document the step_args portion of the command"
    );
}

// ── Criterion 7 ──────────────────────────────────────────────────────────────
// docs/examples/prd-workflow.md states that human review is required between
// steps (the tool exits after each step; the human decides when to run the next).

#[test]
fn prd_workflow_md_states_human_review_required() {
    let content = std::fs::read_to_string(repo_root().join("docs/examples/prd-workflow.md"))
        .expect("docs/examples/prd-workflow.md must be readable");
    assert!(
        content.to_lowercase().contains("human review")
            || content.to_lowercase().contains("human decides"),
        "prd-workflow.md must state that human review is required between steps"
    );
}

#[test]
fn prd_workflow_md_states_tool_exits_after_each_step() {
    let content = std::fs::read_to_string(repo_root().join("docs/examples/prd-workflow.md"))
        .expect("docs/examples/prd-workflow.md must be readable");
    assert!(
        content.contains("exits after") || content.contains("exit after"),
        "prd-workflow.md must state that ywflow exits after each step"
    );
}

// ── Criterion 8 ──────────────────────────────────────────────────────────────
// docs/examples/prd-workflow.md states that a non-zero exit from the child
// process propagates via std::process::exit.

#[test]
fn prd_workflow_md_states_nonzero_exit_propagates() {
    let content = std::fs::read_to_string(repo_root().join("docs/examples/prd-workflow.md"))
        .expect("docs/examples/prd-workflow.md must be readable");
    assert!(
        content.contains("non-zero") || content.contains("nonzero"),
        "prd-workflow.md must mention non-zero exit code propagation"
    );
}

#[test]
fn prd_workflow_md_mentions_process_exit() {
    let content = std::fs::read_to_string(repo_root().join("docs/examples/prd-workflow.md"))
        .expect("docs/examples/prd-workflow.md must be readable");
    assert!(
        content.contains("std::process::exit"),
        "prd-workflow.md must mention std::process::exit as the propagation mechanism"
    );
}
