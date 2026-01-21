use std::path::PathBuf;
use tokio::process::Command;
use which::which;

#[derive(Debug, Clone)]
pub struct FnmDetection {
    pub found: bool,
    pub path: Option<PathBuf>,
    pub version: Option<String>,
    pub in_path: bool,
}

pub async fn detect_fnm() -> FnmDetection {
    if let Ok(path) = which("fnm") {
        let version = get_fnm_version(&path).await;
        return FnmDetection {
            found: true,
            path: Some(path),
            version,
            in_path: true,
        };
    }

    let common_paths = get_common_fnm_paths();

    for path in common_paths {
        if path.exists() {
            let version = get_fnm_version(&path).await;
            return FnmDetection {
                found: true,
                path: Some(path),
                version,
                in_path: false,
            };
        }
    }

    FnmDetection {
        found: false,
        path: None,
        version: None,
        in_path: false,
    }
}

fn get_common_fnm_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".fnm").join("fnm"));
        paths.push(home.join(".local").join("bin").join("fnm"));

        #[cfg(target_os = "macos")]
        {
            paths.push(PathBuf::from("/opt/homebrew/bin/fnm"));
            paths.push(PathBuf::from("/usr/local/bin/fnm"));
        }

        #[cfg(target_os = "windows")]
        {
            if let Some(local_app_data) = dirs::data_local_dir() {
                paths.push(local_app_data.join("fnm").join("fnm.exe"));
            }
        }
    }

    paths
}

async fn get_fnm_version(path: &PathBuf) -> Option<String> {
    let output = Command::new(path).arg("--version").output().await.ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let version = stdout
        .trim()
        .strip_prefix("fnm ")
        .unwrap_or(stdout.trim())
        .to_string();

    Some(version)
}

pub async fn install_fnm() -> Result<(), crate::FnmError> {
    #[cfg(unix)]
    {
        let status = Command::new("bash")
            .args(["-c", "curl -fsSL https://fnm.vercel.app/install | bash"])
            .status()
            .await?;

        if status.success() {
            Ok(())
        } else {
            Err(crate::FnmError::InstallFailed(
                "fnm installation script failed".to_string(),
            ))
        }
    }

    #[cfg(windows)]
    {
        let status = Command::new("powershell")
            .args(["-Command", "irm https://fnm.vercel.app/install | iex"])
            .status()
            .await?;

        if status.success() {
            Ok(())
        } else {
            Err(crate::FnmError::InstallFailed(
                "fnm installation script failed".to_string(),
            ))
        }
    }
}

pub async fn check_fnm_update(current_version: &str) -> Option<String> {
    let output = Command::new("curl")
        .args([
            "-fsSL",
            "https://api.github.com/repos/Schniz/fnm/releases/latest",
        ])
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;

    let latest_version = json["tag_name"].as_str()?;
    let latest_version = latest_version.strip_prefix('v').unwrap_or(latest_version);
    let current = current_version.strip_prefix('v').unwrap_or(current_version);

    if latest_version != current {
        Some(latest_version.to_string())
    } else {
        None
    }
}
