use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum NvmError {
    #[error("nvm not found")]
    NotFound,

    #[error("Command failed: {stderr}")]
    CommandFailed { stderr: String },

    #[error("Failed to parse version: {0}")]
    ParseError(String),

    #[error("Installation failed: {0}")]
    InstallFailed(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Version not found: {0}")]
    VersionNotFound(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Timeout waiting for command")]
    Timeout,
}

impl From<std::io::Error> for NvmError {
    fn from(err: std::io::Error) -> Self {
        NvmError::IoError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::NvmError;

    #[test]
    fn io_error_conversion_maps_to_io_variant() {
        let mapped = NvmError::from(std::io::Error::other("disk full"));
        assert!(matches!(mapped, NvmError::IoError(msg) if msg.contains("disk full")));
    }

    #[test]
    fn parse_error_display_is_human_readable() {
        let error = NvmError::ParseError("invalid semver".to_string());
        assert_eq!(error.to_string(), "Failed to parse version: invalid semver");
    }
}
