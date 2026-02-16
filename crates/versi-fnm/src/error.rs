use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum FnmError {
    #[error("fnm not found")]
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

impl From<std::io::Error> for FnmError {
    fn from(err: std::io::Error) -> Self {
        FnmError::IoError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::FnmError;

    #[test]
    fn io_error_conversion_maps_to_io_variant() {
        let io_error = std::io::Error::other("disk full");

        let mapped = FnmError::from(io_error);

        assert!(matches!(mapped, FnmError::IoError(msg) if msg.contains("disk full")));
    }

    #[test]
    fn install_failed_error_formats_message() {
        let error = FnmError::InstallFailed("script exited 1".to_string());

        assert_eq!(error.to_string(), "Installation failed: script exited 1");
    }
}
