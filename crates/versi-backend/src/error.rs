use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum BackendError {
    #[error("Backend not found")]
    NotFound,

    #[error("Command failed: {stderr}")]
    CommandFailed { stderr: String },

    #[error(transparent)]
    ParseError(#[from] crate::types::VersionParseError),

    #[error("Installation failed during {phase}: {details}")]
    InstallFailed {
        phase: &'static str,
        details: String,
    },

    #[error("Network error during {operation} ({stage}): {details}")]
    NetworkError {
        operation: &'static str,
        stage: NetworkStage,
        details: String,
    },

    #[error("Version not found: {version}")]
    VersionNotFound { version: String },

    #[error("IO error ({kind}): {message}")]
    IoError {
        kind: std::io::ErrorKind,
        message: String,
    },

    #[error("Operation not supported by this backend: {operation}")]
    Unsupported { operation: &'static str },

    #[error("Backend-specific error in {context}: {details}")]
    BackendSpecific {
        context: &'static str,
        details: String,
    },

    #[error("Timeout waiting for command")]
    Timeout,
}

#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkStage {
    #[error("request")]
    Request,
    #[error("response parse")]
    ResponseParse,
}

impl BackendError {
    pub fn install_failed(phase: &'static str, details: impl Into<String>) -> Self {
        Self::InstallFailed {
            phase,
            details: details.into(),
        }
    }

    pub fn network_request(operation: &'static str, details: impl Into<String>) -> Self {
        Self::NetworkError {
            operation,
            stage: NetworkStage::Request,
            details: details.into(),
        }
    }

    pub fn network_request_from<E>(operation: &'static str, error: E) -> Self
    where
        E: std::fmt::Display,
    {
        Self::network_request(operation, error.to_string())
    }

    pub fn network_parse(operation: &'static str, details: impl Into<String>) -> Self {
        Self::NetworkError {
            operation,
            stage: NetworkStage::ResponseParse,
            details: details.into(),
        }
    }

    pub fn network_parse_from<E>(operation: &'static str, error: E) -> Self
    where
        E: std::fmt::Display,
    {
        Self::network_parse(operation, error.to_string())
    }
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
    use super::{BackendError, NetworkStage};

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

    #[test]
    fn network_helpers_set_expected_stage() {
        let request = BackendError::network_request("check update", "timed out");
        assert!(matches!(
            request,
            BackendError::NetworkError {
                operation: "check update",
                stage: NetworkStage::Request,
                ..
            }
        ));

        let parse = BackendError::network_parse("check update", "invalid json");
        assert!(matches!(
            parse,
            BackendError::NetworkError {
                operation: "check update",
                stage: NetworkStage::ResponseParse,
                ..
            }
        ));
    }
}
