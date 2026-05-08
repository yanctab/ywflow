// src/plugins.rs
// Plugin install-check and interactive installation wired into `ywflow setup`.

use crate::config::{Config, Plugin};
use anyhow::Result;
use std::collections::HashSet;

// ── Public types ──────────────────────────────────────────────────────────────

pub struct InstalledPlugins {
    pub names: HashSet<String>,
}

#[derive(Debug, PartialEq)]
pub enum PluginStatus {
    AlreadyInstalled { name: String },
    Installed { name: String },
    Skipped { name: String },
    Failed { name: String, error: String },
}

// ── Core plugin-setup logic (dependency-injected for testability) ─────────────

/// Check and optionally install each plugin in `plugins`.
///
/// * `settings_reader` — returns the set of already-installed plugin names.
/// * `installer`       — runs `npx claude-plugins install <package>`.
///
/// Prompts are written to stderr; user input is read from `stdin_reader`.
pub fn run_plugin_setup(
    plugins: &[Plugin],
    settings_reader: &dyn Fn() -> Result<InstalledPlugins>,
    installer: &dyn Fn(&str) -> Result<()>,
) -> Vec<PluginStatus> {
    run_plugin_setup_with_io(
        plugins,
        settings_reader,
        installer,
        &mut std::io::stdin().lock(),
        &mut std::io::stderr(),
    )
}

pub fn run_plugin_setup_with_io(
    plugins: &[Plugin],
    settings_reader: &dyn Fn() -> Result<InstalledPlugins>,
    installer: &dyn Fn(&str) -> Result<()>,
    stdin: &mut dyn std::io::BufRead,
    stderr: &mut dyn std::io::Write,
) -> Vec<PluginStatus> {
    let installed = settings_reader().unwrap_or_else(|_| InstalledPlugins {
        names: HashSet::new(),
    });

    plugins
        .iter()
        .map(|plugin| {
            if installed.names.contains(&plugin.name) {
                return PluginStatus::AlreadyInstalled {
                    name: plugin.name.clone(),
                };
            }

            // Prompt user
            let _ = writeln!(stderr, "Install {}? [y/N]", plugin.name);
            let mut answer = String::new();
            let _ = stdin.read_line(&mut answer);
            let trimmed = answer.trim().to_lowercase();

            if trimmed == "y" {
                let package = plugin.package.as_deref().unwrap_or(&plugin.name);
                match installer(package) {
                    Ok(()) => PluginStatus::Installed {
                        name: plugin.name.clone(),
                    },
                    Err(e) => PluginStatus::Failed {
                        name: plugin.name.clone(),
                        error: e.to_string(),
                    },
                }
            } else {
                PluginStatus::Skipped {
                    name: plugin.name.clone(),
                }
            }
        })
        .collect()
}

// ── Filesystem helpers ────────────────────────────────────────────────────────

/// Read installed plugins from `~/.claude/settings.json`.
pub fn read_installed_plugins() -> Result<InstalledPlugins> {
    let path = dirs_home().join(".claude").join("settings.json");
    let content = std::fs::read_to_string(&path)?;
    let value: serde_json::Value = serde_json::from_str(&content)?;

    let names = value
        .get("plugins")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| item.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    Ok(InstalledPlugins { names })
}

fn dirs_home() -> std::path::PathBuf {
    std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("/root"))
}

