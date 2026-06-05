use crate::config::Theme;

#[derive(Debug)]
pub struct UIState {
    pub is_running: bool,
    pub theme: Theme,
}

impl Default for UIState {
    fn default() -> Self {
        Self {
            is_running: true,
            theme: Theme::default(),
        }
    }
}

pub type UIStateGuard<'a> = parking_lot::MutexGuard<'a, UIState>;
