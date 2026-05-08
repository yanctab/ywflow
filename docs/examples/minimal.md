# Minimal ywflow.yaml

This is the simplest valid `ywflow.yaml`. Only `cli.command` is required — every
other top-level field is annotated with `#[serde(default)]` in the source, so it
defaults to empty or absent when omitted.

```yaml
context:
  task: "my task description"

cli:
  command: claude

workflow:
  run:
    description: "Run the workflow step"
```

## Field explanations

**`context`** — A map of named variables that can be referenced elsewhere in the
config via `${variable_name}`. Here we define a single variable called `task`.
This section defaults to an empty map when omitted.

**`cli.command`** — The executable to invoke for each workflow step (e.g. `claude`).
This is the only required field in the entire config. All other top-level fields
(`required_env`, `context`, `plugins`, and `workflow`) are optional and default to
empty/absent via `#[serde(default)]`.

**`workflow`** — A named map of workflow steps. Each step must have at minimum a
`description` field. Steps may additionally specify `args` (user-supplied inputs)
and a `cli` override. Here the single step `run` has no `args`, keeping it as
simple as possible.

> **Note**: `cli.command` is the only required field. All other top-level fields
> default to empty/absent via `#[serde(default)]`, so you can omit `required_env`,
> `context`, and `plugins` entirely in the simplest case.
