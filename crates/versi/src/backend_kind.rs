use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackendKind {
    Fnm,
    Nvm,
}

impl BackendKind {
    pub const DEFAULT: Self = Self::Fnm;

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Fnm => "fnm",
            Self::Nvm => "nvm",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "fnm" => Some(Self::Fnm),
            "nvm" => Some(Self::Nvm),
            _ => None,
        }
    }
}

impl std::fmt::Display for BackendKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::BackendKind;

    #[test]
    fn default_backend_is_fnm() {
        assert_eq!(BackendKind::DEFAULT, BackendKind::Fnm);
    }

    #[test]
    fn as_str_matches_expected_backend_names() {
        assert_eq!(BackendKind::Fnm.as_str(), "fnm");
        assert_eq!(BackendKind::Nvm.as_str(), "nvm");
    }

    #[test]
    fn from_name_accepts_known_backend_names() {
        assert_eq!(BackendKind::from_name("fnm"), Some(BackendKind::Fnm));
        assert_eq!(BackendKind::from_name("nvm"), Some(BackendKind::Nvm));
        assert_eq!(BackendKind::from_name("FNM"), None);
    }

    #[test]
    fn display_outputs_backend_name() {
        assert_eq!(BackendKind::Fnm.to_string(), "fnm");
        assert_eq!(BackendKind::Nvm.to_string(), "nvm");
    }
}
