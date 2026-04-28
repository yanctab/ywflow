# ywflow

A configurable human-in-the-loop workflow runner for Claude Code.

## Project Overview

ywflow is a Rust-based CLI tool that wraps Claude Code sessions with
a structured, config-driven workflow. Each step in the workflow launches
an interactive Claude Code session with the correct model, context, and
skills pre-configured — keeping a human in control of each transition.

## Tech Stack

- **Language**: Rust
- **Target**: x86_64-unknown-linux-musl (static binary)
- **Packaging**: .deb and Arch Linux (pacman)
- **CLI**: clap v4 (dynamic subcommand registration)
- **Config**: serde + serde_yaml

## Project Structure

```
ywflow/
├── Cargo.toml
├── PKGBUILD
├── ywflow.yaml
├── .cargo/
│   └── config.toml
├── src/
│   ├── main.rs
│   ├── cli.rs
│   ├── config.rs
│   ├── context.rs
│   ├── workflow.rs
│   ├── plugins.rs
│   └── input.rs
└── packaging/
    └── debian/
```

## Config Structure

The workflow is defined in `ywflow.yaml` at the project root. The config
supports variable expansion via `${variable}` syntax, environment variable
injection via `${env:VAR}`, and per-step CLI arg inheritance from the
global CLI section.

## Key Design Decisions

- `setup` is the only built-in subcommand — all workflow steps are
  dynamically registered from `ywflow.yaml`
- Each workflow step launches a full interactive Claude Code session —
  never headless
- Human review is required between each step by design
- The binary must remain a single static musl-linked executable
- Variable expansion order: context variables first, then runtime
  variables like `${task}`, then `${env:X}`