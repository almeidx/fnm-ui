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
pub enum AppState {
    Loading,
    Onboarding(OnboardingState),
    Main(Box<MainState>),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum MainViewKind {
    #[default]
    Versions,
    Settings,
    About,
}

#[cfg(test)]
mod tests {
    use super::MainViewKind;

    #[test]
    fn main_view_kind_default_is_versions() {
        assert_eq!(MainViewKind::default(), MainViewKind::Versions);
    }
}
