mod environment;
mod main;
mod onboarding;
mod operations;
mod ui;

pub use environment::*;
pub use main::*;
pub use onboarding::*;
pub use operations::*;
pub use ui::*;

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum AppState {
    Loading,
    Onboarding(OnboardingState),
    Main(MainState),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum MainViewKind {
    #[default]
    Versions,
    Settings,
    About,
}
