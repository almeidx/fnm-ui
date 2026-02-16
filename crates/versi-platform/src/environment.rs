use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EnvironmentId {
    Native,
    Wsl {
        distro: String,
        backend_path: String,
    },
}

impl EnvironmentId {
    #[must_use]
    pub fn display_name(&self) -> String {
        match self {
            EnvironmentId::Native => {
                #[cfg(target_os = "macos")]
                {
                    "macOS".to_string()
                }
                #[cfg(target_os = "windows")]
                {
                    "Windows".to_string()
                }
                #[cfg(all(unix, not(target_os = "macos")))]
                {
                    "Linux".to_string()
                }
            }
            EnvironmentId::Wsl { distro, .. } => format!("WSL: {distro}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Environment {
    pub id: EnvironmentId,
    pub name: String,
    pub enabled: bool,
}

impl Environment {
    #[must_use]
    pub fn native() -> Self {
        Self {
            id: EnvironmentId::Native,
            name: EnvironmentId::Native.display_name(),
            enabled: true,
        }
    }

    #[must_use]
    pub fn wsl(distro: String, backend_path: String) -> Self {
        let name = format!("WSL: {distro}");
        let id = EnvironmentId::Wsl {
            distro,
            backend_path,
        };
        Self {
            name,
            id,
            enabled: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Environment, EnvironmentId};

    fn native_label() -> &'static str {
        if cfg!(target_os = "macos") {
            "macOS"
        } else if cfg!(target_os = "windows") {
            "Windows"
        } else {
            "Linux"
        }
    }

    #[test]
    fn native_display_name_matches_platform() {
        assert_eq!(EnvironmentId::Native.display_name(), native_label());
    }

    #[test]
    fn wsl_display_name_uses_distro_name() {
        let id = EnvironmentId::Wsl {
            distro: "Ubuntu".to_string(),
            backend_path: "/home/user/.local/share/fnm/fnm".to_string(),
        };

        assert_eq!(id.display_name(), "WSL: Ubuntu");
    }

    #[test]
    fn native_environment_defaults_to_enabled() {
        let env = Environment::native();

        assert_eq!(env.id, EnvironmentId::Native);
        assert_eq!(env.name, native_label());
        assert!(env.enabled);
    }

    #[test]
    fn wsl_environment_sets_expected_fields() {
        let env = Environment::wsl("Debian".to_string(), "/usr/bin/fnm".to_string());

        assert_eq!(env.name, "WSL: Debian");
        assert!(env.enabled);
        assert_eq!(
            env.id,
            EnvironmentId::Wsl {
                distro: "Debian".to_string(),
                backend_path: "/usr/bin/fnm".to_string(),
            }
        );
    }
}
