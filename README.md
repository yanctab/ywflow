# ywflow

A configurable human-in-the-loop workflow runner for Claude Code.

Each step launches an interactive Claude Code session with the correct model,
context, and skills pre-configured — keeping a human in control of every
transition.

## Installation

### Arch Linux (AUR)

```
yay -S ywflow
```

### Debian / Ubuntu

Download the latest `.deb` from the [releases page](https://github.com/manszigher/ywflow/releases)
and install:

```
sudo dpkg -i ywflow_<version>_amd64.deb
```

### crates.io

```
cargo install ywflow
```

### From source

```
cargo build --release --target x86_64-unknown-linux-musl
```

The resulting static binary is at `target/x86_64-unknown-linux-musl/release/ywflow`.

## Usage

```
ywflow [OPTIONS] COMMAND
```

### Built-in commands

| Command | Description |
|---------|-------------|
| `setup` | Initialise ywflow in the current project (creates `ywflow.yaml` scaffold) |

All workflow steps defined in `ywflow.yaml` are registered as subcommands at
runtime.

### Options

| Flag | Description |
|------|-------------|
| `--help` | Print help |
| `--version` | Print version |

### Examples

```
# Initialise a new ywflow project
ywflow setup

# Run a workflow step defined in ywflow.yaml
ywflow <step-name> --task "implement login feature"
```

## Configuration

ywflow reads `ywflow.yaml` from the project root. A minimal example:

```yaml
context:
  repo: my-project
  owner: acme

steps:
  - name: prd
    description: "Capture a product requirements document"
    model: claude-opus-4-5
    skills:
      - write-prd

  - name: implement
    description: "Implement a feature slice test-first"
    model: claude-sonnet-4-6
    skills:
      - tdd
      - git-commit
    args:
      task: "${task}"
```

### Variable expansion

| Syntax | Resolved from |
|--------|---------------|
| `${variable}` | Context variables defined in the `context:` block |
| `${task}` | Runtime CLI argument `--task` |
| `${env:VAR}` | Shell environment variable `VAR` |

Variables are expanded in this order: context variables, then runtime
variables, then environment variables.

## Development

```
make build    # compile static musl binary
make test     # run tests
make lint     # cargo fmt --check + clippy
make release  # tag v<version> and push to trigger CI release pipeline
make package  # build .deb and AUR packages from the release binary
make publish  # publish the crate to crates.io
```

See [`docs/config.md`](docs/config.md) for the full `ywflow.yaml` field reference,
or `man ywflow` for the man page.
