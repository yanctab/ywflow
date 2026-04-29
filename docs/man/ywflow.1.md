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

The workflow is defined in **ywflow.yaml** at the project root. All steps
declared in that file are registered as subcommands at runtime; **setup** is
the only built-in subcommand.

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

**ywflow** reads **ywflow.yaml** from the current working directory. The file
defines a *context* block of named variables and a *steps* list.

Variable expansion is supported in step arguments:

**${variable}**
:   Replaced with the value of a context variable defined in the *context:*
    block.

**${task}**
:   Replaced with the value of the runtime **--task** CLI argument passed to
    the step subcommand.

**${env:VAR}**
:   Replaced with the value of the shell environment variable *VAR*.

Variables are expanded in order: context variables, runtime variables,
environment variables.

# EXAMPLES

Initialise a new workflow in the current project:

```
ywflow setup
```

Run a step named *implement* with a runtime task argument:

```
ywflow implement --task "add user authentication"
```

# FILES

**ywflow.yaml**
:   Workflow configuration file, read from the current working directory.

# AUTHOR

Written by manszigher.

# SEE ALSO

**claude**(1)
