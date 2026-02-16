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
