// tests/man_page.rs
// Integration tests for docs/man/ywflow.1.md content requirements.

use std::fs;

fn man_page() -> String {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/man/ywflow.1.md");
    fs::read_to_string(path).expect("docs/man/ywflow.1.md must exist")
}

// ── Criterion 1: CONFIGURATION section accurate content ───────────────────────

#[test]
fn configuration_section_documents_all_five_top_level_keys() {
    let content = man_page();
    // All five top-level keys must appear in the CONFIGURATION section
    assert!(
        content.contains("required_env"),
        "CONFIGURATION must document required_env"
    );
    assert!(
        content.contains("context"),
        "CONFIGURATION must document context"
    );
    assert!(content.contains("cli"), "CONFIGURATION must document cli");
    assert!(
        content.contains("plugins"),
        "CONFIGURATION must document plugins"
    );
    assert!(
        content.contains("workflow"),
        "CONFIGURATION must document workflow"
    );
}

#[test]
fn configuration_section_notes_cli_command_is_only_required_field() {
    let content = man_page();
    // cli.command must be identified as the only required field
    assert!(
        content.contains("cli.command"),
        "CONFIGURATION must mention cli.command"
    );
    // The text must indicate it is required
    let idx = content.find("cli.command").unwrap();
    let surrounding = &content[idx.saturating_sub(100)..std::cmp::min(content.len(), idx + 200)];
    assert!(
        surrounding.contains("required") || surrounding.contains("only required"),
        "cli.command must be described as the only required field, found: {surrounding}"
    );
}

#[test]
fn configuration_section_documents_reserved_context_keys() {
    let content = man_page();
    // The three reserved keys must be documented
    let has_input = content.contains("input");
    let has_cwd = content.contains("cwd");
    let has_date = content.contains("date");
    assert!(
        has_input && has_cwd && has_date,
        "CONFIGURATION must document reserved keys: input, cwd, date"
    );
}

// ── Criterion 2: cli.args concatenation statement ─────────────────────────────

#[test]
fn configuration_states_step_args_appended_after_global_args() {
    let content = man_page();
    // Must explicitly state that step-level cli.args are appended after global cli.args
    let has_appended = content.contains("appended");
    let has_global_step = content.contains("global") && content.contains("step");
    assert!(
        has_appended && has_global_step,
        "CONFIGURATION must state step-level cli.args are appended after global cli.args"
    );
}

// ── Criterion 3: three-pass expansion documented in order ─────────────────────

#[test]
fn configuration_documents_three_pass_expansion_in_order() {
    let content = man_page();
    // Must document all three passes in order
    let pos_pass1 = content.find("context").unwrap_or(usize::MAX);
    let pos_pass2_cwd = content.find("cwd").unwrap_or(usize::MAX);
    let pos_pass3_env = content.find("env:").unwrap_or(usize::MAX);
    assert!(
        pos_pass1 < pos_pass2_cwd,
        "Pass 1 (context) must appear before pass 2 (cwd/date/step-args) in CONFIGURATION"
    );
    assert!(
        pos_pass2_cwd < pos_pass3_env,
        "Pass 2 (runtime keys) must appear before pass 3 (env:) in CONFIGURATION"
    );
}

// ── Criterion 4: DIAGNOSTICS section with all error conditions ────────────────

#[test]
fn diagnostics_section_exists() {
    let content = man_page();
    assert!(
        content.contains("# DIAGNOSTICS"),
        "DIAGNOSTICS section must be present"
    );
}

#[test]
fn diagnostics_documents_missing_required_env() {
    let content = man_page();
    assert!(
        content.contains("required_env") || content.contains("required environment"),
        "DIAGNOSTICS must describe missing required environment variable error"
    );
    // Must appear after DIAGNOSTICS heading
    let diag_pos = content.find("# DIAGNOSTICS").unwrap();
    let relevant = &content[diag_pos..];
    assert!(
        relevant.contains("required_env") || relevant.contains("required environment"),
        "Missing required env error must appear in DIAGNOSTICS section"
    );
}

#[test]
fn diagnostics_documents_plugin_not_installed_with_hint() {
    let content = man_page();
    let diag_pos = content.find("# DIAGNOSTICS").unwrap();
    let relevant = &content[diag_pos..];
    // Must mention plugin not installed and include the hint pattern
    assert!(
        (relevant.contains("plugin") || relevant.contains("Plugin"))
            && relevant.contains("Install Claude Code"),
        "DIAGNOSTICS must document plugin not installed with hint '→ Install Claude Code: <package>'"
    );
}

#[test]
fn diagnostics_documents_circular_reference() {
    let content = man_page();
    let diag_pos = content.find("# DIAGNOSTICS").unwrap();
    let relevant = &content[diag_pos..];
    assert!(
        relevant.contains("circular") || relevant.contains("Circular"),
        "DIAGNOSTICS must document circular variable reference in context:"
    );
}

#[test]
fn diagnostics_documents_undefined_variable() {
    let content = man_page();
    let diag_pos = content.find("# DIAGNOSTICS").unwrap();
    let relevant = &content[diag_pos..];
    assert!(
        relevant.contains("undefined") || relevant.contains("Undefined"),
        "DIAGNOSTICS must document undefined variable reference"
    );
}

