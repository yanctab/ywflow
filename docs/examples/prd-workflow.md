# PRD Workflow Walkthrough

This document narrates the complete plan → breakdown → execute sequence as it runs
in practice. Each step is a separate `ywflow` invocation so that a human can review
the output and decide whether to continue before the next step begins.

## Overview

The three-step workflow covers the full lifecycle of a feature or task:

1. **`ywflow plan`** — analyse the task and produce a Product Requirements Document (PRD).
2. **`ywflow breakdown <prd>`** — decompose the PRD into a set of implementation slices
   filed as GitHub issues.
3. **`ywflow execute <issue>`** — implement a single slice from the breakdown.

Human review is required between steps. ywflow exits after each step; the human
inspects the output and decides when to run the next command.

---

## Step 1: Plan

```
ywflow plan
```

ywflow assembles the final command by concatenating the global args from `cli.args`
with the step-level args from `workflow.plan.cli.args`:

```
claude [global_args...] [step_args...]
# expands to, for example:
claude --model claude-opus-4-5 --worktree plan-session /prd-skill
```

ywflow invokes `claude` as a **blocking interactive child process** — the terminal
is handed over to Claude Code and ywflow waits. The session is fully interactive;
the human converses with Claude until the PRD is complete.

When the session ends (the human types `/exit` or presses Ctrl-D), `claude` exits
and ywflow receives the exit code:

- If the exit code is **zero**, ywflow itself exits cleanly.
- If the exit code is **non-zero**, ywflow propagates it immediately via
  `std::process::exit(code)` so that the failure is visible to the calling shell
  or CI system.

At this point the human reviews the PRD produced by the plan session. ywflow exits
after each step; the human decides when to proceed.

---

## Step 2: Breakdown

```
ywflow breakdown path/to/prd.md
# or
ywflow breakdown https://github.com/org/repo/issues/1
```

The `prd` argument (required; accepts a file path or a URL) is validated by ywflow
before the subprocess is launched. The assembled command follows the same pattern:

```
claude [global_args...] [step_args...]
# expands to, for example:
claude --model claude-opus-4-5 --worktree breakdown-session /prd-to-issues-skill
```

Again, ywflow launches `claude` as a **blocking interactive child process**. The
human works with Claude to confirm and file the implementation slices as GitHub
issues. Non-zero exit from the child process propagates via `std::process::exit`.

After the session ends the human inspects the issues that were created. Human review
is required before deciding which issue to execute first.

---

## Step 3: Execute

```
ywflow execute https://github.com/org/repo/issues/42
# optionally:
ywflow execute https://github.com/org/repo/issues/42 "focus on the happy path first"
```

The `issue` argument (required; must be a URL) identifies the GitHub issue to
implement. The optional `notes` argument passes freeform guidance into the session.

ywflow assembles:

```
claude [global_args...] [step_args...]
# expands to, for example:
claude --model claude-opus-4-5 --worktree execute-session /execute-skill
```

The session is **blocking** and **interactive**. ywflow waits for `claude` to exit,
then propagates any non-zero exit code via `std::process::exit`.

Human review of the resulting branch and pull request happens outside ywflow. If
more slices remain, the human runs `ywflow execute` again for the next issue.

---

## Exit-code contract

Every `ywflow <step>` command follows the same contract:

| Child exit code | ywflow behaviour |
|-----------------|-----------------|
| `0` | ywflow exits `0` |
| non-zero | ywflow calls `std::process::exit(code)` with the same code |

This means `ywflow` is safe to use in scripts: a failing Claude session stops the
script with a meaningful exit code.
