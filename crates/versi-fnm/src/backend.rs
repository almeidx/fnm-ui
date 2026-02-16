use async_trait::async_trait;
use log::{debug, error, info, trace};
use std::path::PathBuf;
use tokio::process::Command;

use versi_core::HideWindow;

use versi_backend::{
    BackendError, BackendInfo, InstalledVersion, ManagerCapabilities, NodeVersion, RemoteVersion,
    ShellInitOptions, VersionManager,
};

use crate::version::{parse_installed_versions, parse_remote_versions};

#[derive(Debug, Clone)]
pub enum Environment {
    Native,
    Wsl { distro: String, fnm_path: String },
}

#[derive(Clone)]
pub struct FnmBackend {
    info: BackendInfo,
    fnm_dir: Option<PathBuf>,
    node_dist_mirror: Option<String>,
    environment: Environment,
}

impl FnmBackend {
    #[must_use]
    pub fn new(path: PathBuf, version: Option<String>, fnm_dir: Option<PathBuf>) -> Self {
        Self {
            info: BackendInfo {
                name: "fnm",
                path,
                version,
                data_dir: fnm_dir.clone(),
                in_path: true,
            },
            fnm_dir,
            node_dist_mirror: None,
            environment: Environment::Native,
        }
    }

    #[must_use]
    pub fn with_fnm_dir(mut self, dir: PathBuf) -> Self {
        self.fnm_dir = Some(dir.clone());
        self.info.data_dir = Some(dir);
        self
    }

    #[must_use]
    pub fn with_node_dist_mirror(mut self, mirror: String) -> Self {
        self.node_dist_mirror = Some(mirror);
        self
    }

    #[must_use]
    pub fn with_wsl(distro: String, fnm_path: String) -> Self {
        Self {
            info: BackendInfo {
                name: "fnm",
                path: PathBuf::from(&fnm_path),
                version: None,
                data_dir: None,
                in_path: true,
            },
            fnm_dir: None,
            node_dist_mirror: None,
            environment: Environment::Wsl { distro, fnm_path },
        }
    }

    fn build_command(&self, args: &[&str]) -> Command {
        match &self.environment {
            Environment::Native => {
                debug!(
                    "Building native fnm command: {} {}",
                    self.info.path.display(),
                    args.join(" ")
                );

                let mut cmd = Command::new(&self.info.path);
                cmd.args(args);

                if let Some(dir) = &self.fnm_dir {
                    debug!("Setting FNM_DIR={}", dir.display());
                    cmd.env("FNM_DIR", dir);
                }

                if let Some(mirror) = &self.node_dist_mirror {
                    debug!("Setting FNM_NODE_DIST_MIRROR={mirror}");
                    cmd.env("FNM_NODE_DIST_MIRROR", mirror);
                }

                cmd.hide_window();
                cmd
            }
            Environment::Wsl { distro, fnm_path } => {
                debug!(
                    "Building WSL fnm command: wsl.exe -d {} -- {} {}",
                    distro,
                    fnm_path,
                    args.join(" ")
                );

                let mut cmd = Command::new("wsl.exe");
                cmd.args(["-d", distro, "--", fnm_path]);
                cmd.args(args);
                cmd.hide_window();
                cmd
            }
        }
    }

    async fn execute(&self, args: &[&str]) -> Result<String, BackendError> {
        info!("Executing fnm command: {}", args.join(" "));

        let output = self.build_command(args).output().await?;

        debug!("fnm command exit status: {:?}", output.status);
        trace!("fnm stdout: {}", String::from_utf8_lossy(&output.stdout));

        if !output.stderr.is_empty() {
            trace!("fnm stderr: {}", String::from_utf8_lossy(&output.stderr));
        }

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            debug!("fnm command succeeded, output: {} bytes", stdout.len());
            Ok(stdout)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            error!("fnm command failed: args={args:?}, stderr='{stderr}'");
            Err(BackendError::CommandFailed { stderr })
        }
    }
}

