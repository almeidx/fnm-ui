mod config;
mod detect;
mod verify;

pub mod shells;

pub use config::{ShellConfig, ShellConfigEdit};
pub use detect::{FnmShellOptions, ShellInfo, ShellType, detect_shells};
pub use verify::{VerificationResult, get_or_create_config_path, verify_shell_config};
