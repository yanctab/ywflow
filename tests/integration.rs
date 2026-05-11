// tests/integration.rs
// Full pipeline integration tests: config::parse_and_validate → context::resolve →
// workflow::assemble_argv, exercising all four PRD sub-cases.
//
// All tests load tests/assets/ywflow.yaml — the live top-level ywflow.yaml is never read.

use std::collections::HashMap;
use ywflow::{
    config::parse_and_validate,
    context::resolve,
    workflow::{EmptyTokenSource, assemble_argv},
};

const FIXTURE_YAML: &str = include_str!("assets/ywflow.yaml");

fn load_fixture() -> ywflow::config::Config {
    parse_and_validate(FIXTURE_YAML).expect("fixture must parse cleanly")
}

// ── Criterion 1: plan "fix parser bug" ──────────────────────────────────────

/// `ywflow plan "fix parser bug"` path: the combined-prompt argv entry expands
/// to `/<plugin>:new-prd fix parser bug` as a single token.
#[test]
fn plan_prompt_expands_to_single_combined_token() {
    let config = load_fixture();
    let step = &config.workflow["plan"];
    let raw_args: HashMap<String, String> =
        [("task".to_string(), "fix parser bug".to_string())].into();

    let resolved = resolve(&config, "plan", &raw_args).expect("context::resolve must succeed");

    let (argv, _empty_tokens) =
        assemble_argv(&config.cli, step, "plan", &resolved, &config.context);

    // The combined-prompt entry (last arg) must expand to the full slash-command in one token.
    let combined = argv
        .iter()
        .find(|a| a.contains(":new-prd"))
        .expect("argv must contain an entry with ':new-prd'");
    assert_eq!(
        combined, "/yanct-claude-plugin:new-prd fix parser bug",
        "combined-prompt must expand plugin and task into a single token"
    );
}

// ── Criterion 2: breakdown <prd-url> ────────────────────────────────────────

/// `ywflow breakdown <prd-url>` path: the combined-prompt argv entry contains
/// the prd value inline — not as a separate argv entry.
#[test]
fn breakdown_prompt_contains_prd_value_inline() {
    let config = load_fixture();
    let prd_url = "https://github.com/yanctab/ywflow/issues/62";
    let raw_args: HashMap<String, String> = [("prd".to_string(), prd_url.to_string())].into();

    // Remove YWFLOW_TEST_UNSET_VAR so it is absent (env-empty scenario handled by criterion 5).
    unsafe {
        std::env::remove_var("YWFLOW_TEST_UNSET_VAR");
    }

    let resolved = resolve(&config, "breakdown", &raw_args).expect("context::resolve must succeed");

    let step = &config.workflow["breakdown"];
    let (argv, _empty_tokens) =
        assemble_argv(&config.cli, step, "breakdown", &resolved, &config.context);

    // The combined prompt entry must contain the prd URL inline.
    let combined = argv
        .iter()
        .find(|a| a.contains(":prd-to-issues"))
        .expect("argv must contain ':prd-to-issues' entry");
    assert!(
        combined.contains(prd_url),
        "combined-prompt must contain the prd URL inline; got: {combined:?}"
    );
    // The prd URL must NOT appear as a standalone separate argv entry.
    let standalone_prd_entries: Vec<_> = argv.iter().filter(|a| a.as_str() == prd_url).collect();
    assert!(
        standalone_prd_entries.is_empty(),
        "prd URL must not appear as a standalone argv entry; got argv: {argv:?}"
    );
}

// ── Criterion 3: execute <issue-url> (no notes) ─────────────────────────────

/// `ywflow execute <issue-url>` without notes: returns one EmptyToken with
/// name="notes" and source=StepArg; no context/env empties present.
#[test]
fn execute_without_notes_has_only_step_arg_empty() {
    let config = load_fixture();
    let step = &config.workflow["execute"];
    let issue_url = "https://github.com/yanctab/ywflow/issues/66";
    // Only provide the required arg; omit optional "notes".
    let raw_args: HashMap<String, String> = [("issue".to_string(), issue_url.to_string())].into();

    let resolved = resolve(&config, "execute", &raw_args).expect("context::resolve must succeed");

    let (_argv, empty_tokens) =
        assemble_argv(&config.cli, step, "execute", &resolved, &config.context);

    // Exactly one empty token: "notes" (StepArg).
    let notes_empty: Vec<_> = empty_tokens.iter().filter(|t| t.name == "notes").collect();
    assert_eq!(
        notes_empty.len(),
        1,
        "must have exactly one empty token for 'notes'; got: {empty_tokens:?}"
    );
    assert!(
        matches!(notes_empty[0].source, EmptyTokenSource::StepArg),
        "notes empty token must be classified StepArg; got: {:?}",
        notes_empty[0].source
    );

    // No context or env empties.
    let context_empties: Vec<_> = empty_tokens
        .iter()
        .filter(|t| matches!(t.source, EmptyTokenSource::Context))
        .collect();
    assert!(
        context_empties.is_empty(),
        "must have no context empty tokens; got: {context_empties:?}"
    );
    let env_empties: Vec<_> = empty_tokens
        .iter()
        .filter(|t| matches!(t.source, EmptyTokenSource::Env))
        .collect();
    assert!(
        env_empties.is_empty(),
        "must have no env empty tokens; got: {env_empties:?}"
    );
}

