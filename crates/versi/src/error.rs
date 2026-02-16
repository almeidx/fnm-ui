#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    Message(String),
    Timeout { operation: &'static str, seconds: u64 },
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
