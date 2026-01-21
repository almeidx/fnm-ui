use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use which::which;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShellType {
    Bash,
    Zsh,
    Fish,
    PowerShell,
    Cmd,
}

impl ShellType {
    pub fn name(&self) -> &'static str {
        match self {
            ShellType::Bash => "Bash",
            ShellType::Zsh => "Zsh",
            ShellType::Fish => "Fish",
            ShellType::PowerShell => "PowerShell",
            ShellType::Cmd => "Command Prompt",
        }
    }

    pub fn fnm_shell_arg(&self) -> &'static str {
        match self {
            ShellType::Bash => "bash",
            ShellType::Zsh => "zsh",
            ShellType::Fish => "fish",
            ShellType::PowerShell => "powershell",
            ShellType::Cmd => "cmd",
        }
    }

    pub fn config_files(&self) -> Vec<PathBuf> {
        let home = dirs::home_dir().unwrap_or_default();

        match self {
            ShellType::Bash => vec![
                home.join(".bashrc"),
                home.join(".bash_profile"),
                home.join(".profile"),
            ],
            ShellType::Zsh => vec![home.join(".zshrc"), home.join(".zprofile")],
            ShellType::Fish => vec![home.join(".config/fish/config.fish")],
            ShellType::PowerShell => {
                #[cfg(target_os = "windows")]
                {
                    if let Some(docs) = dirs::document_dir() {
                        vec![
                            docs.join("PowerShell/Microsoft.PowerShell_profile.ps1"),
                            docs.join("WindowsPowerShell/Microsoft.PowerShell_profile.ps1"),
                        ]
                    } else {
                        vec![]
                    }
                }
                #[cfg(not(target_os = "windows"))]
                {
                    vec![home.join(".config/powershell/Microsoft.PowerShell_profile.ps1")]
                }
            }
            ShellType::Cmd => vec![],
        }
    }

    pub fn fnm_init_command(&self, options: &FnmShellOptions) -> String {
        let mut flags = Vec::new();

        if options.use_on_cd {
            flags.push("--use-on-cd");
        }
        if options.resolve_engines {
            flags.push("--resolve-engines");
        }
        if options.corepack_enabled {
            flags.push("--corepack-enabled");
        }

        let flags_str = flags.join(" ");

        match self {
            ShellType::Bash => {
                if flags_str.is_empty() {
                    r#"eval "$(fnm env --shell bash)""#.to_string()
                } else {
                    format!(r#"eval "$(fnm env {} --shell bash)""#, flags_str)
                }
            }
            ShellType::Zsh => {
                if flags_str.is_empty() {
                    r#"eval "$(fnm env --shell zsh)""#.to_string()
                } else {
                    format!(r#"eval "$(fnm env {} --shell zsh)""#, flags_str)
                }
            }
            ShellType::Fish => {
                if flags_str.is_empty() {
                    "fnm env --shell fish | source".to_string()
                } else {
                    format!("fnm env {} --shell fish | source", flags_str)
                }
            }
            ShellType::PowerShell => {
                if flags_str.is_empty() {
                    "fnm env --shell powershell | Out-String | Invoke-Expression".to_string()
                } else {
                    format!(
                        "fnm env {} --shell powershell | Out-String | Invoke-Expression",
                        flags_str
                    )
                }
            }
            ShellType::Cmd => String::new(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct FnmShellOptions {
    pub use_on_cd: bool,
    pub resolve_engines: bool,
    pub corepack_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct ShellInfo {
    pub shell_type: ShellType,
    pub path: Option<PathBuf>,
    pub config_file: Option<PathBuf>,
    pub is_configured: bool,
}

pub fn detect_shells() -> Vec<ShellInfo> {
    let mut shells = Vec::new();

    #[cfg(unix)]
    {
        if let Ok(path) = which("bash") {
            let config_file = find_existing_config(&ShellType::Bash);
            shells.push(ShellInfo {
                shell_type: ShellType::Bash,
                path: Some(path),
                config_file,
                is_configured: false,
            });
        }

        if let Ok(path) = which("zsh") {
            let config_file = find_existing_config(&ShellType::Zsh);
            shells.push(ShellInfo {
                shell_type: ShellType::Zsh,
                path: Some(path),
                config_file,
                is_configured: false,
            });
        }

        if let Ok(path) = which("fish") {
            let config_file = find_existing_config(&ShellType::Fish);
            shells.push(ShellInfo {
                shell_type: ShellType::Fish,
                path: Some(path),
                config_file,
                is_configured: false,
            });
        }
    }

    #[cfg(target_os = "windows")]
    {
        if which("pwsh").is_ok() || which("powershell").is_ok() {
            let config_file = find_existing_config(&ShellType::PowerShell);
            shells.push(ShellInfo {
                shell_type: ShellType::PowerShell,
                path: which("pwsh").ok().or_else(|| which("powershell").ok()),
                config_file,
                is_configured: false,
            });
        }

        shells.push(ShellInfo {
            shell_type: ShellType::Cmd,
            path: Some(PathBuf::from("cmd.exe")),
            config_file: None,
            is_configured: false,
        });
    }

    shells
}

fn find_existing_config(shell: &ShellType) -> Option<PathBuf> {
    shell.config_files().into_iter().find(|path| path.exists())
}
