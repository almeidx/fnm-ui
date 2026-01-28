use crate::detect::{FnmShellOptions, ShellType};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

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

    pub fn has_fnm_init(&self) -> bool {
        self.content.contains("fnm env")
    }

    pub fn detect_fnm_options(&self) -> Option<FnmShellOptions> {
        if !self.has_fnm_init() {
            return None;
        }

        Some(FnmShellOptions {
            use_on_cd: self.content.contains("--use-on-cd"),
            resolve_engines: self.content.contains("--resolve-engines"),
            corepack_enabled: self.content.contains("--corepack-enabled"),
        })
    }

    pub fn add_fnm_init(&mut self, options: &FnmShellOptions) -> ShellConfigEdit {
        let init_command = self.shell_type.fnm_init_command(options);

        if self.has_fnm_init() {
            return self.update_fnm_flags(options);
        }

        let addition = format!("\n# fnm (Fast Node Manager)\n{}\n", init_command);
        let modified = format!("{}{}", self.content, addition);

        ShellConfigEdit {
            original: self.content.clone(),
            modified,
            changes: vec![format!("Add fnm initialization: {}", init_command)],
        }
    }

    pub fn apply_edit(&mut self, edit: &ShellConfigEdit) -> Result<(), ConfigError> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&self.config_path, &edit.modified)?;
        self.content = edit.modified.clone();

        Ok(())
    }

    pub fn update_fnm_flags(&mut self, options: &FnmShellOptions) -> ShellConfigEdit {
        if !self.has_fnm_init() {
            return self.add_fnm_init(options);
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
                modified = Self::add_flag_to_fnm_env(&modified, flag);
                changes.push(format!("Added {}", flag));
            } else if !enabled && has_flag {
                modified = Self::remove_flag_from_fnm_env(&modified, flag);
                changes.push(format!("Removed {}", flag));
            }
        }

        ShellConfigEdit {
            original: self.content.clone(),
            modified,
            changes,
        }
    }

    fn add_flag_to_fnm_env(content: &str, flag: &str) -> String {
        let mut result = String::new();
        for line in content.lines() {
            if line.contains("fnm env") && !line.contains(flag) {
                let modified_line = line.replacen("fnm env", &format!("fnm env {}", flag), 1);
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

    fn remove_flag_from_fnm_env(content: &str, flag: &str) -> String {
        let mut result = String::new();
        for line in content.lines() {
            if line.contains("fnm env") && line.contains(flag) {
                let modified_line = line
                    .replace(&format!("{} ", flag), "")
                    .replace(&format!(" {}", flag), "")
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
    pub fn has_changes(&self) -> bool {
        !self.changes.is_empty()
    }

    pub fn diff_preview(&self) -> String {
        if !self.has_changes() {
            return "No changes needed.".to_string();
        }

        let mut preview = String::new();

        for change in &self.changes {
            preview.push_str(&format!("+ {}\n", change));
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
    fn test_has_fnm_init_true() {
        let config = create_test_config(r#"eval "$(fnm env --shell bash)""#);
        assert!(config.has_fnm_init());
    }

    #[test]
    fn test_has_fnm_init_false() {
        let config = create_test_config("export PATH=$PATH:/usr/bin");
        assert!(!config.has_fnm_init());
    }

    #[test]
    fn test_has_fnm_init_empty() {
        let config = create_test_config("");
        assert!(!config.has_fnm_init());
    }

    #[test]
    fn test_detect_fnm_options_all_flags() {
        let config = create_test_config(
            r#"eval "$(fnm env --use-on-cd --resolve-engines --corepack-enabled --shell bash)""#,
        );
        let options = config.detect_fnm_options().unwrap();
        assert!(options.use_on_cd);
        assert!(options.resolve_engines);
        assert!(options.corepack_enabled);
    }

    #[test]
    fn test_detect_fnm_options_no_flags() {
        let config = create_test_config(r#"eval "$(fnm env --shell bash)""#);
        let options = config.detect_fnm_options().unwrap();
        assert!(!options.use_on_cd);
        assert!(!options.resolve_engines);
        assert!(!options.corepack_enabled);
    }

    #[test]
    fn test_detect_fnm_options_partial_flags() {
        let config = create_test_config(r#"eval "$(fnm env --use-on-cd --shell bash)""#);
        let options = config.detect_fnm_options().unwrap();
        assert!(options.use_on_cd);
        assert!(!options.resolve_engines);
        assert!(!options.corepack_enabled);
    }

    #[test]
    fn test_detect_fnm_options_no_fnm() {
        let config = create_test_config("export PATH=$PATH");
        assert!(config.detect_fnm_options().is_none());
    }

    #[test]
    fn test_add_fnm_init() {
        let mut config = create_test_config("# My bashrc\nexport PATH=$PATH");
        let options = FnmShellOptions::default();
        let edit = config.add_fnm_init(&options);

        assert!(edit.has_changes());
        assert!(edit.modified.contains("fnm env"));
        assert!(edit.modified.contains("# fnm (Fast Node Manager)"));
    }

    #[test]
    fn test_add_fnm_init_with_flags() {
        let mut config = create_test_config("");
        let options = FnmShellOptions {
            use_on_cd: true,
            resolve_engines: false,
            corepack_enabled: false,
        };
        let edit = config.add_fnm_init(&options);

        assert!(edit.modified.contains("--use-on-cd"));
    }

    #[test]
    fn test_add_flag_to_fnm_env() {
        let content = r#"eval "$(fnm env --shell bash)""#;
        let result = ShellConfig::add_flag_to_fnm_env(content, "--use-on-cd");
        assert!(result.contains("fnm env --use-on-cd"));
    }

    #[test]
    fn test_add_flag_preserves_existing() {
        let content = r#"eval "$(fnm env --use-on-cd --shell bash)""#;
        let result = ShellConfig::add_flag_to_fnm_env(content, "--resolve-engines");
        assert!(result.contains("--use-on-cd"));
        assert!(result.contains("--resolve-engines"));
    }

    #[test]
    fn test_remove_flag_from_fnm_env() {
        let content = r#"eval "$(fnm env --use-on-cd --shell bash)""#;
        let result = ShellConfig::remove_flag_from_fnm_env(content, "--use-on-cd");
        assert!(!result.contains("--use-on-cd"));
        assert!(result.contains("fnm env"));
    }

    #[test]
    fn test_remove_flag_preserves_others() {
        let content = r#"eval "$(fnm env --use-on-cd --resolve-engines --shell bash)""#;
        let result = ShellConfig::remove_flag_from_fnm_env(content, "--use-on-cd");
        assert!(!result.contains("--use-on-cd"));
        assert!(result.contains("--resolve-engines"));
    }

    #[test]
    fn test_update_fnm_flags_add() {
        let mut config = create_test_config(r#"eval "$(fnm env --shell bash)""#);
        let options = FnmShellOptions {
            use_on_cd: true,
            resolve_engines: false,
            corepack_enabled: false,
        };
        let edit = config.update_fnm_flags(&options);

        assert!(edit.has_changes());
        assert!(edit.modified.contains("--use-on-cd"));
    }

    #[test]
    fn test_update_fnm_flags_remove() {
        let mut config = create_test_config(r#"eval "$(fnm env --use-on-cd --shell bash)""#);
        let options = FnmShellOptions {
            use_on_cd: false,
            resolve_engines: false,
            corepack_enabled: false,
        };
        let edit = config.update_fnm_flags(&options);

        assert!(edit.has_changes());
        assert!(!edit.modified.contains("--use-on-cd"));
    }

    #[test]
    fn test_update_fnm_flags_no_change() {
        let mut config = create_test_config(r#"eval "$(fnm env --use-on-cd --shell bash)""#);
        let options = FnmShellOptions {
            use_on_cd: true,
            resolve_engines: false,
            corepack_enabled: false,
        };
        let edit = config.update_fnm_flags(&options);

        assert!(!edit.has_changes());
    }

    #[test]
    fn test_shell_config_edit_has_changes() {
        let edit = ShellConfigEdit {
            original: "".to_string(),
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
            original: "".to_string(),
            modified: "new".to_string(),
            changes: vec!["Added fnm".to_string()],
        };
        let preview = edit.diff_preview();
        assert!(preview.contains("+ Added fnm"));
    }

    #[test]
    fn test_diff_preview_no_changes() {
        let edit = ShellConfigEdit {
            original: "".to_string(),
            modified: "".to_string(),
            changes: vec![],
        };
        let preview = edit.diff_preview();
        assert_eq!(preview, "No changes needed.");
    }
}
