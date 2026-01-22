use std::process::Command;
use thiserror::Error;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

trait HideWindow {
    fn hide_window(&mut self) -> &mut Self;
}

impl HideWindow for Command {
    #[cfg(windows)]
    fn hide_window(&mut self) -> &mut Self {
        self.creation_flags(CREATE_NO_WINDOW)
    }

    #[cfg(not(windows))]
    fn hide_window(&mut self) -> &mut Self {
        self
    }
}

impl HideWindow for tokio::process::Command {
    #[cfg(windows)]
    fn hide_window(&mut self) -> &mut Self {
        self.creation_flags(CREATE_NO_WINDOW)
    }

    #[cfg(not(windows))]
    fn hide_window(&mut self) -> &mut Self {
        self
    }
}

#[derive(Debug, Clone)]
pub struct WslDistro {
    pub name: String,
    pub is_default: bool,
    pub version: u8,
    pub fnm_path: Option<String>,
}

#[derive(Error, Debug)]
pub enum WslError {
    #[error("WSL not available")]
    NotAvailable,

    #[error("Command failed: {stderr}")]
    CommandFailed { stderr: String },

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub fn detect_wsl_distros() -> Vec<WslDistro> {
    // Use --list --running to only get distros that are already running
    // This avoids accidentally starting WSL
    let output = Command::new("wsl.exe")
        .args(["--list", "--running", "--verbose"])
        .hide_window()
        .output();

    match output {
        Ok(output) if output.status.success() => {
            // wsl.exe outputs UTF-16LE on Windows
            let stdout = decode_wsl_output(&output.stdout);
            let mut distros = parse_wsl_list(&stdout);

            for distro in &mut distros {
                distro.fnm_path = find_fnm_path(&distro.name);
            }

            distros
        }
        _ => Vec::new(),
    }
}

fn find_fnm_path(distro: &str) -> Option<String> {
    // Check common fnm installation locations directly
    let common_paths = [
        "$HOME/.local/share/fnm/fnm",
        "$HOME/.cargo/bin/fnm",
        "/usr/local/bin/fnm",
        "/usr/bin/fnm",
        "$HOME/.fnm/fnm",
    ];

    // Build a command that checks each path and returns the first one that exists
    let check_cmd = common_paths
        .iter()
        .map(|p| format!("[ -x {} ] && echo {}", p, p))
        .collect::<Vec<_>>()
        .join(" || ");

    let output = Command::new("wsl.exe")
        .args(["-d", distro, "--", "sh", "-c", &check_cmd])
        .hide_window()
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(path);
            }
        }
        _ => {}
    }

    None
}

fn decode_wsl_output(bytes: &[u8]) -> String {
    // Try UTF-16LE first (Windows wsl.exe output)
    if bytes.len() >= 2 {
        let u16_iter = bytes
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]));
        let decoded: String = char::decode_utf16(u16_iter)
            .filter_map(|r| r.ok())
            .collect();
        if !decoded.is_empty() && decoded.chars().any(|c| c.is_alphabetic()) {
            return decoded;
        }
    }
    // Fallback to UTF-8
    String::from_utf8_lossy(bytes).to_string()
}

fn parse_wsl_list(output: &str) -> Vec<WslDistro> {
    output
        .lines()
        .skip(1)
        .filter_map(|line| {
            let line = line.trim().replace('\0', "");
            if line.is_empty() {
                return None;
            }

            let is_default = line.starts_with('*');
            let line = line.trim_start_matches('*').trim();

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                Some(WslDistro {
                    name: parts[0].to_string(),
                    is_default,
                    version: parts[2].parse().unwrap_or(2),
                    fnm_path: None,
                })
            } else if !parts.is_empty() {
                Some(WslDistro {
                    name: parts[0].to_string(),
                    is_default,
                    version: 2,
                    fnm_path: None,
                })
            } else {
                None
            }
        })
        .collect()
}

pub async fn execute_in_wsl(distro: &str, command: &str) -> Result<String, WslError> {
    let output = tokio::process::Command::new("wsl.exe")
        .args(["-d", distro, "--", "bash", "-c", command])
        .hide_window()
        .output()
        .await?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(WslError::CommandFailed {
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}

#[allow(dead_code)]
pub async fn check_fnm_in_wsl(distro: &str) -> bool {
    execute_in_wsl(distro, "which fnm")
        .await
        .map(|output| !output.trim().is_empty())
        .unwrap_or(false)
}
