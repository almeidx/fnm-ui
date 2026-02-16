#![allow(clippy::missing_errors_doc)]

mod config;
mod detect;
mod verify;

pub mod shells;

pub use config::{ShellConfig, ShellConfigEdit};
pub use detect::{ShellInfo, ShellType, detect_native_shells, detect_shells, detect_wsl_shells};
pub use verify::{
    VerificationResult, configure_wsl_shell_config, get_or_create_config_path, verify_shell_config,
    verify_wsl_shell_config,
};
pub use versi_backend::ShellInitOptions;
