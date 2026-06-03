mod page;
mod popup;

pub use page::*;

use crate::config::Theme;

#[derive(Debug)]
pub struct UIState {
    pub is_running: bool,
    pub theme: Theme,
    pub current_page: PageState,
}

impl Default for UIState {
    fn default() -> Self {
        Self {
            is_running: true,
            theme: Theme::default(),
            current_page: PageState::_Other,
        }
    }
}

impl UIState {
    pub fn current_page(&self) -> &PageState {
        &self.current_page
    }

    pub fn current_page_mut(&mut self) -> &mut PageState {
        &mut self.current_page
    }
}

pub type UIStateGuard<'a> = parking_lot::MutexGuard<'a, UIState>;
