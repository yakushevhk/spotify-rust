//! State management module
//!
//! This module manages all application state, including:
//! - UI state (current view, selections)
//! - Player state (playback, devices, queue)
//! - Application data (playlists, albums, cache)
//!
//! # State Access Pattern
//!
//! The state uses a shared state pattern with `Arc<State>`:
//!
//! ```rust
//! // UI State - Mutex for exclusive access
//! let ui = state.ui.lock();
//!
//! // Player State - RwLock for read-heavy access
//! let player = state.player.read();
//! let mut player = state.player.write();
//!
//! // Data - RwLock for read-heavy access
//! let data = state.data.read();
//! ```
//!
//! # Lock Hierarchy
//!
//! To prevent deadlocks, always acquire locks in this order:
//! 1. `state.ui` (Mutex)
//! 2. `state.player` (RwLock)
//! 3. `state.data` (RwLock)
//! 4. `state.toast_queue` (Mutex)
//! 5. `stream_conn` (Mutex) - streaming feature only

mod constant;
mod data;
mod model;
mod player;
mod queue;
mod ui;

use std::{collections::VecDeque, sync::Arc, sync::atomic::AtomicBool};

pub use constant::*;
pub use data::*;
pub use model::*;
pub use player::*;
#[allow(unused_imports)]
pub use queue::*;
pub use ui::*;

use crate::config;

pub use parking_lot::{Mutex, RwLock};

/// Application's shared state
pub type SharedState = Arc<State>;

/// Application's state
pub struct State {
    #[allow(dead_code)]
    pub ui: Mutex<UIState>,
    // TODO: player and data see more writes than reads in typical usage,
    // making RwLock suboptimal. Consider replacing with parking_lot::Mutex
    // to reduce overhead and avoid writer starvation.
    pub player: RwLock<PlayerState>,
    pub data: RwLock<AppData>,

    pub is_daemon: bool,

    /// Shutdown signal for background threads. Set to false in main.rs after
    /// the GUI window closes, allowing `player_event_watcher` and `media_control`
    /// loops to exit cleanly.
    pub running: Arc<AtomicBool>,

    /// Shared FFT frequency-band data written by the audio sink and read by the UI.
    /// `Some` only when `enable_audio_visualization` is `true`; avoids allocating
    /// the mutex/state entirely when the feature is not in use.
    #[cfg(feature = "streaming")]
    pub vis_bands: Option<Arc<Mutex<crate::ui::streaming::VisBands>>>,

    pub logs: Arc<Mutex<VecDeque<String>>>,

    /// A queue of toast messages to display in the GUI, written from background tasks.
    pub toast_queue: Mutex<VecDeque<String>>,
}

impl State {
    pub fn new(is_daemon: bool, log_buffer: Arc<Mutex<VecDeque<String>>>) -> Self {
        let mut ui = UIState::default();
        let configs = config::get_config();

        if let Some(theme) = configs.theme_config.find_theme(&configs.app_config.theme) {
            // update the UI's theme based on the `theme` config option
            ui.theme = theme;
        }

        let app_data = AppData::new(&configs.cache_folder);

        Self {
            ui: Mutex::new(ui),
            player: RwLock::new(PlayerState::default()),
            data: RwLock::new(app_data),
            is_daemon,
            running: Arc::new(AtomicBool::new(true)),
            #[cfg(feature = "streaming")]
            vis_bands: if configs.app_config.enable_audio_visualization {
                Some(Arc::new(Mutex::new(
                    crate::ui::streaming::VisBands::default(),
                )))
            } else {
                None
            },

            logs: log_buffer,
            toast_queue: Mutex::new(VecDeque::new()),
        }
    }

    const MAX_TOAST_QUEUE: usize = 32;

    pub fn push_toast(&self, msg: impl Into<String>) {
        let mut queue = self.toast_queue.lock();
        if queue.len() >= Self::MAX_TOAST_QUEUE {
            queue.pop_front();
        }
        queue.push_back(msg.into());
    }

    #[cfg(feature = "streaming")]
    pub fn is_streaming_enabled(&self) -> bool {
        let configs = config::get_config();
        configs.app_config.enable_streaming == config::StreamingType::Always
            || (configs.app_config.enable_streaming == config::StreamingType::DaemonOnly
                && self.is_daemon)
    }

    /// Reset user data to avoid stale data from a previous account/session.
    pub fn reset_user_data(&self) {
        let mut data = self.data.write();
        data.user_data = UserData {
            user: None,
            playlists: Vec::new(),
            playlist_folder_node: None,
            followed_artists: Vec::new(),
            saved_shows: Vec::new(),
            saved_albums: Vec::new(),
            saved_tracks: std::collections::HashMap::new(),
        };
        data.caches = data::MemoryCaches::new();
        data.browse = data::BrowseData::default();
    }
}
