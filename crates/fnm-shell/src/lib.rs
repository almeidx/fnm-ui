mod config;
mod detect;
mod verify;

pub mod shells;

pub use config::{ShellConfig, ShellConfigEdit};
pub use detect::{detect_shells, FnmShellOptions, ShellInfo, ShellType};
pub use verify::{get_or_create_config_path, verify_shell_config, VerificationResult};
