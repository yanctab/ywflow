// src/prompt.rs
// Formats the assembled command as a shell-pasteable line and handles user
// confirmation when context or env tokens expanded to empty.

use crate::workflow::EmptyToken;
use crate::workflow::EmptyTokenSource;

/// Shell metacharacters that require quoting.
const SHELL_METACHARACTERS: &[char] = &[
    ' ', '\t', '\n', '"', '\'', '\\', '`', '!', '#', '$', '&', '(', ')', '*', ';', '<', '>', '?',
    '[', ']', '^', '{', '}', '|', '~',
];

/// Returns a single shell-pasteable string with the command and each argument.
/// Arguments containing whitespace or shell metacharacters are wrapped in double quotes,
/// with internal double quotes escaped.
pub fn format_command_line(command: &str, args: &[String]) -> String {
    let mut parts: Vec<String> = Vec::with_capacity(args.len() + 1);
    parts.push(shell_quote(command));
    for arg in args {
        parts.push(shell_quote(arg));
    }
    parts.join(" ")
}

/// Wrap a token in double quotes if it contains whitespace or shell metacharacters.
fn shell_quote(s: &str) -> String {
    if s.is_empty() || s.chars().any(|c| SHELL_METACHARACTERS.contains(&c)) {
        // Escape internal double quotes and backslashes, then wrap.
        let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{escaped}\"")
    } else {
        s.to_string()
    }
}

/// Write per-variable warnings to stderr for each empty context or env token,
/// echo the formatted command, prompt `Continue? [y/N]` on stdin, and return
/// whether to proceed.
///
/// Returns `true` only when the user answers `y` or `Y`.
/// Returns `false` when stdin is not a TTY (with a message) or when the user
/// answers with anything other than `y`/`Y`.
pub fn warn_and_confirm(empties: &[EmptyToken], formatted_cmd: &str) -> bool {
    // When stdin is not a TTY, auto-abort.
    if !is_stdin_tty() {
        eprintln!("ywflow: stdin is not a TTY — aborting to avoid unattended execution");
        return false;
    }

    // Print one warning line per empty token.
    for token in empties {
        let source_label = match token.source {
            EmptyTokenSource::Context => "context",
            EmptyTokenSource::Env => "env",
            EmptyTokenSource::StepArg => "step_arg",
        };
        eprintln!(
            "warning: '{}' ({}) expanded to empty",
            token.name, source_label
        );
    }

    // Echo the assembled command.
    eprintln!("{}", formatted_cmd);

    // Prompt the user.
    eprint!("Continue? [y/N] ");

    let mut input = String::new();
    match std::io::stdin().read_line(&mut input) {
        Ok(_) => {
            let trimmed = input.trim();
            trimmed == "y" || trimmed == "Y"
        }
        Err(_) => false,
    }
}

/// Returns `true` when stdin is connected to a TTY.
fn is_stdin_tty() -> bool {
    use std::os::unix::io::AsRawFd;
    libc_isatty(std::io::stdin().as_raw_fd())
}

/// Thin wrapper around the libc `isatty` call without importing the libc crate.
fn libc_isatty(fd: i32) -> bool {
    unsafe extern "C" {
        fn isatty(fd: i32) -> i32;
    }
    // SAFETY: isatty is a standard C function with well-defined behavior for any fd value.
    unsafe { isatty(fd) != 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Criterion 1: format_command_line ──────────────────────────────────────

    #[test]
    fn format_simple_command_no_args() {
        let result = format_command_line("claude", &[]);
        assert_eq!(result, "claude");
    }

    #[test]
    fn format_command_with_plain_args() {
        let args: Vec<String> = vec!["--model".to_string(), "claude-opus-4-5".to_string()];
        let result = format_command_line("claude", &args);
        assert_eq!(result, "claude --model claude-opus-4-5");
    }

    #[test]
    fn args_with_spaces_are_double_quoted() {
        let args: Vec<String> = vec!["hello world".to_string()];
        let result = format_command_line("claude", &args);
        assert_eq!(result, "claude \"hello world\"");
    }

    #[test]
    fn args_with_shell_metacharacters_are_quoted() {
        let args: Vec<String> = vec!["--flag".to_string(), "val$ue".to_string()];
        let result = format_command_line("claude", &args);
        assert_eq!(result, "claude --flag \"val$ue\"");
    }

    #[test]
    fn args_with_internal_double_quotes_are_escaped() {
        let args: Vec<String> = vec!["say \"hi\"".to_string()];
        let result = format_command_line("claude", &args);
        assert_eq!(result, "claude \"say \\\"hi\\\"\"");
    }

    #[test]
    fn empty_arg_is_double_quoted() {
        let args: Vec<String> = vec!["".to_string()];
        let result = format_command_line("claude", &args);
        assert_eq!(result, "claude \"\"");
    }
}