// ── Criterion 4: execute <issue-url> "review carefully" ─────────────────────

/// `ywflow execute <issue-url> "review carefully"` path: the combined-prompt
/// entry expands to `/<plugin>:execute <issue-url> review carefully`; empty tokens empty.
#[test]
fn execute_with_notes_prompt_fully_expanded_and_no_empty_tokens() {
    let config = load_fixture();
    let step = &config.workflow["execute"];
    let issue_url = "https://github.com/yanctab/ywflow/issues/66";
    let raw_args: HashMap<String, String> = [
        ("issue".to_string(), issue_url.to_string()),
        ("notes".to_string(), "review carefully".to_string()),
    ]
    .into();

    let resolved = resolve(&config, "execute", &raw_args).expect("context::resolve must succeed");

    let (argv, empty_tokens) =
        assemble_argv(&config.cli, step, "execute", &resolved, &config.context);

    // Combined prompt must contain all three values inline.
    let combined = argv
        .iter()
        .find(|a| a.contains(":execute"))
        .expect("argv must contain ':execute' entry");
    assert!(
        combined.contains(issue_url),
        "combined prompt must contain issue URL; got: {combined:?}"
    );
    assert!(
        combined.contains("review carefully"),
        "combined prompt must contain notes; got: {combined:?}"
    );
    assert!(
        combined.starts_with("/yanct"),
        "combined prompt must start with the plugin prefix; got: {combined:?}"
    );

    // Vec<EmptyToken> must be empty — all tokens resolved to non-empty values.
    assert!(
        empty_tokens.is_empty(),
        "no empty tokens expected when all args provided; got: {empty_tokens:?}"
    );
}

// ── Criterion 5: env-unset path ─────────────────────────────────────────────

/// When YWFLOW_TEST_UNSET_VAR is not set, the returned Vec<EmptyToken> from
/// the breakdown step contains an entry classified EmptyTokenSource::Env.
#[test]
fn breakdown_with_unset_env_var_produces_env_empty_token() {
    // Ensure the env var is NOT set.
    unsafe {
        std::env::remove_var("YWFLOW_TEST_UNSET_VAR");
    }

    let config = load_fixture();
    let step = &config.workflow["breakdown"];
    let prd_url = "https://github.com/yanctab/ywflow/issues/62";
    let raw_args: HashMap<String, String> = [("prd".to_string(), prd_url.to_string())].into();

    let resolved = resolve(&config, "breakdown", &raw_args).expect("context::resolve must succeed");

    let (_argv, empty_tokens) =
        assemble_argv(&config.cli, step, "breakdown", &resolved, &config.context);

    let env_empties: Vec<_> = empty_tokens
        .iter()
        .filter(|t| matches!(t.source, EmptyTokenSource::Env))
        .collect();
    assert!(
        !env_empties.is_empty(),
        "must have at least one Env empty token when YWFLOW_TEST_UNSET_VAR is absent; got: {empty_tokens:?}"
    );
    assert!(
        env_empties
            .iter()
            .any(|t| t.name.contains("YWFLOW_TEST_UNSET_VAR")),
        "Env empty token must reference YWFLOW_TEST_UNSET_VAR; got: {env_empties:?}"
    );
}

// ── Criterion 6: empty-value context variable ────────────────────────────────

/// When the fixture's empty-value context variable (empty_var: "") is exercised
/// in a template, the returned Vec<EmptyToken> contains an entry classified
/// EmptyTokenSource::Context.
#[test]
fn empty_context_var_produces_context_empty_token() {
    use ywflow::config::{StepCliConfig, StepConfig};

    let config = load_fixture();
    // Construct a synthetic step whose cli.args template references ${empty_var}.
    let synthetic_step = StepConfig {
        description: "test step for empty_var".to_string(),
        args: vec![],
        cli: Some(StepCliConfig {
            args: vec!["${empty_var}".to_string()],
        }),
    };
    let raw_args: HashMap<String, String> = HashMap::new();
    let resolved = resolve(&config, "plan", &raw_args).expect("context::resolve must succeed");

    let (_argv, empty_tokens) = assemble_argv(
        &config.cli,
        &synthetic_step,
        "plan",
        &resolved,
        &config.context,
    );

    let context_empties: Vec<_> = empty_tokens
        .iter()
        .filter(|t| matches!(t.source, EmptyTokenSource::Context))
        .collect();
    assert!(
        !context_empties.is_empty(),
        "must have at least one Context empty token for empty_var; got: {empty_tokens:?}"
    );
    assert!(
        context_empties.iter().any(|t| t.name == "empty_var"),
        "Context empty token must be named 'empty_var'; got: {context_empties:?}"
    );
}
