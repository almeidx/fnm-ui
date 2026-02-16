use crate::detect::ShellType;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;
use thiserror::Error;
use versi_backend::ShellInitOptions;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Config file not found: {0}")]
    FileNotFound(PathBuf),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Shell type does not support config files")]
    UnsupportedShell,
}

pub struct ShellConfig {
    pub shell_type: ShellType,
    pub config_path: PathBuf,
    pub content: String,
}

impl ShellConfig {
    pub fn load(shell_type: ShellType, config_path: PathBuf) -> Result<Self, ConfigError> {
        let content = if config_path.exists() {
            fs::read_to_string(&config_path)?
        } else {
            String::new()
        };

        Ok(Self {
            shell_type,
            config_path,
            content,
        })
    }

    #[must_use]
    pub fn has_init(&self, marker: &str) -> bool {
        self.content.contains(marker)
    }

    #[must_use]
    pub fn detect_options(&self, marker: &str) -> Option<ShellInitOptions> {
        if !self.has_init(marker) {
            return None;
        }

        Some(ShellInitOptions {
            use_on_cd: self.content.contains("--use-on-cd"),
            resolve_engines: self.content.contains("--resolve-engines"),
            corepack_enabled: self.content.contains("--corepack-enabled"),
        })
    }

    pub fn add_init(&mut self, init_command: &str, label: &str) -> ShellConfigEdit {
        let addition = format!("\n# {label}\n{init_command}\n");
        let modified = format!("{}{}", self.content, addition);

        ShellConfigEdit {
            original: self.content.clone(),
            modified,
            changes: vec![format!("Add initialization: {}", init_command)],
        }
    }

    pub fn update_flags(&mut self, marker: &str, options: &ShellInitOptions) -> ShellConfigEdit {
        if !self.has_init(marker) {
            return ShellConfigEdit {
                original: self.content.clone(),
                modified: self.content.clone(),
                changes: vec![],
            };
        }

        let mut modified = self.content.clone();
        let mut changes = Vec::new();

        let flags = [
            ("--use-on-cd", options.use_on_cd),
            ("--resolve-engines", options.resolve_engines),
            ("--corepack-enabled", options.corepack_enabled),
        ];

        for (flag, enabled) in flags {
            let has_flag = modified.contains(flag);

            if enabled && !has_flag {
                modified = Self::add_flag_to_init(&modified, marker, flag);
                changes.push(format!("Added {flag}"));
            } else if !enabled && has_flag {
                modified = Self::remove_flag_from_init(&modified, marker, flag);
                changes.push(format!("Removed {flag}"));
            }
        }

        ShellConfigEdit {
            original: self.content.clone(),
            modified,
            changes,
        }
    }

    pub fn apply_edit(&mut self, edit: &ShellConfigEdit) -> Result<(), ConfigError> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&self.config_path, &edit.modified)?;
        self.content.clone_from(&edit.modified);

        Ok(())
    }

    fn add_flag_to_init(content: &str, marker: &str, flag: &str) -> String {
        let mut result = String::new();
        for line in content.lines() {
            if line.contains(marker) && !line.contains(flag) {
                let modified_line = line.replacen(marker, &format!("{marker} {flag}"), 1);
                result.push_str(&modified_line);
            } else {
                result.push_str(line);
            }
            result.push('\n');
        }
        if !content.ends_with('\n') && result.ends_with('\n') {
            result.pop();
        }
        result
    }

    fn remove_flag_from_init(content: &str, marker: &str, flag: &str) -> String {
        let mut result = String::new();
        for line in content.lines() {
            if line.contains(marker) && line.contains(flag) {
                let modified_line = line
                    .replace(&format!("{flag} "), "")
                    .replace(&format!(" {flag}"), "")
                    .replace(flag, "");
                result.push_str(&modified_line);
            } else {
                result.push_str(line);
            }
            result.push('\n');
        }
        if !content.ends_with('\n') && result.ends_with('\n') {
            result.pop();
        }
        result
    }
}

pub struct ShellConfigEdit {
    pub original: String,
    pub modified: String,
    pub changes: Vec<String>,
}

impl ShellConfigEdit {
    #[must_use]
    pub fn has_changes(&self) -> bool {
        !self.changes.is_empty()
    }

