% YWFLOW(1) Version 0.1.0 | User Commands

# NAME

ywflow - configurable human-in-the-loop workflow runner for Claude Code

# SYNOPSIS

**ywflow** [*OPTIONS*] *COMMAND* [*ARGS*]

# DESCRIPTION

**ywflow** is a Rust-based CLI tool that wraps Claude Code sessions with a
structured, config-driven workflow. Each step launches an interactive Claude
Code session with the correct model, context, and skills pre-configured,
keeping a human in control of every transition.

The workflow is defined in **ywflow.yaml**. All steps declared in that file
are registered as subcommands at runtime; **setup** is the only built-in
subcommand.

# COMMANDS

**setup**
:   Initialise ywflow in the current project. Creates a **ywflow.yaml**
    scaffold in the working directory.

*\<step\>*
:   Run the named workflow step as defined in **ywflow.yaml**. Step
    subcommands are registered dynamically at startup.

# OPTIONS

**--help**
:   Print help and exit.

**--version**
:   Print version and exit.

# CONFIGURATION

**ywflow** reads **ywflow.yaml**, walking up from the current working
directory to the filesystem root until the file is found.

The file recognises five top-level keys:

**required_env**
:   A list of environment variable names that must be set before any step
    runs. Missing variables produce an error with an export hint.

**context**
:   A map of named string variables available for `${variable}` expansion
    in step arguments. The keys **input**, **cwd**, and **date** are
    reserved and may not be defined here.

**cli**
:   Configures the command used to launch each workflow step.
    `cli.command` is the only required field in the entire configuration
    file. The optional `cli.args` list provides global flags that are
    prepended to every step invocation.

**plugins**
:   A list of Claude Code plugins to verify (and optionally install) when
    `ywflow setup` runs.

**workflow**
:   A map of step names to step definitions. Each step has a
    **description**, an optional **args** list, and an optional **cli**
    block. The step-level `cli.args` are appended after the global
    `cli.args`; they do not replace them.

## Variable expansion

Variable expansion uses three passes applied in order:

**Pass 1 — context keys against each other**
:   Each entry in the `context:` block is expanded against the other
    context entries using a cycle-safe depth-first search.

**Pass 2 — runtime keys**
:   The reserved keys `cwd` (process working directory) and `date`
    (ISO-8601 YYYY-MM-DD) are injected, then named step-arg values are
    merged in. Any remaining `${...}` tokens that reference now-available
    keys are substituted.

**Pass 3 — environment variables**
:   `${env:VAR}` tokens are replaced with the value of the corresponding
    process environment variable.

Any `${...}` token that remains unresolved after all three passes is an
error.

# DIAGNOSTICS

**Required environment variable not set**
:   A variable listed under `required_env` is absent from the environment.
    The error names the missing variable and suggests adding it to the
    shell profile.

**Plugin not installed**
:   A plugin listed under `plugins` is not present in the Claude Code
    plugin directory. The hint reads: `→ Install Claude Code: <package>`

**Circular variable reference in context:**
:   Two or more context entries reference each other in a cycle.
    The error names one of the variables involved.

**Undefined variable reference**
:   A `${...}` token remains unresolved after all three expansion passes.
    The error lists every unresolved token name.

**Arg-order violation**
:   Within a step's `args` list, a required arg appears after an optional
    arg. Required args must precede optional args.

**ywflow.yaml not found**
:   No config file was found after walking up from the current directory.
    Exact message: `no ywflow.yaml found in current directory or any parent directory`

# EXAMPLES

Run the planning step (no arguments):

```
ywflow plan
```

Break down a PRD file into implementation slices:

```
ywflow breakdown <prd-path>
```

Execute an issue with an optional notes argument:

```
ywflow execute <issue-url> [notes]
```

# FILES

**ywflow.yaml**
:   Workflow configuration file. Discovery starts from the current working
    directory and walks parent directories to the filesystem root. The first
    **ywflow.yaml** found is used.

# AUTHOR

Written by manszigher.

# SEE ALSO

**claude**(1)
