# ywflow.yaml Configuration Reference

This document is a field-by-field reference for `ywflow.yaml`, the configuration
file that drives all ywflow behaviour. It covers every top-level key, every nested
struct, variable expansion rules, config discovery, and validation constraints.

---

## Top-level keys

| Key            | Type                        | Required | Purpose |
|----------------|-----------------------------|----------|---------|
| `required_env` | `Vec<String>`               | Optional | Environment variable names that must be set before ywflow runs. |
| `context`      | `map<String, String>`       | Optional | Named string variables available for `${variable}` expansion in args. |
| `cli`          | [`CliConfig`](#cli)         | **Required** | The command ywflow launches for each workflow step. |
| `plugins`      | `Vec<`[`Plugin`](#plugins)`>` | Optional | Claude Code plugins to install or verify before running. |
| `workflow`     | `map<String, `[`StepConfig`](#workflow)`>` | Optional | Workflow step definitions, keyed by step name. |

All top-level fields except `cli` have `#[serde(default)]` and may be omitted.
`cli.command` is the only required field in the entire schema.

---

## `cli`

Global CLI configuration. Applied to every workflow step unless overridden.

| Field     | Type          | Required | Purpose |
|-----------|---------------|----------|---------|
| `command` | `String`      | **Required** | The executable to run (e.g. `claude`). |
| `args`    | `Vec<String>` | Optional (default: `[]`) | Arguments prepended to every invocation. |

### Example

```yaml
cli:
  command: claude
  args:
    - --model
    - claude-opus-4-5
```

---

## `plugins`

A list of Claude Code plugins to check or install via `ywflow setup`.

Each entry in the list is a `Plugin` object:

| Field     | Type     | Required | Purpose |
|-----------|----------|----------|---------|
| `name`    | `String` | Required | Human-readable plugin identifier. |
| `source`  | `String` | Required | Where the plugin lives: `"marketplace"` or `"local"`. |
| `package` | `String` | Optional | Package name used when `source: marketplace`. Ignored for local plugins. |
| `path`    | `String` | Optional | Filesystem path used when `source: local`. Ignored for marketplace plugins. |

### Example

```yaml
plugins:
  - name: my-marketplace-plugin
    source: marketplace
    package: my-org/my-plugin

  - name: my-local-plugin
    source: local
    path: /home/user/plugins/my-plugin
```

---

## `workflow`

An ordered map from step name to step configuration. Step names become subcommands
of the `ywflow` binary (e.g. `ywflow plan`).

Each step is a `StepConfig` object:

| Field         | Type                        | Required | Purpose |
|---------------|-----------------------------|----------|---------|
| `description` | `String`                    | Required | Short description shown in `ywflow help`. |
| `args`        | `Vec<`[`StepArg`](#args)`>` | Optional (default: `[]`) | Positional arguments accepted by this step. |
| `cli`         | [`StepCliConfig`](#step-cli) | Optional | Step-level CLI overrides. |

### Example

```yaml
workflow:
  plan:
    description: "Plan the work"
    args:
      - name: task
        required: true
        help: "The task to plan"
    cli:
      args:
        - --extra-flag
```

---

## `args`

Each `StepArg` defines one positional argument for a workflow step.

| Field     | Type          | Required | Purpose |
|-----------|---------------|----------|---------|
| `name`    | `String`      | Required | Argument name; also used as the variable name in context expansion. |
| `accepts` | `Vec<String>` | Optional (default: `[]`) | Input type constraints. Possible values: `[]` (any string), `["file"]`, `["url"]`, or `["file", "url"]`. |
| `required`| `bool`        | Required | Whether the argument must be supplied. |
| `help`    | `String`      | Required | Help text shown in usage output. |

### Example

```yaml
args:
  - name: task
    required: true
    help: "The task to execute"
  - name: reference
    accepts:
      - file
      - url
    required: false
    help: "Optional reference file or URL"
```

---

## Step `cli`

Each workflow step may include a `cli` block (`StepCliConfig`) to supply
step-specific arguments to the underlying command.

| Field  | Type          | Required | Purpose |
|--------|---------------|----------|---------|
| `args` | `Vec<String>` | Optional (default: `[]`) | Additional arguments passed to the command for this step only. |

**Append behaviour:** step-level `cli.args` are appended after the global
`cli.args`; they do not replace them. The final argument list seen by the
command is: `[global cli.args] + [step cli.args]`.

### Example

```yaml
workflow:
  execute:
    description: "Execute the plan"
    cli:
      args:
        - --no-cache
```

When combined with the global config above, the full argument list would be:
`--model claude-opus-4-5 --no-cache`.

---

## Reserved context keys

The following context key names are reserved by ywflow and must not appear in
the `context:` block:

| Key     | Injected value |
|---------|----------------|
| `input` | The raw user-supplied input value (populated internally). |
| `cwd`   | The process working directory at invocation time (ISO path string). |
| `date`  | Today's date in ISO-8601 `YYYY-MM-DD` format. |

If any of these keys appear in `context:`, ywflow will reject the config with a
schema error at startup. They cannot be overridden by user-defined context values.

---

## Variable expansion

All string values in `cli.args` and `workflow.<step>.args` support
`${variable}` expansion. Expansion is performed in three passes, in order:

### Pass 1 — context variables

Context keys defined in the `context:` block are expanded against each other
using a cycle-safe depth-first search (DFS). A key may reference another context
key: `${other_key}`. If a circular reference is detected, ywflow exits with an
error naming the key involved in the cycle.

### Pass 2 — runtime keys

After context expansion, ywflow injects the following runtime values:

- `cwd` — the process working directory (`std::env::current_dir()`).
- `date` — today's date in ISO-8601 `YYYY-MM-DD` format.
- Named step arg values — each argument supplied on the command line is injected
  under its `name` as defined in the step's `args` list.

Any `${variable}` tokens in context values that reference these runtime keys are
resolved during this pass.

### Pass 3 — environment variables

Tokens of the form `${env:VAR}` are resolved by reading `VAR` from the process
environment (`std::env::var`). For example, `${env:GITHUB_TOKEN}` expands to the
value of the `GITHUB_TOKEN` environment variable.

### Undefined references

After all three passes, any remaining `${...}` token that could not be resolved
is treated as an error (undefined reference). ywflow will exit and report the
unresolved variable names.

---

## Config discovery

ywflow locates `ywflow.yaml` by walking up the directory tree:

1. Start from `std::env::current_dir()` (the directory where `ywflow` is invoked).
2. Check if `ywflow.yaml` exists in that directory.
3. If not found, move to the parent directory and repeat.
4. Continue until `ywflow.yaml` is found or the filesystem root is reached.

If no `ywflow.yaml` is found anywhere in the ancestor chain, ywflow exits with
the error:

```
no ywflow.yaml found in current directory or any parent directory
```

---

## Validation rules

After parsing `ywflow.yaml`, ywflow runs structural validation and rejects the
config if either of the following constraints is violated:

1. **Reserved context keys** — context keys must not use any of the reserved
   names (`input`, `cwd`, `date`). Using a reserved key is a schema error.

2. **Argument ordering** — within each step's `args` list, required args must
   precede optional args. A required argument that appears after an optional
   argument is a validation error. For example, placing a `required: false` arg
   before a `required: true` arg in the same step is forbidden.
