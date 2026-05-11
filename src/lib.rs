// src/lib.rs
// Re-exports ywflow's public modules so integration tests can reach them via
// `ywflow::config`, `ywflow::context`, and `ywflow::workflow`.

pub mod config;
pub mod context;
pub mod workflow;

// `prompt` is not part of the public API but is referenced internally by
// `workflow::run_step` via `crate::prompt::…`, so it must be in scope.
pub(crate) mod prompt;
