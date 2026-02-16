#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    Message(String),
    Timeout {
        operation: &'static str,
        seconds: u64,
    },
}

impl AppError {
    pub fn message(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }

    pub fn timeout(operation: &'static str, seconds: u64) -> Self {
        Self::Timeout { operation, seconds }
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
}
