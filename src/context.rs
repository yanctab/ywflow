// src/context.rs
// Builds and manages the variable-expansion context used across workflow steps.
// Expansion order: context variables, then runtime variables (${task}), then ${env:X}.

use anyhow::Result;

pub fn run() -> Result<()> {
    todo!()
}
