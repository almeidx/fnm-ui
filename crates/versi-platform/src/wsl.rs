use log::{debug, error, info, trace, warn};
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
    info!("Detecting WSL distros...");
    debug!("Running: wsl.exe --list --running --verbose");

    let output = Command::new("wsl.exe")
        .args(["--list", "--running", "--verbose"])
        .hide_window()
        .output();

    match output {
        Ok(output) => {
            debug!("wsl.exe exit status: {:?}", output.status);
            trace!("wsl.exe stdout raw bytes: {:?}", &output.stdout);
            trace!(
                "wsl.exe stderr: {:?}",
                String::from_utf8_lossy(&output.stderr)
            );

            if output.status.success() {
                let stdout = decode_wsl_output(&output.stdout);
                debug!("Decoded WSL output:\n{}", stdout);

                let mut distros = parse_wsl_list(&stdout);
                info!("Found {} WSL distros before fnm detection", distros.len());

                for distro in &mut distros {
                    debug!("Checking for fnm in distro: {}", distro.name);
                    distro.fnm_path = find_fnm_path(&distro.name);
                    if let Some(ref path) = distro.fnm_path {
                        info!("Found fnm in {}: {}", distro.name, path);
                    } else {
                        warn!("fnm not found in distro: {}", distro.name);
                    }
                }

                let with_fnm: Vec<_> = distros.iter().filter(|d| d.fnm_path.is_some()).collect();
                info!(
                    "WSL detection complete: {} distros with fnm out of {} total",
                    with_fnm.len(),
                    distros.len()
                );
                distros
            } else {
                warn!(
                    "wsl.exe command failed with status: {:?}, stderr: {}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr)
                );
                Vec::new()
            }
        }
        Err(e) => {
            error!("Failed to execute wsl.exe: {}", e);
            Vec::new()
        }
    }
}

fn find_fnm_path(distro: &str) -> Option<String> {
    let common_paths = [
        "$HOME/.local/share/fnm/fnm",
        "$HOME/.cargo/bin/fnm",
        "/usr/local/bin/fnm",
        "/usr/bin/fnm",
        "$HOME/.fnm/fnm",
    ];

    let check_cmd = common_paths
        .iter()
        .map(|p| format!("[ -x {} ] && {{ echo {}; exit 0; }}", p, p))
        .collect::<Vec<_>>()
        .join("; ");

    debug!(
        "Running fnm path detection for {}: wsl.exe -d {} -- sh -c \"{}\"",
        distro, distro, check_cmd
    );

    let output = Command::new("wsl.exe")
        .args(["-d", distro, "--", "sh", "-c", &check_cmd])
        .hide_window()
        .output();

    match output {
        Ok(output) => {
            debug!(
                "fnm path detection for {} - exit status: {:?}",
                distro, output.status
            );
            trace!(
                "fnm path detection stdout: {:?}",
                String::from_utf8_lossy(&output.stdout)
            );
            trace!(
                "fnm path detection stderr: {:?}",
                String::from_utf8_lossy(&output.stderr)
            );

            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .next()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty());

                if let Some(ref p) = path {
                    debug!("fnm found at: {}", p);
                    return path;
                }
                debug!("fnm path detection returned empty output");
            } else {
                warn!(
                    "fnm path detection failed for {}: {}",
                    distro,
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
        Err(e) => {
            error!("Failed to run fnm path detection for {}: {}", distro, e);
        }
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
    debug!(
        "Executing in WSL {}: wsl.exe -d {} -- bash -c \"{}\"",
        distro, distro, command
    );

    let output = tokio::process::Command::new("wsl.exe")
        .args(["-d", distro, "--", "bash", "-c", command])
        .hide_window()
        .output()
        .await?;

    debug!("WSL command exit status: {:?}", output.status);
    trace!(
        "WSL command stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    trace!(
        "WSL command stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        debug!(
            "WSL command succeeded, output length: {} bytes",
            stdout.len()
        );
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        error!(
            "WSL command failed in {}: command='{}', stderr='{}'",
            distro, command, stderr
        );
        Err(WslError::CommandFailed { stderr })
    }
}

#[allow(dead_code)]
pub async fn check_fnm_in_wsl(distro: &str) -> bool {
    execute_in_wsl(distro, "which fnm")
        .await
        .map(|output| !output.trim().is_empty())
        .unwrap_or(false)
}
