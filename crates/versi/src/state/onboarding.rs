use std::path::PathBuf;

use versi_shell::ShellType;

use crate::backend_kind::BackendKind;
use crate::error::AppError;

#[derive(Debug)]
pub struct OnboardingState {
    pub step: OnboardingStep,
    pub backend_installing: bool,
    pub install_error: Option<AppError>,
    pub detected_shells: Vec<ShellConfigStatus>,
    pub available_backends: Vec<BackendOption>,
    pub selected_backend: Option<BackendKind>,
}

impl OnboardingState {
    pub fn new() -> Self {
        Self {
            step: OnboardingStep::Welcome,
            backend_installing: false,
            install_error: None,
            detected_shells: Vec::new(),
            available_backends: Vec::new(),
            selected_backend: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OnboardingStep {
    Welcome,
    SelectBackend,
    InstallBackend,
    ConfigureShell,
}

#[derive(Debug, Clone)]
pub struct ShellConfigStatus {
    pub shell_type: ShellType,
    pub shell_name: String,
    pub configured: bool,
    pub config_path: Option<PathBuf>,
    pub configuring: bool,
    pub error: Option<AppError>,
}

#[derive(Debug, Clone)]
pub struct BackendOption {
    pub kind: BackendKind,
    pub display_name: &'static str,
    pub detected: bool,
}
