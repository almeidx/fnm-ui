use versi_backend::{BackendError, BackendUpdate};
use versi_core::{GitHubRelease, is_newer_version};

use crate::detection::NvmVariant;

const NVM_UNIX_REPO: &str = "nvm-sh/nvm";
const NVM_WINDOWS_REPO: &str = "coreybutler/nvm-windows";

pub async fn check_for_nvm_update(
    client: &reqwest::Client,
    current_version: &str,
    variant: &NvmVariant,
) -> Result<Option<BackendUpdate>, BackendError> {
    let repo = match variant {
        NvmVariant::Unix | NvmVariant::NotFound => NVM_UNIX_REPO,
        NvmVariant::Windows => NVM_WINDOWS_REPO,
    };

    let url = format!("https://api.github.com/repos/{repo}/releases/latest");

    let response = client
        .get(&url)
        .header("User-Agent", "versi")
        .send()
        .await
        .map_err(|error| BackendError::network_request_from("nvm update check", error))?;

    if !response.status().is_success() {
        return Ok(None);
    }

    let release: GitHubRelease = response
        .json()
        .await
        .map_err(|error| BackendError::network_parse_from("nvm update check", error))?;

    let latest = release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name);
    let current = current_version.strip_prefix('v').unwrap_or(current_version);

    if is_newer_version(latest, current) {
        Ok(Some(BackendUpdate {
            current_version: current.to_string(),
            latest_version: latest.to_string(),
            release_url: release.html_url,
        }))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use versi_core::is_newer_version;

    #[test]
    fn newer_version_returns_true() {
        assert!(is_newer_version("1.0.1", "1.0.0"));
        assert!(is_newer_version("2.0.0", "1.9.9"));
        assert!(is_newer_version("1.1.0", "1.0.9"));
    }

    #[test]
    fn older_version_returns_false() {
        assert!(!is_newer_version("1.0.0", "1.0.1"));
        assert!(!is_newer_version("1.9.9", "2.0.0"));
    }

    #[test]
    fn same_version_returns_false() {
        assert!(!is_newer_version("1.0.0", "1.0.0"));
        assert!(!is_newer_version("0.40.1", "0.40.1"));
    }

    #[test]
    fn two_part_versions() {
        assert!(is_newer_version("1.2", "1.1"));
        assert!(!is_newer_version("1.1", "1.2"));
        assert!(!is_newer_version("1.1", "1.1"));
    }

    #[test]
    fn one_part_versions() {
        assert!(is_newer_version("2", "1"));
        assert!(!is_newer_version("1", "2"));
        assert!(!is_newer_version("1", "1"));
    }

    #[test]
    fn v_prefix_not_stripped_by_function() {
        assert!(is_newer_version("v2.0.0", "v1.0.0"));
    }
}
