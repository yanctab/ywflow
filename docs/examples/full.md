# Full Annotated ywflow.yaml

This document contains the complete `ywflow.yaml` from the repository root, with
prose explanations for every field. This is a real-world configuration that drives
the plan → breakdown → execute workflow used to build ywflow itself.

```yaml
required_env:
  - ANTHROPIC_API_KEY

context:
  planning_model: claude-opus-4-5
  api_key: ${env:ANTHROPIC_API_KEY}
  project_dir: ${cwd}

cli:
  command: claude
  args:
    - --model
    - ${planning_model}

plugins:
  - name: yanct-claude-plugin
    source: marketplace
    package: yanct/yanct-claude-plugin
  - name: local-skills
    source: local
    path: .claude/skills

workflow:
  plan:
    description: "Plan the work: analyse the task and produce a PRD"
    cli:
      args:
        - --worktree
        - plan-session
        - /prd-skill

  breakdown:
    description: "Break down the PRD into implementation slices"
    args:
      - name: prd
        accepts:
          - file
          - url
        required: true
        help: "Path or URL of the PRD to break down"
    cli:
      args:
        - --worktree
        - breakdown-session
        - /prd-to-issues-skill

  execute:
    description: "Execute a single implementation slice"
    args:
      - name: issue
        accepts:
          - url
        required: true
        help: "GitHub issue URL for the slice to implement"
      - name: notes
        accepts: []
        required: false
        help: "Optional implementation notes"
    cli:
      args:
        - --worktree
        - execute-session
        - /execute-skill
```

## Field-by-field explanation

### `required_env`

A list of environment variable names that must be set before ywflow runs. If any
listed variable is absent, ywflow exits with an error before launching any
subprocess. Here `ANTHROPIC_API_KEY` is required because `claude` needs it to
authenticate with Anthropic's API.

### `context`

A map of named variables that can be referenced via `${variable_name}` anywhere
else in the config (including inside `cli.args` and `workflow` step args). Three
special runtime keys are always available and cannot be overridden: `${cwd}` (the
working directory when ywflow is invoked), `${date}` (today's date), and
`${input}` (the value of the first positional arg, when present).

- **`planning_model`** — a plain string value used later via `${planning_model}`.
- **`api_key`** — expands the environment variable `ANTHROPIC_API_KEY` at runtime
  via the `${env:VAR}` syntax.
- **`project_dir`** — captures the current working directory via `${cwd}`.

### `cli`

The global CLI configuration applied to every workflow step.

- **`command`** — the executable to invoke (`claude`). This is the only required
  field in the entire config; all other top-level fields are optional.
- **`args`** — a list of arguments prepended to every step invocation. Here
  `--model ${planning_model}` passes the model name resolved from the context.

### `plugins`

An optional list of plugins to check (and optionally install) before running a
workflow step. Running `ywflow setup` walks this list and offers to install any
missing plugin.

- **`yanct-claude-plugin`** — a marketplace plugin sourced from
  `yanct/yanct-claude-plugin`. Marketplace plugins are installed from the Claude
  Code plugin registry.
- **`local-skills`** — a local plugin sourced from the `.claude/skills` directory
  in the project tree. Local plugins are checked for existence on disk.

### `workflow`

A named, ordered map of workflow steps. Each step is exposed as a subcommand of
the `ywflow` binary (e.g. `ywflow plan`, `ywflow breakdown <prd>`,
`ywflow execute <issue>`).

#### Step: `plan`

Launches `claude` with the global args plus the step-specific args to produce a
PRD from a task description.

- **`description`** — shown in `ywflow --help` next to the subcommand name.
- **`cli.args`** — step-level args appended after the global args. Here
  `--worktree plan-session /prd-skill` puts the session in a dedicated git worktree
  and loads the PRD skill.
- This step defines no `args` entries, so it takes no user-supplied positional
  inputs.

#### Step: `breakdown`

Accepts a PRD (as a file path or URL) and breaks it into implementation slices
filed as GitHub issues.

- **`args`** — a list of positional arguments the user supplies on the command
  line. Each arg has a `name`, an `accepts` list (restricting the value to `file`,
  `url`, or free-form text), a `required` flag, and a `help` string.
  - **`prd`** — required; accepts a file path or a URL pointing to the PRD
    document.
- **`cli.args`** — `--worktree breakdown-session /prd-to-issues-skill` loads the
  breakdown skill in its own worktree.

#### Step: `execute`

Takes a single GitHub issue URL and implements the slice it describes.

- **`args`**:
  - **`issue`** — required; accepts only a URL (the GitHub issue for the slice).
  - **`notes`** — optional; a free-form string for extra implementation guidance.
    Optional args must come after all required args.
- **`cli.args`** — `--worktree execute-session /execute-skill` places the
  session in a dedicated worktree and loads the execute skill.