#[async_trait]
impl VersionManager for FnmBackend {
    fn name(&self) -> &'static str {
        "fnm"
    }

    fn capabilities(&self) -> ManagerCapabilities {
        ManagerCapabilities {
            supports_lts_filter: true,
            supports_use_version: true,
            supports_shell_integration: true,
            supports_auto_switch: true,
            supports_corepack: true,
            supports_resolve_engines: true,
        }
    }

    fn backend_info(&self) -> &BackendInfo {
        &self.info
    }

    async fn list_installed(&self) -> Result<Vec<InstalledVersion>, BackendError> {
        let output = self.execute(&["list"]).await?;
        Ok(parse_installed_versions(&output))
    }

    async fn list_remote(&self) -> Result<Vec<RemoteVersion>, BackendError> {
        let output = self.execute(&["list-remote"]).await?;
        Ok(parse_remote_versions(&output))
    }

    async fn list_remote_lts(&self) -> Result<Vec<RemoteVersion>, BackendError> {
        let output = self.execute(&["list-remote", "--lts"]).await?;
        Ok(parse_remote_versions(&output))
    }

    async fn current_version(&self) -> Result<Option<NodeVersion>, BackendError> {
        let output = self.execute(&["current"]).await?;
        let output = output.trim();

        if output.is_empty() || output == "none" || output == "system" {
            return Ok(None);
        }

        output
            .parse()
            .map(Some)
            .map_err(|e: versi_backend::VersionParseError| BackendError::ParseError(e.to_string()))
    }

    async fn default_version(&self) -> Result<Option<NodeVersion>, BackendError> {
        let versions = self.list_installed().await?;
        Ok(versions
            .into_iter()
            .find(|v| v.is_default)
            .map(|v| v.version))
    }

    async fn install(&self, version: &str) -> Result<(), BackendError> {
        self.execute(&["install", version]).await?;
        Ok(())
    }

    async fn uninstall(&self, version: &str) -> Result<(), BackendError> {
        self.execute(&["uninstall", version]).await?;
        Ok(())
    }

    async fn set_default(&self, version: &str) -> Result<(), BackendError> {
        self.execute(&["default", version]).await?;
        Ok(())
    }

    async fn use_version(&self, version: &str) -> Result<(), BackendError> {
        self.execute(&["use", version]).await?;
        Ok(())
    }

    fn shell_init_command(&self, shell: &str, options: &ShellInitOptions) -> Option<String> {
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

        let flags_str = if flags.is_empty() {
            String::new()
        } else {
            format!(" {}", flags.join(" "))
        };

        match shell {
            "bash" | "zsh" => Some(format!("eval \"$(fnm env{flags_str})\"")),
            "fish" => Some(format!("fnm env{flags_str} | source")),
            "powershell" | "pwsh" => Some(format!(
                "fnm env{flags_str} | Out-String | Invoke-Expression"
            )),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use versi_backend::{ShellInitOptions, VersionManager};

    use super::FnmBackend;

    fn backend() -> FnmBackend {
        FnmBackend::new(PathBuf::from("fnm"), Some("1.38.0".to_string()), None)
    }

    #[test]
    fn capabilities_enable_fnm_supported_features() {
        let capabilities = backend().capabilities();

        assert!(capabilities.supports_lts_filter);
        assert!(capabilities.supports_use_version);
        assert!(capabilities.supports_shell_integration);
        assert!(capabilities.supports_auto_switch);
        assert!(capabilities.supports_corepack);
        assert!(capabilities.supports_resolve_engines);
    }

    #[test]
    fn shell_init_command_builds_bash_flags() {
        let options = ShellInitOptions {
            use_on_cd: true,
            resolve_engines: true,
            corepack_enabled: false,
        };

        let command = backend()
            .shell_init_command("bash", &options)
            .expect("bash init command should be supported");

        assert_eq!(
            command,
            "eval \"$(fnm env --use-on-cd --resolve-engines)\""
        );
    }

    #[test]
    fn shell_init_command_builds_fish_command() {
        let options = ShellInitOptions {
            use_on_cd: false,
            resolve_engines: false,
            corepack_enabled: true,
        };

        let command = backend()
            .shell_init_command("fish", &options)
            .expect("fish init command should be supported");

        assert_eq!(command, "fnm env --corepack-enabled | source");
    }

    #[test]
    fn shell_init_command_returns_none_for_unknown_shell() {
        let options = ShellInitOptions::default();

        assert!(backend().shell_init_command("nu", &options).is_none());
    }
}
