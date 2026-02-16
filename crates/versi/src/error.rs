#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    Message(String),
    Timeout {
        operation: &'static str,
        seconds: u64,
    },
    ShellConfigPathNotFound {
        shell: &'static str,
    },
    ShellNotSupported {
        shell: &'static str,
    },
    ShellConfigFailed {
        shell: &'static str,
        action: &'static str,
        details: String,
    },
    BackendInstallFailed {
        backend: &'static str,
        details: String,
    },
}

impl AppError {
    pub fn message(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }

    pub fn timeout(operation: &'static str, seconds: u64) -> Self {
        Self::Timeout { operation, seconds }
    }

    pub fn shell_config_path_not_found(shell: &'static str) -> Self {
        Self::ShellConfigPathNotFound { shell }
    }

    pub fn shell_not_supported(shell: &'static str) -> Self {
        Self::ShellNotSupported { shell }
    }

    pub fn shell_config_failed(
        shell: &'static str,
        action: &'static str,
        details: impl Into<String>,
    ) -> Self {
        Self::ShellConfigFailed {
            shell,
            action,
            details: details.into(),
        }
    }

    pub fn backend_install_failed(backend: &'static str, details: impl Into<String>) -> Self {
        Self::BackendInstallFailed {
            backend,
            details: details.into(),
        }
    }
}

impl From<String> for AppError {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}

impl From<&str> for AppError {
    fn from(value: &str) -> Self {
        Self::Message(value.to_string())
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Message(message) => write!(f, "{message}"),
            Self::Timeout { operation, seconds } => {
                write!(f, "{operation} timed out after {seconds}s")
            }
            Self::ShellConfigPathNotFound { shell } => {
                write!(f, "No shell config file path found for {shell}")
            }
            Self::ShellNotSupported { shell } => write!(f, "{shell} shell is not supported"),
            Self::ShellConfigFailed {
                shell,
                action,
                details,
            } => write!(f, "{shell} shell {action} failed: {details}"),
            Self::BackendInstallFailed { backend, details } => {
                write!(f, "Failed to install backend {backend}: {details}")
            }
        }
    }
}

impl std::error::Error for AppError {}

#[cfg(test)]
mod tests {
    use super::AppError;

    #[test]
    fn message_constructor_and_display_match() {
        let error = AppError::message("something failed");
        assert_eq!(error, AppError::Message("something failed".to_string()));
        assert_eq!(error.to_string(), "something failed");
    }

    #[test]
    fn timeout_constructor_and_display_match() {
        let error = AppError::timeout("install", 30);
        assert_eq!(
            error,
            AppError::Timeout {
                operation: "install",
                seconds: 30
            }
        );
        assert_eq!(error.to_string(), "install timed out after 30s");
    }

    #[test]
    fn string_conversions_build_message_variant() {
        assert_eq!(
            AppError::from("from str"),
            AppError::Message("from str".to_string())
        );
        assert_eq!(
            AppError::from("from string".to_string()),
            AppError::Message("from string".to_string())
        );
    }

    #[test]
    fn shell_error_constructors_include_context() {
        let missing = AppError::shell_config_path_not_found("Bash");
        let unsupported = AppError::shell_not_supported("Fish");
        let failed = AppError::shell_config_failed("Zsh", "load config", "permission denied");

        assert_eq!(missing, AppError::ShellConfigPathNotFound { shell: "Bash" });
        assert_eq!(unsupported, AppError::ShellNotSupported { shell: "Fish" });
        assert_eq!(
            failed,
            AppError::ShellConfigFailed {
                shell: "Zsh",
                action: "load config",
                details: "permission denied".to_string()
            }
        );
        assert_eq!(
            missing.to_string(),
            "No shell config file path found for Bash"
        );
        assert_eq!(unsupported.to_string(), "Fish shell is not supported");
        assert_eq!(
            failed.to_string(),
            "Zsh shell load config failed: permission denied"
        );
    }

    #[test]
    fn backend_install_failed_constructor_includes_backend_name() {
        let error = AppError::backend_install_failed("fnm", "network unavailable");

        assert_eq!(
            error,
            AppError::BackendInstallFailed {
                backend: "fnm",
                details: "network unavailable".to_string()
            }
        );
        assert_eq!(
            error.to_string(),
            "Failed to install backend fnm: network unavailable"
        );
    }
}
