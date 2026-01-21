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
        let init_patterns = [
            "fnm env",
            "eval \"$(fnm",
            "fnm env --use-on-cd",
            "Invoke-Expression",
        ];

        init_patterns
            .iter()
            .any(|pattern| self.content.contains(pattern))
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
            return ShellConfigEdit {
                original: self.content.clone(),
                modified: self.content.clone(),
                changes: Vec::new(),
            };
        }

        let comment = match self.shell_type {
            ShellType::Fish => "# fnm (Fast Node Manager)",
            ShellType::PowerShell => "# fnm (Fast Node Manager)",
            _ => "# fnm (Fast Node Manager)",
        };

        let addition = format!("\n{}\n{}\n", comment, init_command);
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

    pub fn update_fnm_init(&mut self, options: &FnmShellOptions) -> ShellConfigEdit {
        let new_init_command = self.shell_type.fnm_init_command(options);

        if !self.has_fnm_init() {
            return self.add_fnm_init(options);
        }

        let mut modified = self.content.clone();
        let mut changes = Vec::new();

        // Pattern to match fnm env commands with various flag combinations
        // Matches: eval "$(fnm env ...)" or eval "`fnm env ...`" or fnm env ... | source
        let patterns = [
            r#"eval "\$\(fnm env[^)]*\)""#,
            r#"eval "`fnm env[^`]*`""#,
            r"fnm env[^\n]*\| source",
            r"fnm env[^\n]*\| Out-String \| Invoke-Expression",
        ];

        for pattern in patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if re.is_match(&modified) {
                    modified = re.replace(&modified, new_init_command.as_str()).to_string();
                    changes.push(format!("Updated fnm initialization to: {}", new_init_command));
                    break;
                }
            }
        }

        ShellConfigEdit {
            original: self.content.clone(),
            modified,
            changes,
        }
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
