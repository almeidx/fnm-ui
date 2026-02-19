use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum BackendError {
    #[error("Backend not found")]
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

    #[error("IO error ({kind}): {message}")]
    IoError {
        kind: std::io::ErrorKind,
        message: String,
    },

    #[error("Operation not supported by this backend: {0}")]
    Unsupported(String),

    #[error("Backend-specific error: {0}")]
    BackendSpecific(String),

    #[error("Timeout waiting for command")]
    Timeout,
}

impl From<std::io::Error> for BackendError {
    fn from(err: std::io::Error) -> Self {
        BackendError::IoError {
            kind: err.kind(),
            message: err.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BackendError;

    #[test]
    fn io_error_conversion_maps_to_io_variant() {
        let mapped = BackendError::from(std::io::Error::other("permission denied"));
        assert!(
            matches!(mapped, BackendError::IoError { kind, ref message } if kind == std::io::ErrorKind::Other && message.contains("permission denied"))
        );
    }

    #[test]
    fn command_failed_display_includes_stderr() {
        let error = BackendError::CommandFailed {
            stderr: "nvm: command not found".to_string(),
        };

        assert_eq!(error.to_string(), "Command failed: nvm: command not found");
    }
}
