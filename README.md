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

ywflow reads `ywflow.yaml` by walking up from the current working directory
to the filesystem root — the first file found is used.

### Top-level keys

| Key | Type | Required |
|-----|------|----------|
| `required_env` | `Vec<String>` | no |
| `context` | `IndexMap<String, String>` | no |
| `cli` | `CliConfig` | yes (`cli.command` only) |
| `plugins` | `Vec<Plugin>` | no |
| `workflow` | `IndexMap<String, StepConfig>` | no |

See [`docs/config.md`](docs/config.md) for the full field reference.

### Plugins

Install Claude Code plugins via `ywflow setup`. Two source types are supported:

```yaml
# marketplace plugin
plugins:
  - name: yanct-claude-plugin
    source: marketplace
    package: yanct/yanct-claude-plugin
```

```yaml
# local plugin
plugins:
  - name: local-skills
    source: local
    path: .claude/skills
```

### Notes

> **Reserved context keys:** `input`, `cwd`, and `date` are reserved by ywflow
> and cannot be used in the `context:` block. ywflow will reject the config at
> startup if any of these keys are present.

> **Arg order:** within a step's `args` list, required args must precede
> optional args. A required arg placed after an optional arg is a validation
> error.

> **Config discovery:** ywflow walks up from the current working directory to
> the filesystem root, using the first `ywflow.yaml` it finds.

### Variable expansion

| Syntax | Resolved from |
|--------|---------------|
| `${variable}` | Context variables defined in the `context:` block |
| `${<arg-name>}` | Value of a named step arg (e.g. `${task}` is the value of a step arg named `task` — not a special keyword) |
| `${env:VAR}` | Shell environment variable `VAR` |

Variables are expanded in three passes: context variables first, then runtime
values (`cwd`, `date`, and step arg values), then environment variables.

### Step-level cli.args

Each workflow step may define a `cli.args` list under its `cli:` block.
Step-level `cli.args` are appended after the global `cli.args` — they are
not a replacement. The command receives `[global cli.args] + [step cli.args]`.

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
