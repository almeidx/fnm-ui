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
