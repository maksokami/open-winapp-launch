use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

const MAX_CONFIG_SIZE: u64 = 1024 * 1024;
const DEFAULT_CONFIG: &str = r#"settings:
  hotkey: "Ctrl+Alt+E"
groups:
  - name: Tools
    apps:
      - name: Notepad
        path: "%SystemRoot%\\System32\\notepad.exe"
      - name: Calculator
        path: "%SystemRoot%\\System32\\calc.exe"
  - name: Dev
    apps: []
"#;

pub struct Config {
    pub settings: Settings,
    pub groups: Vec<Group>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    #[serde(default)]
    settings: Settings,
    #[serde(default)]
    groups: Vec<Group>,
    #[serde(default)]
    apps: Vec<App>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    #[serde(default = "default_hotkey")]
    pub hotkey: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: default_hotkey(),
        }
    }
}

fn default_hotkey() -> String {
    "Ctrl+Alt+E".to_owned()
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Group {
    pub name: String,
    #[serde(default)]
    pub apps: Vec<App>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct App {
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub workdir: Option<String>,
    pub icon: Option<String>,
    #[serde(default)]
    pub group: Option<String>,
}

pub fn config_path() -> Result<PathBuf> {
    let root =
        dirs::config_dir().context("could not locate the per-user configuration directory")?;
    Ok(root.join("AppLauncher").join("config.yaml"))
}

pub fn ensure_config(path: &Path) -> Result<()> {
    if path.exists() {
        return Ok(());
    }
    let parent = path
        .parent()
        .context("configuration path has no parent directory")?;
    fs::create_dir_all(parent).with_context(|| format!("could not create {}", parent.display()))?;
    fs::write(path, DEFAULT_CONFIG).with_context(|| format!("could not write {}", path.display()))
}

pub fn load(path: &Path) -> Result<Config> {
    let metadata =
        fs::metadata(path).with_context(|| format!("could not read {}", path.display()))?;
    if metadata.len() > MAX_CONFIG_SIZE {
        bail!("configuration exceeds the 1 MiB size limit");
    }
    let contents =
        fs::read_to_string(path).with_context(|| format!("could not read {}", path.display()))?;
    parse(&contents)
}

pub fn parse(contents: &str) -> Result<Config> {
    if contents.len() as u64 > MAX_CONFIG_SIZE {
        bail!("configuration exceeds the 1 MiB size limit");
    }
    let raw: RawConfig = serde_yaml_ng::from_str(contents).context("invalid YAML configuration")?;
    if !raw.groups.is_empty() && !raw.apps.is_empty() {
        bail!("use either top-level groups or top-level apps, not both");
    }
    let mut config = Config {
        settings: raw.settings,
        groups: raw.groups,
    };
    if !raw.apps.is_empty() {
        let mut group_indices = HashMap::new();
        for mut app in raw.apps {
            let group_name = app
                .group
                .take()
                .unwrap_or_else(|| "Other".to_owned())
                .trim()
                .to_owned();
            if group_name.is_empty() {
                bail!("app group names must not be empty (app: {})", app.name);
            }
            let index = *group_indices
                .entry(group_name.to_lowercase())
                .or_insert_with(|| {
                    config.groups.push(Group {
                        name: group_name.clone(),
                        apps: Vec::new(),
                    });
                    config.groups.len() - 1
                });
            config.groups[index].apps.push(app);
        }
    }
    let mut groups = HashSet::new();
    for group in &mut config.groups {
        group.name = group.name.trim().to_owned();
        if group.name.is_empty() {
            bail!("group names must not be empty");
        }
        if !groups.insert(group.name.to_lowercase()) {
            bail!("duplicate group name: {}", group.name);
        }
        for app in &mut group.apps {
            if app.group.is_some() {
                bail!(
                    "app group field is only valid with top-level apps (app: {})",
                    app.name
                );
            }
            app.name = app.name.trim().to_owned();
            if app.name.is_empty() {
                bail!("app names must not be empty (group: {})", group.name);
            }
            if app.path.trim().is_empty() {
                bail!("app paths must not be empty (app: {})", app.name);
            }
            app.group = None;
            app.path = expand_path(&app.path);
            crate::security::validate_executable(&app.path)
                .with_context(|| format!("invalid executable path for {}", app.name))?;
            if let Some(workdir) = &mut app.workdir {
                *workdir = expand_path(workdir);
                crate::security::validate_local_path(workdir)
                    .with_context(|| format!("invalid working directory for {}", app.name))?;
            }
            if let Some(icon) = &mut app.icon {
                *icon = expand_path(icon);
                crate::security::validate_local_path(icon)
                    .with_context(|| format!("invalid icon path for {}", app.name))?;
            }
        }
    }
    Ok(config)
}

fn expand_path(value: &str) -> String {
    let mut expanded = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(start) = rest.find('%') {
        expanded.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = after.find('%') else {
            expanded.push('%');
            rest = after;
            break;
        };
        let key = &after[..end];
        if let Ok(value) = std::env::var(key) {
            expanded.push_str(&value);
        } else {
            expanded.push('%');
            expanded.push_str(key);
            expanded.push('%');
        }
        rest = &after[end + 1..];
    }
    expanded.push_str(rest);
    expanded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_group_and_app_order_and_defaults() {
        let config = parse("groups:\n  - name: First\n    apps:\n      - name: A\n        path: C:\\Apps\\a.exe\n      - name: B\n        path: C:\\Apps\\b.exe\n  - name: Second\n    apps: []\n").unwrap();
        assert_eq!(config.groups[0].name, "First");
        assert_eq!(config.groups[0].apps[1].name, "B");
        assert_eq!(config.settings.hotkey, "Ctrl+Alt+E");
    }

    #[test]
    fn rejects_unknown_fields_and_duplicate_groups() {
        assert!(parse("groups: []\nunknown: true\n").is_err());
        assert!(parse("groups:\n  - name: Dev\n  - name: dev\n").is_err());
    }

    #[test]
    fn accepts_missing_paths_for_placeholder_rendering() {
        let config = parse("groups:\n  - name: Apps\n    apps:\n      - name: Missing\n        path: Z:\\missing.exe\n").unwrap();
        assert!(config.groups[0].apps[0].path.contains("missing.exe"));
    }

    #[test]
    fn rejects_blank_app_paths() {
        assert!(
            parse(
                "groups:\n  - name: Apps\n    apps:\n      - name: Missing\n        path: '  '\n"
            )
            .is_err()
        );
    }

    #[test]
    fn groups_flat_apps_in_first_seen_order() {
        let config = parse(
            "apps:\n  - name: Editor\n    path: C:\\Apps\\editor.exe\n    group: Dev\n  - name: Notepad\n    path: C:\\Apps\\notepad.exe\n    group: Tools\n  - name: Terminal\n    path: C:\\Apps\\terminal.exe\n    group: Dev\n",
        )
        .unwrap();
        assert_eq!(config.groups[0].name, "Dev");
        assert_eq!(config.groups[0].apps[1].name, "Terminal");
        assert_eq!(config.groups[1].name, "Tools");
    }

    #[test]
    fn rejects_mixed_grouped_and_flat_apps() {
        assert!(
            parse(
                "groups:\n  - name: Dev\n    apps: []\napps:\n  - name: Editor\n    path: editor.exe\n"
            )
            .is_err()
        );
    }

    #[test]
    fn rejects_unsafe_paths_before_loading_icons_or_launching() {
        for field in [
            "path: calc.exe",
            "path: C:\\Apps\\app.cmd",
            "path: \\\\server\\share\\app.exe",
            "path: C:\\Apps\\app.exe\n    workdir: .",
            "path: C:\\Apps\\app.exe\n    icon: \\\\server\\share\\icon.png",
        ] {
            assert!(parse(&format!("apps:\n  - name: Unsafe\n    {field}\n")).is_err());
        }
    }
}
