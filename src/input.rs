// src/input.rs
// Argument type validation — checks each step arg value against its declared accepts list.

#![allow(dead_code)]

use crate::config::AcceptsType;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum InputError {
    #[error("argument '{name}' accepts [{accepts}] but '{value}' is a {actual}")]
    TypeMismatch {
        name: String,
        accepts: String,
        value: String,
        actual: String,
    },
    #[error("argument '{name}': file '{value}' does not exist")]
    FileNotFound { name: String, value: String },
}

/// Validate `value` against the declared `accepts` list for argument `arg_name`.
///
/// `http_check` is injected so callers can stub it in tests (no real network calls).
pub fn validate(
    arg_name: &str,
    value: &str,
    accepts: &[AcceptsType],
    http_check: &dyn Fn(&str) -> bool,
) -> Result<(), InputError> {
    let _ = http_check;
    if accepts.is_empty() {
        return Ok(());
    }
    let is_url = value.starts_with("http://") || value.starts_with("https://");
    for accept in accepts {
        if let AcceptsType::File = accept
            && !is_url
            && std::path::Path::new(value).is_file()
        {
            return Ok(());
        }
    }
    let accepts_str = accepts
        .iter()
        .map(|a| match a {
            AcceptsType::File => "file",
            AcceptsType::Url => "url",
        })
        .collect::<Vec<_>>()
        .join(", ");
    if is_url {
        return Err(InputError::TypeMismatch {
            name: arg_name.to_string(),
            accepts: accepts_str,
            value: value.to_string(),
            actual: "URL".to_string(),
        });
    }
    Err(InputError::FileNotFound {
        name: arg_name.to_string(),
        value: value.to_string(),
    })
}

/// Production HTTP HEAD check using reqwest blocking.
pub fn http_head_check(_url: &str) -> bool {
    todo!("http_head_check not yet implemented")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn no_http(_url: &str) -> bool {
        panic!("http_check must not be called for file-only tests")
    }

    // Criterion 3: non-existent file path + accepts [File] → FileNotFound
    #[test]
    fn file_missing() {
        let result = validate(
            "myarg",
            "/nonexistent/path/to/file.txt",
            &[AcceptsType::File],
            &no_http,
        );
        assert!(
            matches!(result, Err(InputError::FileNotFound { ref name, .. }) if name == "myarg"),
            "expected FileNotFound, got {:?}",
            result
        );
    }

    // Criterion 2: URL string + accepts [File] only → TypeMismatch
    #[test]
    fn file_url_rejected() {
        let result = validate(
            "myarg",
            "https://example.com/spec.html",
            &[AcceptsType::File],
            &no_http,
        );
        assert!(
            matches!(result, Err(InputError::TypeMismatch { ref name, .. }) if name == "myarg"),
            "expected TypeMismatch, got {:?}",
            result
        );
    }

    // Criterion 1: existing file path + accepts [File] → Ok
    #[test]
    fn file_valid() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "content").unwrap();
        let path = f.path().to_str().unwrap();

        let result = validate("myarg", path, &[AcceptsType::File], &no_http);
        assert!(
            result.is_ok(),
            "expected Ok for existing file, got {:?}",
            result
        );
    }
}
