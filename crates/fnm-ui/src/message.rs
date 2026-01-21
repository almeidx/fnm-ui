use std::path::PathBuf;

use fnm_core::{
    AppUpdate, InstallProgress, InstalledVersion, NodeVersion, ReleaseSchedule, RemoteVersion,
};
use fnm_platform::EnvironmentId;
use fnm_shell::ShellType;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Message {
    NoOp,
    Initialized(InitResult),

    EnvironmentSelected(usize),
    EnvironmentLoaded {
        env_id: EnvironmentId,
        versions: Vec<InstalledVersion>,
        default_version: Option<NodeVersion>,
    },
    EnvironmentError {
        env_id: EnvironmentId,
        error: String,
    },
    RefreshEnvironment,

    VersionGroupToggled {
        major: u32,
    },
    SearchChanged(String),

    FetchRemoteVersions,
    RemoteVersionsFetched(Result<Vec<RemoteVersion>, String>),
    ReleaseScheduleFetched(Result<ReleaseSchedule, String>),

    OpenInstallModal,
    CloseModal,
    InstallModalSearchChanged(String),
    OpenChangelog(String),
    SelectVersionToInstall(String),
    StartInstall(String),
    InstallProgress {
        version: String,
        progress: InstallProgress,
    },
    InstallComplete {
        version: String,
        success: bool,
        error: Option<String>,
    },

    RequestUninstall(String),
    ConfirmUninstall(String),
    CancelUninstall,
    UninstallComplete {
        version: String,
        success: bool,
        error: Option<String>,
    },

    SetDefault(String),
    DefaultChanged {
        version: String,
        previous: Option<String>,
        success: bool,
        error: Option<String>,
    },

    ToastTimeout(usize),
    ToastDismiss(usize),
    ToastUndo(usize),

    OpenSettings,
    CloseSettings,
    ThemeChanged(crate::settings::ThemeSetting),
    ShellOptionUseOnCdToggled(bool),
    ShellOptionResolveEnginesToggled(bool),
    ShellOptionCorepackEnabledToggled(bool),
    CheckShellSetup,
    ShellSetupChecked(Vec<(ShellType, fnm_shell::VerificationResult)>),
    ConfigureShell(ShellType),
    ShellConfigured(ShellType, Result<(), String>),

    OnboardingNext,
    OnboardingBack,
    OnboardingInstallFnm,
    OnboardingFnmInstallResult(Result<(), String>),
    OnboardingConfigureShell(ShellType),
    OnboardingShellConfigResult(Result<(), String>),
    OnboardingSelectVersion(String),
    OnboardingComplete,

    Tick,
    WindowEvent(iced::window::Event),

    CheckForAppUpdate,
    AppUpdateChecked(Option<AppUpdate>),
    OpenAppUpdate,
    DismissAppUpdate,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct InitResult {
    pub fnm_found: bool,
    pub fnm_path: Option<PathBuf>,
    pub fnm_dir: Option<PathBuf>,
    pub fnm_version: Option<String>,
    pub environments: Vec<EnvironmentId>,
}