    #[must_use]
    pub fn diff_preview(&self) -> String {
        if !self.has_changes() {
            return "No changes needed.".to_string();
        }

        let mut preview = String::new();

        for change in &self.changes {
            let _ = writeln!(preview, "+ {change}");
        }

        preview
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config(content: &str) -> ShellConfig {
        ShellConfig {
            shell_type: ShellType::Bash,
            config_path: PathBuf::from("/test/.bashrc"),
            content: content.to_string(),
        }
    }

    #[test]
    fn test_has_init_true() {
        let config = create_test_config(r#"eval "$(fnm env --shell bash)""#);
        assert!(config.has_init("fnm env"));
    }

    #[test]
    fn test_has_init_false() {
        let config = create_test_config("export PATH=$PATH:/usr/bin");
        assert!(!config.has_init("fnm env"));
    }

    #[test]
    fn test_has_init_empty() {
        let config = create_test_config("");
        assert!(!config.has_init("fnm env"));
    }

    #[test]
    fn test_detect_options_all_flags() {
        let config = create_test_config(
            r#"eval "$(fnm env --use-on-cd --resolve-engines --corepack-enabled --shell bash)""#,
        );
        let options = config.detect_options("fnm env").unwrap();
        assert!(options.use_on_cd);
        assert!(options.resolve_engines);
        assert!(options.corepack_enabled);
    }

    #[test]
    fn test_detect_options_no_flags() {
        let config = create_test_config(r#"eval "$(fnm env --shell bash)""#);
        let options = config.detect_options("fnm env").unwrap();
        assert!(!options.use_on_cd);
        assert!(!options.resolve_engines);
        assert!(!options.corepack_enabled);
    }

    #[test]
    fn test_detect_options_partial_flags() {
        let config = create_test_config(r#"eval "$(fnm env --use-on-cd --shell bash)""#);
        let options = config.detect_options("fnm env").unwrap();
        assert!(options.use_on_cd);
        assert!(!options.resolve_engines);
        assert!(!options.corepack_enabled);
    }

    #[test]
    fn test_detect_options_no_marker() {
        let config = create_test_config("export PATH=$PATH");
        assert!(config.detect_options("fnm env").is_none());
    }

    #[test]
    fn test_add_init() {
        let mut config = create_test_config("# My bashrc\nexport PATH=$PATH");
        let edit = config.add_init(
            r#"eval "$(fnm env --shell bash)""#,
            "fnm (Fast Node Manager)",
        );

        assert!(edit.has_changes());
        assert!(edit.modified.contains("fnm env"));
        assert!(edit.modified.contains("# fnm (Fast Node Manager)"));
    }

    #[test]
    fn test_add_flag_to_init() {
        let content = r#"eval "$(fnm env --shell bash)""#;
        let result = ShellConfig::add_flag_to_init(content, "fnm env", "--use-on-cd");
        assert!(result.contains("fnm env --use-on-cd"));
    }

    #[test]
    fn test_add_flag_preserves_existing() {
        let content = r#"eval "$(fnm env --use-on-cd --shell bash)""#;
        let result = ShellConfig::add_flag_to_init(content, "fnm env", "--resolve-engines");
        assert!(result.contains("--use-on-cd"));
        assert!(result.contains("--resolve-engines"));
    }

    #[test]
    fn test_remove_flag_from_init() {
        let content = r#"eval "$(fnm env --use-on-cd --shell bash)""#;
        let result = ShellConfig::remove_flag_from_init(content, "fnm env", "--use-on-cd");
        assert!(!result.contains("--use-on-cd"));
        assert!(result.contains("fnm env"));
    }

    #[test]
    fn test_remove_flag_preserves_others() {
        let content = r#"eval "$(fnm env --use-on-cd --resolve-engines --shell bash)""#;
        let result = ShellConfig::remove_flag_from_init(content, "fnm env", "--use-on-cd");
        assert!(!result.contains("--use-on-cd"));
        assert!(result.contains("--resolve-engines"));
    }

    #[test]
    fn test_update_flags_add() {
        let mut config = create_test_config(r#"eval "$(fnm env --shell bash)""#);
        let options = ShellInitOptions {
            use_on_cd: true,
            resolve_engines: false,
            corepack_enabled: false,
        };
        let edit = config.update_flags("fnm env", &options);

        assert!(edit.has_changes());
        assert!(edit.modified.contains("--use-on-cd"));
    }

    #[test]
    fn test_update_flags_remove() {
        let mut config = create_test_config(r#"eval "$(fnm env --use-on-cd --shell bash)""#);
        let options = ShellInitOptions {
            use_on_cd: false,
            resolve_engines: false,
            corepack_enabled: false,
        };
        let edit = config.update_flags("fnm env", &options);

        assert!(edit.has_changes());
        assert!(!edit.modified.contains("--use-on-cd"));
    }

    #[test]
    fn test_update_flags_no_change() {
        let mut config = create_test_config(r#"eval "$(fnm env --use-on-cd --shell bash)""#);
        let options = ShellInitOptions {
            use_on_cd: true,
            resolve_engines: false,
            corepack_enabled: false,
        };
        let edit = config.update_flags("fnm env", &options);

        assert!(!edit.has_changes());
    }

    #[test]
    fn test_shell_config_edit_has_changes() {
        let edit = ShellConfigEdit {
            original: String::new(),
            modified: "new".to_string(),
            changes: vec!["Added something".to_string()],
        };
        assert!(edit.has_changes());
    }

    #[test]
    fn test_shell_config_edit_no_changes() {
        let edit = ShellConfigEdit {
            original: "same".to_string(),
            modified: "same".to_string(),
            changes: vec![],
        };
        assert!(!edit.has_changes());
    }

    #[test]
    fn test_diff_preview_with_changes() {
        let edit = ShellConfigEdit {
            original: String::new(),
            modified: "new".to_string(),
            changes: vec!["Added fnm".to_string()],
        };
        let preview = edit.diff_preview();
        assert!(preview.contains("+ Added fnm"));
    }

    #[test]
    fn test_diff_preview_no_changes() {
        let edit = ShellConfigEdit {
            original: String::new(),
            modified: String::new(),
            changes: vec![],
        };
        let preview = edit.diff_preview();
        assert_eq!(preview, "No changes needed.");
    }
}
