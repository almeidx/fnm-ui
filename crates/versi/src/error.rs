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
    SettingsDialogCancelled,
    SettingsExportFailed {
        action: &'static str,
        details: String,
    },
    SettingsImportFailed {
        action: &'static str,
        details: String,
    },
    OperationFailed {
        operation: &'static str,
        details: String,
    },
    OperationCancelled {
        operation: &'static str,
    },
    EnvironmentLoadFailed {
        details: String,
    },
    VersionFetchFailed {
        resource: &'static str,
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

    pub fn settings_dialog_cancelled() -> Self {
        Self::SettingsDialogCancelled
    }

    pub fn settings_export_failed(action: &'static str, details: impl Into<String>) -> Self {
        Self::SettingsExportFailed {
            action,
            details: details.into(),
        }
    }

    pub fn settings_import_failed(action: &'static str, details: impl Into<String>) -> Self {
        Self::SettingsImportFailed {
            action,
            details: details.into(),
        }
    }

    pub fn operation_failed(operation: &'static str, details: impl Into<String>) -> Self {
        Self::OperationFailed {
            operation,
            details: details.into(),
        }
    }

    pub fn operation_cancelled(operation: &'static str) -> Self {
        Self::OperationCancelled { operation }
    }

    pub fn environment_load_failed(details: impl Into<String>) -> Self {
        Self::EnvironmentLoadFailed {
            details: details.into(),
        }
    }

    pub fn version_fetch_failed(resource: &'static str, details: impl Into<String>) -> Self {
        Self::VersionFetchFailed {
            resource,
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
            Self::SettingsDialogCancelled => write!(f, "Cancelled"),
            Self::SettingsExportFailed { action, details } => {
                write!(f, "Settings export {action} failed: {details}")
            }
            Self::SettingsImportFailed { action, details } => {
                write!(f, "Settings import {action} failed: {details}")
            }
            Self::OperationFailed { operation, details } => {
                write!(f, "{operation} failed: {details}")
            }
            Self::OperationCancelled { operation } => write!(f, "{operation} cancelled"),
            Self::EnvironmentLoadFailed { details } => {
                write!(f, "Failed to load versions: {details}")
            }
            Self::VersionFetchFailed { resource, details } => {
                write!(f, "{resource} fetch failed: {details}")
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

    #[test]
    fn settings_and_cancellation_constructors_are_structured() {
        let cancelled = AppError::settings_dialog_cancelled();
        let export = AppError::settings_export_failed("write file", "permission denied");
        let import = AppError::settings_import_failed("parse json", "invalid type");
        let op_failed = AppError::operation_failed("Install", "backend reported failure");
        let op_cancelled = AppError::operation_cancelled("Remote versions fetch");

        assert_eq!(cancelled, AppError::SettingsDialogCancelled);
        assert_eq!(
            export,
            AppError::SettingsExportFailed {
                action: "write file",
                details: "permission denied".to_string()
            }
        );
        assert_eq!(
            import,
            AppError::SettingsImportFailed {
                action: "parse json",
                details: "invalid type".to_string()
            }
        );
        assert_eq!(
            op_failed,
            AppError::OperationFailed {
                operation: "Install",
                details: "backend reported failure".to_string()
            }
        );
        assert_eq!(
            op_cancelled,
            AppError::OperationCancelled {
                operation: "Remote versions fetch"
            }
        );
        assert_eq!(cancelled.to_string(), "Cancelled");
        assert_eq!(
            export.to_string(),
            "Settings export write file failed: permission denied"
        );
        assert_eq!(
            import.to_string(),
            "Settings import parse json failed: invalid type"
        );
        assert_eq!(
            op_failed.to_string(),
            "Install failed: backend reported failure"
        );
        assert_eq!(op_cancelled.to_string(), "Remote versions fetch cancelled");
    }

    #[test]
    fn fetch_and_environment_error_constructors_include_context() {
        let env_load = AppError::environment_load_failed("backend unavailable");
        let fetch = AppError::version_fetch_failed("Release schedule", "network timeout");

        assert_eq!(
            env_load,
            AppError::EnvironmentLoadFailed {
                details: "backend unavailable".to_string()
            }
        );
        assert_eq!(
            fetch,
            AppError::VersionFetchFailed {
                resource: "Release schedule",
                details: "network timeout".to_string()
            }
        );
        assert_eq!(
            env_load.to_string(),
            "Failed to load versions: backend unavailable"
        );
        assert_eq!(
            fetch.to_string(),
            "Release schedule fetch failed: network timeout"
        );
    }
}