#[test]
fn diagnostics_documents_arg_order_violation() {
    let content = man_page();
    let diag_pos = content.find("# DIAGNOSTICS").unwrap();
    let relevant = &content[diag_pos..];
    assert!(
        (relevant.contains("arg") || relevant.contains("Arg")) && relevant.contains("order"),
        "DIAGNOSTICS must document arg-order violation"
    );
}

#[test]
fn diagnostics_documents_ywflow_yaml_not_found() {
    let content = man_page();
    let diag_pos = content.find("# DIAGNOSTICS").unwrap();
    let relevant = &content[diag_pos..];
    assert!(
        relevant.contains("no ywflow.yaml found in current directory or any parent directory"),
        "DIAGNOSTICS must include exact not-found message"
    );
}

// ── Criterion 5: EXAMPLES replaced with three specific examples ───────────────

#[test]
fn examples_contains_ywflow_plan() {
    let content = man_page();
    assert!(
        content.contains("ywflow plan"),
        "EXAMPLES must include 'ywflow plan'"
    );
}

#[test]
fn examples_contains_ywflow_breakdown_with_prd_path() {
    let content = man_page();
    assert!(
        content.contains("ywflow breakdown"),
        "EXAMPLES must include 'ywflow breakdown <prd-path>'"
    );
}

#[test]
fn examples_contains_ywflow_execute_with_issue_url() {
    let content = man_page();
    assert!(
        content.contains("ywflow execute"),
        "EXAMPLES must include 'ywflow execute <issue-url> [notes]'"
    );
}

// ── Criterion 6: FILES section notes walk-up search ──────────────────────────

#[test]
fn files_section_documents_walk_up_search() {
    let content = man_page();
    let files_pos = content.find("# FILES").unwrap_or_else(|| {
        panic!("FILES section must be present");
    });
    let relevant = &content[files_pos..];
    // Must mention searching parent directories or walking up
    assert!(
        relevant.contains("parent") || relevant.contains("walk"),
        "FILES section must document walk-up search through parent directories"
    );
    assert!(
        relevant.contains("root") || relevant.contains("filesystem root"),
        "FILES section must mention the walk goes to filesystem root"
    );
}

// ── Criterion 7: No stale references ─────────────────────────────────────────

#[test]
fn no_reference_to_steps_field() {
    let content = man_page();
    // "steps" as a YAML key should not appear
    // We check for the pattern "steps" not appearing as a documented field name
    // (but it could appear in prose like "workflow steps")
    // The stale content said "steps list" — check for that
    assert!(
        !content.contains("*steps* list") && !content.contains("steps list"),
        "File must not reference 'steps list' (stale field name)"
    );
}

#[test]
fn no_reference_to_task_flag() {
    let content = man_page();
    assert!(
        !content.contains("--task"),
        "File must not reference --task flag"
    );
}

#[test]
fn no_reference_to_model_field() {
    let content = man_page();
    // "model" should not appear as a documented YAML field
    assert!(
        !content.contains("*model*") && !content.contains("`model`"),
        "File must not reference model field"
    );
}

// ── Criterion 8: make docs succeeds (tested separately via CI/manual) ─────────
// The pandoc render test is validated by the existence of the rendered output.
// We just check the markdown file is valid enough that pandoc won't reject it.

#[test]
fn man_page_has_title_block() {
    let content = man_page();
    // pandoc requires a % title block for man output
    assert!(
        content.starts_with('%'),
        "man page markdown must start with pandoc title block (% ...)"
    );
}

// ── Issue #43 AC1: CONFIGURATION section lists "string" as a valid accepts token ─

#[test]
fn configuration_section_lists_string_as_valid_accepts_token() {
    let content = man_page();
    // Find the CONFIGURATION section
    let config_pos = content
        .find("# CONFIGURATION")
        .expect("CONFIGURATION section must exist");
    // Find end of CONFIGURATION section (next top-level # heading)
    let after_config = &content[config_pos..];
    let next_section_offset = after_config[1..]
        .find("\n# ")
        .map(|i| i + 1)
        .unwrap_or(after_config.len());
    let config_section = &after_config[..next_section_offset];
    // "string" must appear as a valid accepts token alongside "file" and "url"
    assert!(
        config_section.contains("\"string\"") || config_section.contains("`string`"),
        "CONFIGURATION section must list \"string\" as a valid accepts token"
    );
}

// ── Issue #43 AC2: EXAMPLES section shows ywflow plan "my task" ───────────────

#[test]
fn examples_shows_plan_with_my_task_argument() {
    let content = man_page();
    // The EXAMPLES section must show ywflow plan "my task" not argumentless ywflow plan
    assert!(
        content.contains("ywflow plan \"my task\""),
        "EXAMPLES must show 'ywflow plan \"my task\"' with a quoted argument"
    );
}

#[test]
fn examples_does_not_show_argumentless_plan() {
    let content = man_page();
    // The argumentless "ywflow plan" (alone on a line) must not appear in EXAMPLES
    // It is acceptable as part of a larger string like 'ywflow plan "my task"'
    // Check that there is no line that is just "ywflow plan" (with no argument after it)
    let lines_with_plan: Vec<&str> = content
        .lines()
        .filter(|l| l.trim() == "ywflow plan")
        .collect();
    assert!(
        lines_with_plan.is_empty(),
        "EXAMPLES must not show argumentless 'ywflow plan' on its own line; found: {:?}",
        lines_with_plan
    );
}
