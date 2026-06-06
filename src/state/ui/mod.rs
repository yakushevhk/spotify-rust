use crate::config::Theme;

#[derive(Debug)]
#[allow(dead_code)]
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