/// Return the names of any skill files referenced in `config` that are not
/// present on disk.
pub fn verify_skill_files(config: &Config) -> Vec<String> {
    // Skill files are referenced via the `skills` key inside step context
    // (not yet part of the typed Config struct — we scan the plugins list
    // for local-source entries whose `path` does not exist).
    config
        .plugins
        .iter()
        .filter_map(|p| {
            if let crate::config::PluginSource::Local = p.source {
                let path_str = p.path.as_deref().unwrap_or("");
                if !path_str.is_empty() && !std::path::Path::new(path_str).exists() {
                    Some(p.name.clone())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
}

// ── Ready summary ─────────────────────────────────────────────────────────────

/// Print the structured "ready" summary to stdout.
pub fn print_ready_summary(
    config_valid: bool,
    env_ready: bool,
    claude_version: Option<&str>,
    plugin_statuses: &[PluginStatus],
    missing_skills: &[String],
) {
    print_ready_summary_to(
        config_valid,
        env_ready,
        claude_version,
        plugin_statuses,
        missing_skills,
        &mut std::io::stdout(),
    )
}

pub fn print_ready_summary_to(
    config_valid: bool,
    env_ready: bool,
    claude_version: Option<&str>,
    plugin_statuses: &[PluginStatus],
    missing_skills: &[String],
    out: &mut dyn std::io::Write,
) {
    let check = |ok: bool| if ok { "✓" } else { "✗" };

    let _ = writeln!(out, "{} Config valid", check(config_valid));
    let _ = writeln!(out, "{} Environment ready", check(env_ready));
    if let Some(ver) = claude_version {
        let _ = writeln!(out, "✓ Claude Code found ({})", ver);
    } else {
        let _ = writeln!(out, "✗ Claude Code not found");
    }

    let plugins_ok = plugin_statuses.iter().all(|s| {
        matches!(
            s,
            PluginStatus::AlreadyInstalled { .. } | PluginStatus::Installed { .. }
        )
    });
    let _ = writeln!(out, "{} Plugins installed", check(plugins_ok));

    let skills_ok = missing_skills.is_empty();
    let _ = writeln!(out, "{} Skills found", check(skills_ok));

    let _ = writeln!(out, "Ready. Run: ywflow plan");
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Plugin, PluginSource};

    fn make_marketplace_plugin(name: &str) -> Plugin {
        Plugin {
            name: name.to_string(),
            source: PluginSource::Marketplace,
            package: Some(format!("my-org/{}", name)),
            path: None,
        }
    }

    // Criterion 1: AlreadyInstalled when plugin name is in settings
    #[test]
    fn already_installed() {
        let plugin = make_marketplace_plugin("my-plugin");
        let mut installed_names = HashSet::new();
        installed_names.insert("my-plugin".to_string());

        let installer_called = std::cell::Cell::new(false);

        let statuses = run_plugin_setup_with_io(
            &[plugin],
            &|| {
                Ok(InstalledPlugins {
                    names: installed_names.clone(),
                })
            },
            &|_pkg| {
                installer_called.set(true);
                Ok(())
            },
            &mut std::io::empty(),
            &mut std::io::sink(),
        );

        assert_eq!(statuses.len(), 1);
        assert!(
            matches!(&statuses[0], PluginStatus::AlreadyInstalled { name } if name == "my-plugin"),
            "expected AlreadyInstalled, got {:?}",
            statuses[0]
        );
        assert!(
            !installer_called.get(),
            "installer should not be called for already-installed plugin"
        );
    }

    // Criterion 2: Missing plugin answered "N" → Skipped, no installer call
    #[test]
    fn missing_skipped() {
        let plugin = make_marketplace_plugin("new-plugin");
        let installer_called = std::cell::Cell::new(false);

        let statuses = run_plugin_setup_with_io(
            &[plugin],
            &|| {
                Ok(InstalledPlugins {
                    names: HashSet::new(),
                })
            },
            &|_pkg| {
                installer_called.set(true);
                Ok(())
            },
            &mut std::io::Cursor::new(b"N\n"),
            &mut std::io::sink(),
        );

        assert_eq!(statuses.len(), 1);
        assert!(
            matches!(&statuses[0], PluginStatus::Skipped { name } if name == "new-plugin"),
            "expected Skipped, got {:?}",
            statuses[0]
        );
        assert!(
            !installer_called.get(),
            "installer should not be called when user skips"
        );
    }

    // Criterion 3: Missing plugin answered "y" → installer called → Installed
    #[test]
    fn missing_installed() {
        let plugin = make_marketplace_plugin("new-plugin");

        let statuses = run_plugin_setup_with_io(
            &[plugin],
            &|| {
                Ok(InstalledPlugins {
                    names: HashSet::new(),
                })
            },
            &|_pkg| Ok(()),
            &mut std::io::Cursor::new(b"y\n"),
            &mut std::io::sink(),
        );

        assert_eq!(statuses.len(), 1);
        assert!(
            matches!(&statuses[0], PluginStatus::Installed { name } if name == "new-plugin"),
            "expected Installed, got {:?}",
            statuses[0]
        );
    }

    // Criterion 4: verify_skill_files returns names of missing local skill files
    #[test]
    fn skill_files_missing() {
        use crate::config::CliConfig;
        use indexmap::IndexMap;

        let config = Config {
            required_env: vec![],
            context: IndexMap::new(),
            cli: CliConfig {
                command: "claude".to_string(),
                args: vec![],
            },
            plugins: vec![Plugin {
                name: "local-skill".to_string(),
                source: PluginSource::Local,
                package: None,
                path: Some("/nonexistent/path/skill.md".to_string()),
            }],
            workflow: IndexMap::new(),
        };

        let missing = verify_skill_files(&config);
        assert!(
            missing.contains(&"local-skill".to_string()),
            "expected local-skill to appear in missing, got {:?}",
            missing
        );
    }

    // Criterion 5: print_ready_summary produces the exact format
    #[test]
    fn ready_summary_format() {
        let plugin_statuses = vec![PluginStatus::AlreadyInstalled {
            name: "p1".to_string(),
        }];
        let mut output = Vec::new();
        print_ready_summary_to(
            true,
            true,
            Some("v2.x.x"),
            &plugin_statuses,
            &[],
            &mut output,
        );
        let text = String::from_utf8(output).unwrap();
        assert_eq!(
            text,
            "✓ Config valid\n✓ Environment ready\n✓ Claude Code found (v2.x.x)\n✓ Plugins installed\n✓ Skills found\nReady. Run: ywflow plan\n"
        );
    }

    // Criterion 6: run_plugin_setup accepts injected callbacks (fully tested by other tests)
    // Additional: blank answer → Skipped
    #[test]
    fn blank_answer_skipped() {
        let plugin = make_marketplace_plugin("another-plugin");

        let statuses = run_plugin_setup_with_io(
            &[plugin],
            &|| {
                Ok(InstalledPlugins {
                    names: HashSet::new(),
                })
            },
            &|_pkg| Ok(()),
            &mut std::io::Cursor::new(b"\n"),
            &mut std::io::sink(),
        );

        assert!(
            matches!(&statuses[0], PluginStatus::Skipped { .. }),
            "expected Skipped on blank answer, got {:?}",
            statuses[0]
        );
    }

    // installer returns Err → Failed
    #[test]
    fn installer_fails() {
        let plugin = make_marketplace_plugin("bad-plugin");

        let statuses = run_plugin_setup_with_io(
            &[plugin],
            &|| {
                Ok(InstalledPlugins {
                    names: HashSet::new(),
                })
            },
            &|_pkg| Err(anyhow::anyhow!("network error")),
            &mut std::io::Cursor::new(b"y\n"),
            &mut std::io::sink(),
        );

        assert!(
            matches!(&statuses[0], PluginStatus::Failed { name, error }
                if name == "bad-plugin" && error.contains("network error")),
            "expected Failed, got {:?}",
            statuses[0]
        );
    }
}
