use versi_backend::{BackendError, BackendUpdate};
use versi_core::{GitHubRelease, is_newer_version};

const FNM_GITHUB_REPO: &str = "Schniz/fnm";

fn backend_update_from_release(
    release: GitHubRelease,
    current_version: &str,
) -> Option<BackendUpdate> {
    let latest = release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name);
    let current = current_version.strip_prefix('v').unwrap_or(current_version);

    if is_newer_version(latest, current) {
        Some(BackendUpdate {
            current_version: current.to_string(),
            latest_version: latest.to_string(),
            release_url: release.html_url,
        })
    } else {
        None
    }
}

pub async fn check_for_fnm_update(
    client: &reqwest::Client,
    current_version: &str,
) -> Result<Option<BackendUpdate>, BackendError> {
    let url = format!("https://api.github.com/repos/{FNM_GITHUB_REPO}/releases/latest");

    let response = client
        .get(&url)
        .header("User-Agent", "versi")
        .send()
        .await
        .map_err(|e| BackendError::NetworkError(e.to_string()))?;

    if !response.status().is_success() {
        return Ok(None);
    }

    let release: GitHubRelease = response
        .json()
        .await
        .map_err(|e| BackendError::NetworkError(e.to_string()))?;

    Ok(backend_update_from_release(release, current_version))
}

#[cfg(test)]
mod tests {
    use super::{GitHubRelease, backend_update_from_release};

    fn release(tag_name: &str) -> GitHubRelease {
        GitHubRelease {
            tag_name: tag_name.to_string(),
            html_url: "https://github.com/Schniz/fnm/releases/tag/v1.0.0".to_string(),
            body: None,
            assets: Vec::new(),
        }
    }

    #[test]
    fn returns_update_when_release_is_newer() {
        let update = backend_update_from_release(release("v1.38.0"), "v1.37.1")
            .expect("newer release should produce update metadata");

        assert_eq!(update.current_version, "1.37.1");
        assert_eq!(update.latest_version, "1.38.0");
        assert_eq!(
            update.release_url,
            "https://github.com/Schniz/fnm/releases/tag/v1.0.0"
        );
    }

    #[test]
    fn returns_none_when_release_is_not_newer() {
        assert!(backend_update_from_release(release("v1.38.0"), "1.38.0").is_none());
        assert!(backend_update_from_release(release("v1.37.0"), "v1.38.0").is_none());
    }
}
