use std::path::PathBuf;

use versi_shell::ShellType;

#[derive(Debug)]
pub struct OnboardingState {
    pub step: OnboardingStep,
    pub fnm_installing: bool,
    pub install_error: Option<String>,
    pub detected_shells: Vec<ShellConfigStatus>,
}

impl OnboardingState {
    pub fn new() -> Self {
        Self {
            step: OnboardingStep::Welcome,
            fnm_installing: false,
            install_error: None,
            detected_shells: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OnboardingStep {
    Welcome,
    InstallFnm,
    ConfigureShell,
}

#[derive(Debug, Clone)]
pub struct ShellConfigStatus {
    pub shell_type: ShellType,
    pub shell_name: String,
    pub configured: bool,
    pub config_path: Option<PathBuf>,
    pub configuring: bool,
    pub error: Option<String>,
}
