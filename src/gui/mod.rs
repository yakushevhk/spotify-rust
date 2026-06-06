//! GUI module built with egui
//!
//! This module contains all GUI-related code using the egui immediate mode
//! GUI library. The main application struct is `SpotifyApp`.
//!
//! # Module Structure
//!
//! - `SpotifyApp` - Main application implementing `eframe::App`
//! - `View` enum - Different views (Library, Tracks, Search, etc.)
//! - `views.rs` - View rendering implementations
//! - `sidebar.rs` - Navigation sidebar
//! - `playback_bar.rs` - Playback controls
//! - `command_palette.rs` - Quick command access
//! - `context_menu.rs` - Right-click menus
//! - `image_cache.rs` - Album art caching
//! - `theme.rs` - Theme application
//!
//! # Views
//!
//! - `Library` - User's library (playlists, albums, artists)
//! - `Tracks` - Track list for a context
//! - `Search` - Search interface
//! - `Browse` - Browse categories
//! - `Queue` - Playback queue
//! - `Lyrics` - Track lyrics
//! - `Artist` - Artist page
//! - `Shows` - Podcasts
//! - `Settings` - Application settings
//! - `Help` - Keyboard shortcuts help
//! - `Logs` - Application logs

mod command_palette;
mod context_menu;
mod image_cache;
mod playback_bar;
mod sidebar;
mod theme;
mod views;

use eframe::egui;
use rspotify::prelude::Id;

// Accessibility: Initialize reduced motion on startup
#[allow(dead_code)]
fn init_accessibility() {
    theme::init_reduced_motion();
}

use crate::client::{ClientRequest, PlayerRequest};
use crate::command::{self, ActionCommand, Command, NavCommand, PageCommand, PlaybackCommand, SortCommand, ThemeCommand};
use crate::config::keymap::default_keybindings;
use crate::key::{CommandBinding, KeySequenceResult, KeySequenceState};
use crate::state::{self, PlayableId, SharedState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum View {
    Library,
    Tracks,
    Search,
    Browse,
    BrowseCategory { id: String, name: String },
    Queue,
    Settings,
    Lyrics,
    Artist,
    Shows,
    ShowDetail,
    Help,
    Logs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortColumn {
    Title,
    Artist,
    Album,
    Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthStep {
    Idle,
    Step1, // Getting Client ID
    Step2, // OAuth in progress
    #[allow(dead_code)]
    Complete,
}

impl SortDirection {
    pub fn toggle(self) -> Self {
        match self {
            Self::Ascending => Self::Descending,
            Self::Descending => Self::Ascending,
        }
    }

    pub fn arrow(self) -> &'static str {
        match self {
            Self::Ascending => "▲",
            Self::Descending => "▼",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SortState {
    pub column: SortColumn,
    pub direction: SortDirection,
}

impl SortState {
    pub fn compare(&self, a: &state::Track, b: &state::Track) -> std::cmp::Ordering {
        let ord = match self.column {
            // Use cached lowercase values for sorting (avoids O(n log n) allocations)
            SortColumn::Title => a.name_lower_ref().cmp(b.name_lower_ref()),
            SortColumn::Artist => a.artists_info_lower_ref().cmp(&b.artists_info_lower_ref()),
            SortColumn::Album => a.album_info_lower_ref().cmp(&b.album_info_lower_ref()),
            SortColumn::Duration => a.duration.cmp(&b.duration),
        };
        match self.direction {
            SortDirection::Ascending => ord,
            SortDirection::Descending => ord.reverse(),
        }
    }
}

pub enum SortAction {
    Sort(SortState),
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibrarySortOrder {
    Default,
    Alphabetical,
    RecentlyAdded,
}

#[derive(Debug, Clone)]
pub struct ToastMessage {
    pub message: String,
    pub expires: std::time::Instant,
    pub is_error: bool,
}

impl ToastMessage {
    pub fn new(message: String, is_error: bool) -> Self {
        Self {
            message,
            expires: std::time::Instant::now() + std::time::Duration::from_secs(5),
            is_error,
        }
    }
}

enum Action {
    Navigate(View),
    OpenPlaylist(usize),
    OpenAlbum(usize),
    OpenLikedTracks,
    OpenRecentlyPlayed,
    OpenTopTracks,
    OpenSearchResultPlaylist(state::Playlist),
    OpenSearchResultAlbum(state::Album),
    OpenArtist(state::Artist),
    OpenBrowseCategory(String, String),
    OpenBrowsePlaylist(state::Playlist),
    BackToBrowse,
    ContextMenuNavigateArtist(state::Artist),
    ContextMenuNavigateAlbum(state::Album),
    ContextMenuNavigateShow(state::Show),
    OpenCreatePlaylist,
    OpenAuthModal,
    #[allow(dead_code)]
    OpenShows,
    OpenShowDetail(state::Show),
    OpenShowFromSearch(state::Show),
    NavigateToCurrentTrack,
    None,
}

pub struct SpotifyApp {
    state: SharedState,
    client_pub: flume::Sender<ClientRequest>,
    current_view: View,
    search_query: String,
    selected_track: Option<usize>,
    context_tracks: Vec<crate::state::Track>,
    context_title: String,
    image_cache: image_cache::ImageCache,
    artist_context: Option<crate::state::Context>,
    artist_id: Option<String>,
    show_device_popup: bool,
    devices_fetched: bool,
    context_menu: context_menu::ContextMenu,
    sort_state: Option<SortState>,
    show_create_playlist_popup: bool,
    create_playlist_name: String,
    create_playlist_desc: String,
    create_playlist_public: bool,
    create_playlist_collab: bool,
    create_playlist_name_error: Option<String>,
    show_add_to_playlist_popup: bool,
    add_to_playlist_track: Option<state::PlayableId<'static>>,
    add_to_playlist_filter: String,
    toast_messages: Vec<ToastMessage>,
    toast_show_all: bool,
    current_context_id: Option<state::ContextId>,
    key_seq_state: KeySequenceState,
    keybindings: Vec<CommandBinding>,
    help_search: String,
    view_history: Vec<View>,
    forward_history: Vec<View>,
    show_theme_switcher: bool,
    theme_search: String,
    current_theme_name: String,
    show_command_palette: bool,
    command_palette: command_palette::CommandPalette,
    settings_tab: views::SettingsTab,
    podcast_detail_show: Option<state::Show>,
    podcast_episodes: Vec<state::Episode>,
    podcast_context_id: Option<state::ContextId>,
    selected_podcast_episode: Option<usize>,
    settings_editing: crate::config::AppConfig,
    settings_original: crate::config::AppConfig,
    settings_dirty: bool,
    settings_keybinding_search: String,
    settings_editing_keybinding: Option<usize>,
    settings_editing_keybindings: Vec<crate::key::CommandBinding>,
    library_sort_order: LibrarySortOrder,
    scroll_to_selected: bool,
    
    // Onboarding
    show_onboarding: bool,
    onboarding_completed: bool,
    
    // Authentication
    show_auth_modal: bool,
    auth_client_id_input: String,
    auth_step: AuthStep,
    show_browse_playlists_popup: bool,
    show_browse_artists_popup: bool,
    show_browse_albums_popup: bool,
    browse_popup_filter: String,
    show_in_page_search: bool,
    in_page_search_query: String,
    waveform_cache: Option<(String, usize, Vec<f32>)>,
    selected_artist_track: Option<usize>,
    search_debounce_state: crate::gui::views::SearchDebounceState,
    
    // Track selection persistence per context
    #[allow(dead_code)]
    selected_track_per_context: std::collections::HashMap<String, usize>,
    
    // Search results caching
    #[allow(dead_code)]
    last_search_query: String,
    #[allow(dead_code)]
    last_search_results: Option<crate::state::SearchResults>,
    
    // Settings unsaved changes dialog
    show_settings_confirm_dialog: bool,
    pending_view_navigation: Option<View>,
}

impl SpotifyApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        state: SharedState,
        client_pub: flume::Sender<ClientRequest>,
    ) -> Self {
        // Initialize theme from config
        let config = crate::config::get_config();
        let theme_name = &config.app_config.theme;
        let current_theme_name;
        if let Some(t) = config.theme_config.find_theme(theme_name) {
            theme::set_palette_from_config(&t.palette);
            current_theme_name = t.name.clone();
        } else {
            theme::set_palette(theme_name);
            current_theme_name = theme_name.clone();
        }
        theme::setup_theme(&cc.egui_ctx);

        let keybindings = {
            let mut bindings = default_keybindings();
            crate::config::get_config().keymap_config.apply_overrides(&mut bindings);
            bindings
        };
        
        Self {
            state,
            client_pub,
current_view: View::Library,
            search_query: String::new(),
            selected_track: None,
            context_tracks: Vec::with_capacity(100),
            context_title: String::new(),
            image_cache: image_cache::ImageCache::new(),
            artist_context: None,
            artist_id: None,
            show_device_popup: false,
            devices_fetched: false,
            context_menu: context_menu::ContextMenu::new(),
            sort_state: None,
            show_create_playlist_popup: false,
            create_playlist_name: String::new(),
            create_playlist_desc: String::new(),
            create_playlist_public: true,
            create_playlist_collab: false,
            create_playlist_name_error: None,
            show_add_to_playlist_popup: false,
            add_to_playlist_track: None,
            add_to_playlist_filter: String::new(),
            toast_messages: Vec::new(),
            toast_show_all: false,
            current_context_id: None,
            key_seq_state: KeySequenceState::new(),
            keybindings,
            help_search: String::new(),
            view_history: Vec::with_capacity(10),
            forward_history: Vec::with_capacity(10),
            show_theme_switcher: false,
            theme_search: String::new(),
            current_theme_name,
            show_command_palette: false,
            command_palette: command_palette::CommandPalette::new(),
            settings_tab: views::SettingsTab::General,
            podcast_detail_show: None,
            podcast_episodes: Vec::new(),
            podcast_context_id: None,
            selected_podcast_episode: None,
            settings_editing: crate::config::get_config().app_config.clone(),
            settings_original: crate::config::get_config().app_config.clone(),
            settings_dirty: false,
            settings_keybinding_search: String::new(),
            settings_editing_keybinding: None,
            settings_editing_keybindings: {
                let mut bindings = default_keybindings();
                crate::config::get_config().keymap_config.apply_overrides(&mut bindings);
                bindings
            },
            library_sort_order: LibrarySortOrder::Default,
            scroll_to_selected: false,
            show_onboarding: false,
            onboarding_completed: false,
            show_auth_modal: false,
            auth_client_id_input: String::new(),
            auth_step: AuthStep::Idle,
            show_browse_playlists_popup: false,
            show_browse_artists_popup: false,
            show_browse_albums_popup: false,
            browse_popup_filter: String::new(),
            show_in_page_search: false,
            in_page_search_query: String::new(),
            waveform_cache: None,
            selected_artist_track: None,
            search_debounce_state: crate::gui::views::SearchDebounceState::default(),
            selected_track_per_context: std::collections::HashMap::new(),
            last_search_query: String::new(),
            last_search_results: None,
            show_settings_confirm_dialog: false,
            pending_view_navigation: None,
        }
    }

    fn handle_action(&mut self, action: Action) {
        match action {
            Action::Navigate(view) => {
                if view != self.current_view && view != View::Help {
                    self.view_history.push(self.current_view.clone());
                    self.forward_history.clear();
                }
                self.current_view = view;
            }
            Action::OpenPlaylist(idx) => {
                if self.current_view != View::Help {
                    self.view_history.push(self.current_view.clone());
                    self.forward_history.clear();
                }
                let data = self.state.data.read();
                if let Some(state::PlaylistFolderItem::Playlist(playlist)) = data.user_data.playlists.get(idx) {
                    let id = playlist.id.clone();
                    let name = playlist.name.clone();
                    drop(data);
                    self.context_title = name;
                    self.selected_track = None;
                    self.context_tracks.clear();
                    self.sort_state = None;
                self.current_context_id = Some(state::ContextId::Playlist(id.clone()));
                self.send_request(ClientRequest::GetContext(
                    state::ContextId::Playlist(id),
                ));
                self.current_view = View::Tracks;
                }
            }
            Action::OpenAlbum(idx) => {
                if self.current_view != View::Help {
                    self.view_history.push(self.current_view.clone());
                    self.forward_history.clear();
                }
                let data = self.state.data.read();
                if let Some(album) = data.user_data.saved_albums.get(idx) {
                    let id = album.id.clone();
                    let name = album.name.clone();
                    drop(data);
                    self.context_title = name;
                    self.selected_track = None;
                    self.context_tracks.clear();
                    self.sort_state = None;
                    self.current_context_id = Some(state::ContextId::Album(id.clone()));
                    let _ = self
                        .client_pub
                        .send(ClientRequest::GetContext(state::ContextId::Album(id)));
                    self.current_view = View::Tracks;
                }
            }
            Action::OpenLikedTracks => {
                if self.current_view != View::Help {
                    self.view_history.push(self.current_view.clone());
                    self.forward_history.clear();
                }
                self.context_title = "Liked Tracks".to_string();
                self.context_tracks.clear();
                self.sort_state = None;
                self.selected_track = None;
                let ctx_id = state::ContextId::Tracks(state::TracksId::new(
                    state::USER_LIKED_TRACKS_URI,
                    "Liked Tracks",
                ));
                self.current_context_id = Some(ctx_id.clone());
                self.send_request(ClientRequest::GetContext(ctx_id));
                self.current_view = View::Tracks;
            }
            Action::OpenRecentlyPlayed => {
                if self.current_view != View::Help {
                    self.view_history.push(self.current_view.clone());
                    self.forward_history.clear();
                }
                self.context_title = "Recently Played".to_string();
                self.context_tracks.clear();
                self.sort_state = None;
                self.selected_track = None;
                let ctx_id = state::ContextId::Tracks(state::TracksId::new(
                    state::USER_RECENTLY_PLAYED_TRACKS_URI,
                    "Recently Played",
                ));
                self.current_context_id = Some(ctx_id.clone());
                self.send_request(ClientRequest::GetContext(ctx_id));
                self.current_view = View::Tracks;
            }
            Action::OpenTopTracks => {
                if self.current_view != View::Help {
                    self.view_history.push(self.current_view.clone());
                    self.forward_history.clear();
                }
                self.context_title = "Top Tracks".to_string();
                self.context_tracks.clear();
                self.sort_state = None;
                self.selected_track = None;
                let ctx_id = state::ContextId::Tracks(state::TracksId::new(
                    state::USER_TOP_TRACKS_URI,
                    "Top Tracks",
                ));
                self.current_context_id = Some(ctx_id.clone());
                self.send_request(ClientRequest::GetContext(ctx_id));
                self.current_view = View::Tracks;
            }
            Action::OpenSearchResultPlaylist(playlist) => {
                if self.current_view != View::Help {
                    self.view_history.push(self.current_view.clone());
                    self.forward_history.clear();
                }
                self.context_title = playlist.name.clone();
                self.context_tracks.clear();
                self.sort_state = None;
                self.selected_track = None;
                self.current_context_id = Some(state::ContextId::Playlist(playlist.id.clone()));
                self.send_request(ClientRequest::GetContext(
                    state::ContextId::Playlist(playlist.id),
                ));
                self.current_view = View::Tracks;
            }
            Action::OpenSearchResultAlbum(album) => {
                if self.current_view != View::Help {
                    self.view_history.push(self.current_view.clone());
                    self.forward_history.clear();
                }
                self.context_title = album.name.clone();
                self.context_tracks.clear();
                self.sort_state = None;
                self.selected_track = None;
                self.current_context_id = Some(state::ContextId::Album(album.id.clone()));
                let _ = self
                    .client_pub
                    .send(ClientRequest::GetContext(state::ContextId::Album(album.id)));
                self.current_view = View::Tracks;
            }
            Action::OpenArtist(artist) => {
                if self.current_view != View::Help {
                    self.view_history.push(self.current_view.clone());
                    self.forward_history.clear();
                }
                self.artist_id = Some(artist.id.uri());
                self.artist_context = None;
                let _ = self
                    .client_pub
                    .send(ClientRequest::GetContext(state::ContextId::Artist(artist.id)));
                self.current_view = View::Artist;
            }
            Action::OpenBrowseCategory(id, name) => {
                if self.current_view != View::Help {
                    self.view_history.push(self.current_view.clone());
                    self.forward_history.clear();
                }
                self.send_request(ClientRequest::GetBrowseCategoryPlaylists(
                    state::Category {
                        id: id.clone(),
                        name: name.clone(),
                        icon_url: None,
                    },
                ));
                self.current_view = View::BrowseCategory { id, name };
            }
            Action::OpenBrowsePlaylist(playlist) => {
                if self.current_view != View::Help {
                    self.view_history.push(self.current_view.clone());
                    self.forward_history.clear();
                }
                self.context_title = playlist.name.clone();
                self.context_tracks.clear();
                self.sort_state = None;
                self.selected_track = None;
                self.current_context_id = Some(state::ContextId::Playlist(playlist.id.clone()));
                self.send_request(ClientRequest::GetContext(
                    state::ContextId::Playlist(playlist.id),
                ));
                self.current_view = View::Tracks;
            }
            Action::BackToBrowse => {
                self.current_view = View::Browse;
            }
            Action::ContextMenuNavigateArtist(artist) => {
                if self.current_view != View::Help {
                    self.view_history.push(self.current_view.clone());
                    self.forward_history.clear();
                }
                self.artist_id = Some(artist.id.uri());
                self.artist_context = None;
                let _ = self
                    .client_pub
                    .send(ClientRequest::GetContext(state::ContextId::Artist(artist.id)));
                self.current_view = View::Artist;
            }
            Action::ContextMenuNavigateAlbum(album) => {
                if self.current_view != View::Help {
                    self.view_history.push(self.current_view.clone());
                    self.forward_history.clear();
                }
                self.context_title = album.name.clone();
                self.context_tracks.clear();
                self.sort_state = None;
                self.selected_track = None;
                self.current_context_id = Some(state::ContextId::Album(album.id.clone()));
                let _ = self
                    .client_pub
                    .send(ClientRequest::GetContext(state::ContextId::Album(album.id)));
                self.current_view = View::Tracks;
            }
            Action::ContextMenuNavigateShow(show) => {
                if self.current_view != View::Help {
                    self.view_history.push(self.current_view.clone());
                    self.forward_history.clear();
                }
                let ctx_id = state::ContextId::Show(show.id.clone());
                self.podcast_context_id = Some(ctx_id.clone());
                self.podcast_detail_show = Some(show);
                self.podcast_episodes.clear();
                self.selected_podcast_episode = None;
                self.send_request(ClientRequest::GetContext(ctx_id));
                self.current_view = View::ShowDetail;
            }
            Action::OpenCreatePlaylist => {
                self.show_create_playlist_popup = true;
                self.create_playlist_name.clear();
                self.create_playlist_desc.clear();
                self.create_playlist_public = true;
                self.create_playlist_collab = false;
            }
            Action::OpenAuthModal => {
                self.show_auth_modal = true;
                self.auth_step = AuthStep::Step1;
            }
            Action::OpenShows => {
                let data = self.state.data.read();
                let shows_empty = data.user_data.saved_shows.is_empty();
                let shows_loading = data.shows_loading;
                drop(data);
                if shows_empty && !shows_loading {
                    self.state.data.write().shows_loading = true;
                    self.send_request(ClientRequest::GetUserSavedShows);
                }
                self.navigate_to_view(View::Shows);
            }
            Action::OpenShowDetail(show) => {
                if self.current_view != View::Help {
                    self.view_history.push(self.current_view.clone());
                    self.forward_history.clear();
                }
                let ctx_id = state::ContextId::Show(show.id.clone());
                self.podcast_context_id = Some(ctx_id.clone());
                self.podcast_detail_show = Some(show);
                self.podcast_episodes.clear();
                self.selected_podcast_episode = None;
                self.send_request(ClientRequest::GetContext(ctx_id));
                self.current_view = View::ShowDetail;
            }
            Action::OpenShowFromSearch(show) => {
                if self.current_view != View::Help {
                    self.view_history.push(self.current_view.clone());
                    self.forward_history.clear();
                }
                let ctx_id = state::ContextId::Show(show.id.clone());
                self.podcast_context_id = Some(ctx_id.clone());
                self.podcast_detail_show = Some(show);
                self.podcast_episodes.clear();
                self.selected_podcast_episode = None;
                self.send_request(ClientRequest::GetContext(ctx_id));
                self.current_view = View::ShowDetail;
            }
            Action::NavigateToCurrentTrack => {
                // Navigate to the context of the currently playing track
                let player = self.state.player.read();
                if let Some(ref playback) = player.playback {
                    if let Some(ref context) = playback.context {
                        let uri = context.uri.clone();
                        drop(player);
                        
                        // Parse context URI and navigate appropriately
                        if uri.contains(":playlist:") || uri.contains(":album:") {
                            self.navigate_to_view(View::Tracks);
                        } else if uri.contains(":artist:") {
                            self.navigate_to_view(View::Artist);
                        } else if uri.contains(":show:") {
                            self.navigate_to_view(View::ShowDetail);
                        } else {
                            self.navigate_to_view(View::Queue);
                        }
                    } else {
                        drop(player);
                        self.navigate_to_view(View::Queue);
                    }
                } else {
                    drop(player);
                }
            }
            Action::None => {}
        }
    }

    fn navigate_to_view(&mut self, view: View) {
        if view != self.current_view {
            // Check for unsaved settings changes
            if self.current_view == View::Settings && self.settings_dirty {
                self.pending_view_navigation = Some(view);
                self.show_settings_confirm_dialog = true;
                return;
            }
            
            // Don't add modal/popup views to history
            if !self.is_modal_view(&self.current_view) {
                self.view_history.push(self.current_view.clone());
                self.forward_history.clear();
            }
        }
        self.current_view = view;
    }

    /// Check if a view is a modal/popup that shouldn't be added to history
    fn is_modal_view(&self, view: &View) -> bool {
        matches!(view, 
            View::Help | 
            View::Logs |
            View::Settings
        )
    }

    fn go_back(&mut self) {
        if let Some(prev) = self.view_history.pop() {
            self.forward_history.push(std::mem::replace(&mut self.current_view, prev));
            // B2: clear stale track selection and context when navigating back
            self.selected_track = None;
            self.context_tracks.clear();
            self.sort_state = None;
        }
    }

    fn navigate_forward(&mut self) {
        if let Some(next) = self.forward_history.pop() {
            self.view_history.push(std::mem::replace(&mut self.current_view, next));
        }
    }

    fn send_request(&self, req: ClientRequest) {
        // H5: use try_send to avoid blocking the UI thread when the channel is full.
        // With a buffer of 1024 this should rarely happen, but if it does we log and
        // show a toast rather than freezing the GUI.
        if let Err(e) = self.client_pub.try_send(req) {
            match e {
                flume::TrySendError::Full(_) => {
                    let msg = "Client request channel full, dropping request".to_string();
                    tracing::warn!("{}", msg);
                    // Push to toast_queue for display on next frame
                    self.state.push_toast(format!("⚠️ {} (Click to retry)", msg));
                }
                flume::TrySendError::Disconnected(_) => {
                    let msg = "Failed to send request: client channel closed".to_string();
                    tracing::warn!("{}", msg);
                    self.state.push_toast(format!("❌ {}", msg));
                }
            }
        }
    }

    #[allow(dead_code, clippy::result_large_err)]
    fn send_request_with_retry(&self, req: ClientRequest) -> Result<(), flume::TrySendError<ClientRequest>> {
        self.client_pub.try_send(req)
    }

    fn execute_command(&mut self, cmd: &Command, count: usize, ctx: &egui::Context) {
        match cmd {
            Command::Navigation(nav) => match nav {
                NavCommand::Up => {
                    for _ in 0..count {
                        match self.selected_track {
                            Some(ref mut sel) if *sel > 0 => *sel -= 1,
                            None if !self.context_tracks.is_empty() => {
                                self.selected_track = Some(self.context_tracks.len() - 1);
                            }
                            _ => {}
                        }
                    }
                }
                NavCommand::Down => {
                    for _ in 0..count {
                        match self.selected_track {
                            Some(sel) if sel + 1 < self.context_tracks.len() => {
                                self.selected_track = Some(sel + 1);
                            }
                            None if !self.context_tracks.is_empty() => {
                                self.selected_track = Some(0);
                            }
                            _ => {}
                        }
                    }
                }
                NavCommand::PageUp => {
                    let page_size = crate::config::get_config().app_config.page_size_in_rows;
                    for _ in 0..count {
                        match self.selected_track {
                            Some(ref mut sel) => {
                                *sel = sel.saturating_sub(page_size);
                            }
                            None if !self.context_tracks.is_empty() => {
                                self.selected_track = Some(0);
                            }
                            _ => {}
                        }
                    }
                }
                NavCommand::PageDown => {
                    let page_size = crate::config::get_config().app_config.page_size_in_rows;
                    for _ in 0..count {
                        match self.selected_track {
                            Some(sel) => {
                                let new_sel = (sel + page_size).min(self.context_tracks.len().saturating_sub(1));
                                self.selected_track = Some(new_sel);
                            }
                            None if !self.context_tracks.is_empty() => {
                                self.selected_track = Some(0);
                            }
                            _ => {}
                        }
                    }
                }
                NavCommand::First => {
                    if !self.context_tracks.is_empty() {
                        self.selected_track = Some(0);
                    }
                }
                NavCommand::Last => {
                    if !self.context_tracks.is_empty() {
                        self.selected_track = Some(self.context_tracks.len() - 1);
                    }
                }
                NavCommand::FocusNext => {
                    let views = [View::Library, View::Search, View::Browse, View::Queue, View::Lyrics, View::Shows, View::Settings];
                    if let Some(idx) = views.iter().position(|v| *v == self.current_view) {
                        let next = (idx + 1) % views.len();
                        self.navigate_to_view(views[next].clone());
                    }
                }
                NavCommand::FocusPrev => {
                    let views = [View::Library, View::Search, View::Browse, View::Queue, View::Lyrics, View::Shows, View::Settings];
                    if let Some(idx) = views.iter().position(|v| *v == self.current_view) {
                        let prev = if idx == 0 { views.len() - 1 } else { idx - 1 };
                        self.navigate_to_view(views[prev].clone());
                    }
                }
                NavCommand::Back => {
                    self.go_back();
                }
                NavCommand::Forward => {
                    self.navigate_forward();
                }
                NavCommand::Quit => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                NavCommand::InPageSearch => {
                    self.show_in_page_search = true;
                    self.in_page_search_query.clear();
                }
                NavCommand::Enter => {
                    if self.current_view == View::ShowDetail {
                        if let Some(idx) = self.selected_podcast_episode {
                            if idx < self.podcast_episodes.len() {
                                let episode = &self.podcast_episodes[idx];
                                if let Some(ref ctx_id) = self.podcast_context_id {
                                    let playback = state::Playback::Context(
                                        ctx_id.clone(),
                                        Some(rspotify::model::Offset::Uri(episode.id.uri())),
                                    );
                                    self.send_request(ClientRequest::Player(
                                        PlayerRequest::StartPlayback(playback, None),
                                    ));
                                }
                            }
                        }
                    } else if let Some(idx) = self.selected_track {
                        if idx < self.context_tracks.len() {
                            let track = self.context_tracks[idx].clone();
                            self.play_track_from_context(&track);
                        }
                    }
                }
            },
            Command::Playback(pb) => match pb {
                PlaybackCommand::PlayPause => {
                    self.send_request(ClientRequest::Player(PlayerRequest::ResumePause));
                }
                PlaybackCommand::NextTrack => {
                    self.send_request(ClientRequest::Player(PlayerRequest::NextTrack));
                }
                PlaybackCommand::PrevTrack => {
                    self.send_request(ClientRequest::Player(PlayerRequest::PreviousTrack));
                }
                PlaybackCommand::RefreshPlayback => {
                    self.send_request(ClientRequest::GetCurrentPlayback);
                    self.toast("Refreshing playback...".to_string());
                }
                PlaybackCommand::RestartClient => {
                    #[cfg(feature = "streaming")]
                    {
                        self.send_request(ClientRequest::RestartIntegratedClient);
                        self.toast("Restarting client...".to_string());
                    }
                }
                PlaybackCommand::MuteToggle => {
                    self.send_request(ClientRequest::Player(PlayerRequest::ToggleMute));
                }
                PlaybackCommand::SeekToStart => {
                    self.send_request(ClientRequest::Player(PlayerRequest::SeekTrack(
                        chrono::Duration::zero(),
                    )));
                }
                PlaybackCommand::SeekForward => {
                    let seek_secs = crate::config::get_config().app_config.seek_duration_secs as i64;
                    let player = self.state.player.read();
                    let current_pos = player.playback_progress()
                        .unwrap_or(chrono::Duration::zero());
                    drop(player);
                    let new_pos = current_pos + chrono::Duration::seconds(seek_secs * count as i64);
                    self.send_request(ClientRequest::Player(PlayerRequest::SeekTrack(new_pos)));
                }
                PlaybackCommand::SeekBackward => {
                    let seek_secs = crate::config::get_config().app_config.seek_duration_secs as i64;
                    let player = self.state.player.read();
                    let current_pos = player.playback_progress()
                        .unwrap_or(chrono::Duration::zero());
                    drop(player);
                    let new_pos = (current_pos - chrono::Duration::seconds(seek_secs * count as i64))
                        .max(chrono::Duration::zero());
                    self.send_request(ClientRequest::Player(PlayerRequest::SeekTrack(new_pos)));
                }
                PlaybackCommand::PlayRandom => {
                    if !self.context_tracks.is_empty() {
                        let idx = rand::random_range(0..self.context_tracks.len());
                        let track = self.context_tracks[idx].clone();
                        self.play_track_from_context(&track);
                    }
                }
                PlaybackCommand::Shuffle => {
                    self.send_request(ClientRequest::Player(PlayerRequest::Shuffle));
                }
                PlaybackCommand::Repeat => {
                    self.send_request(ClientRequest::Player(PlayerRequest::Repeat));
                }
                PlaybackCommand::VolumeUp => {
                    let vol = self.state.player.read()
                        .current_playback()
                        .and_then(|p| p.device.volume_percent)
                        .unwrap_or(50) as u8;
                    let step = crate::config::get_config().app_config.volume_scroll_step;
                    let new_vol = vol.saturating_add(step).min(100);
                    self.send_request(ClientRequest::Player(PlayerRequest::Volume(new_vol)));
                }
                PlaybackCommand::VolumeDown => {
                    let vol = self.state.player.read()
                        .current_playback()
                        .and_then(|p| p.device.volume_percent)
                        .unwrap_or(50) as u8;
                    let step = crate::config::get_config().app_config.volume_scroll_step;
                    let new_vol = vol.saturating_sub(step);
                    self.send_request(ClientRequest::Player(PlayerRequest::Volume(new_vol)));
                }
            },
            Command::Sorting(sort) => {
                match sort {
                    SortCommand::ByTitle => {
                        let new_state = SortState { column: SortColumn::Title, direction: SortDirection::Ascending };
                        self.apply_sort(new_state);
                    }
                    SortCommand::ByArtist => {
                        let new_state = SortState { column: SortColumn::Artist, direction: SortDirection::Ascending };
                        self.apply_sort(new_state);
                    }
                    SortCommand::ByAlbum => {
                        let new_state = SortState { column: SortColumn::Album, direction: SortDirection::Ascending };
                        self.apply_sort(new_state);
                    }
                    SortCommand::ByDuration => {
                        let new_state = SortState { column: SortColumn::Duration, direction: SortDirection::Ascending };
                        self.apply_sort(new_state);
                    }
                    SortCommand::ByAddedDate => {
                        // Sort by added_at (falling back to track order)
                        self.context_tracks.sort_by_key(|a| a.added_at);
                        self.toast("Sorted by added date".to_string());
                    }
                    SortCommand::Reverse => {
                        self.context_tracks.reverse();
                        self.toast("Reversed track order".to_string());
                    }
                    SortCommand::LibraryAlphabetical => {
                        self.library_sort_order = LibrarySortOrder::Alphabetical;
                        self.toast("Library sorted alphabetically".to_string());
                    }
                    SortCommand::LibraryRecentlyAdded => {
                        self.library_sort_order = LibrarySortOrder::RecentlyAdded;
                        self.toast("Library sorted by recently added".to_string());
                    }
                }
            }
            Command::Page(page) => match page {
                PageCommand::CurrentlyPlaying => {
                    self.update_context_tracks();
                    if !self.context_tracks.is_empty() {
                        // Find the currently playing track and select it
                        let player = self.state.player.read();
                        let current_uri = player.playback.as_ref().and_then(|p| {
                            p.item.as_ref().map(|item| match item {
                                rspotify::model::PlayableItem::Track(t) => {
                                    t.id.as_ref().map(|id| id.uri()).unwrap_or_default()
                                }
                                rspotify::model::PlayableItem::Episode(e) => e.id.uri(),
                                _ => String::new(),
                            })
                        });
                        drop(player);
                        if let Some(uri) = current_uri {
                            if let Some(idx) = self.context_tracks.iter().position(|t| t.id.uri() == uri) {
                                self.selected_track = Some(idx);
                            }
                        }
                        self.navigate_to_view(View::Tracks);
                    }
                }
                PageCommand::TopTracks => {
                    self.handle_action(Action::OpenTopTracks);
                }
                PageCommand::RecentlyPlayed => {
                    self.handle_action(Action::OpenRecentlyPlayed);
                }
                PageCommand::LikedTracks => {
                    self.handle_action(Action::OpenLikedTracks);
                }
                PageCommand::Library => {
                    self.navigate_to_view(View::Library);
                }
                PageCommand::Search => {
                    self.navigate_to_view(View::Search);
                }
                PageCommand::Browse => {
                    self.navigate_to_view(View::Browse);
                }
                PageCommand::Lyrics => {
                    self.navigate_to_view(View::Lyrics);
                }
                PageCommand::Queue => {
                    self.navigate_to_view(View::Queue);
                }
                PageCommand::Logs => {
                    self.navigate_to_view(View::Logs);
                }
                PageCommand::Help => {
                    self.help_search.clear();
                    self.navigate_to_view(View::Help);
                }
                PageCommand::OpenSpotifyLink => {
                    self.open_spotify_link_from_clipboard();
                }
            },
            Command::Action(act) => match act {
                ActionCommand::ShowActionsOnSelected => {
                    if let Some(idx) = self.selected_track {
                        if idx < self.context_tracks.len() {
                            let track = self.context_tracks[idx].clone();
                            let center = egui::pos2(400.0, 400.0);
                            self.context_menu.open(
                                context_menu::ContextTarget::Track {
                                    track,
                                    index: idx,
                                    playlist_id: None,
                                },
                                center,
                            );
                        }
                    }
                }
                ActionCommand::ShowActionsOnCurrent => {
                    let player = self.state.player.read();
                    if let Some(ref playback) = player.playback {
                        if let Some(rspotify::model::PlayableItem::Track(t)) = &playback.item {
                            let track = crate::state::Track::try_from_full_track(t.clone());
                            if let Some(track) = track {
                                let center = egui::pos2(400.0, 400.0);
                                self.context_menu.open(
                                    context_menu::ContextTarget::Track {
                                        track,
                                        index: 0,
                                        playlist_id: None,
                                    },
                                    center,
                                );
                            }
                        }
                    }
                }
                ActionCommand::ShowActionsOnContext => {
                    self.toast("Context actions".to_string());
                }
                ActionCommand::AddToQueue => {
                    if let Some(idx) = self.selected_track {
                        if idx < self.context_tracks.len() {
                            let track = &self.context_tracks[idx];
                            self.send_request(ClientRequest::AddPlayableToQueue(
                                PlayableId::Track(track.id.clone()),
                            ));
                            self.toast("Added to queue".to_string());
                        }
                    }
                }
                ActionCommand::CreatePlaylist => {
                    self.handle_action(Action::OpenCreatePlaylist);
                }
                ActionCommand::JumpToCurrentInContext => {
                    let player = self.state.player.read();
                    let current_uri = player.playback.as_ref().and_then(|p| {
                        p.item.as_ref().map(|item| match item {
                            rspotify::model::PlayableItem::Track(t) => {
                                t.id.as_ref().map(|id| id.uri()).unwrap_or_default()
                            }
                            rspotify::model::PlayableItem::Episode(e) => e.id.uri(),
                            _ => String::new(),
                        })
                    });
                    drop(player);
                    if let Some(uri) = current_uri {
                        if let Some(idx) = self.context_tracks.iter().position(|t| t.id.uri() == uri) {
                            self.selected_track = Some(idx);
                        }
                    }
                }
                ActionCommand::JumpToHighlightedInContext => {
                    if self.selected_track.is_some() {
                        self.scroll_to_selected = true;
                    }
                }
                ActionCommand::GoToRadio => {
                    if let Some(idx) = self.selected_track {
                        if idx < self.context_tracks.len() {
                            let track = &self.context_tracks[idx];
                            let artist_name = track.artists.first().map(|a| a.name.clone()).unwrap_or_default();
                            let query = format!("artist:{}", artist_name);
                            self.send_request(ClientRequest::Search(query));
                            self.navigate_to_view(View::Search);
                            self.toast("Searching for radio...".to_string());
                        }
                    }
}
                ActionCommand::MovePlaylistItemUp => {
                    if let Some(state::ContextId::Playlist(playlist_id)) = self.current_context_id.as_ref() {
                        if let Some(idx) = self.selected_track {
                            if idx > 0 && idx < self.context_tracks.len() {
                                self.send_request(ClientRequest::ReorderPlaylistItems {
                                    playlist_id: playlist_id.clone(),
                                    insert_index: idx.saturating_sub(1),
                                    range_start: idx,
                                    range_length: Some(1),
                                    snapshot_id: None,
                                });
                                self.context_tracks.swap(idx, idx - 1);
                                self.selected_track = Some(idx - 1);
                                self.toast("Moved up".to_string());
                            }
                        }
                    }
                }
                ActionCommand::MovePlaylistItemDown => {
                    if let Some(state::ContextId::Playlist(playlist_id)) = self.current_context_id.as_ref() {
                        if let Some(idx) = self.selected_track {
                            if idx + 1 < self.context_tracks.len() {
                                self.send_request(ClientRequest::ReorderPlaylistItems {
                                    playlist_id: playlist_id.clone(),
                                    insert_index: idx + 2,
                                    range_start: idx,
                                    range_length: Some(1),
                                    snapshot_id: None,
                                });
                                self.context_tracks.swap(idx, idx + 1);
                                self.selected_track = Some(idx + 1);
                                self.toast("Moved down".to_string());
                            }
                        }
                    }
                }
                ActionCommand::SwitchDevice => {
                    if self.show_device_popup {
                        self.show_device_popup = false;
                    } else {
                        self.show_device_popup = true;
                        self.devices_fetched = false;
                    }
                }
            },
            Command::Popup(popup) => match popup {
                crate::command::PopupCommand::Playlists => {
                    self.show_browse_playlists_popup = true;
                    self.browse_popup_filter.clear();
                }
                crate::command::PopupCommand::FollowedArtists => {
                    self.show_browse_artists_popup = true;
                    self.browse_popup_filter.clear();
                }
                crate::command::PopupCommand::SavedAlbums => {
                    self.show_browse_albums_popup = true;
                    self.browse_popup_filter.clear();
                }
            },
            Command::Theme(theme_cmd) => match theme_cmd {
                ThemeCommand::SwitchTheme => {
                    self.show_theme_switcher = !self.show_theme_switcher;
                    self.theme_search.clear();
                }
            },
        }
    }

    fn apply_sort(&mut self, new_state: SortState) {
        if self.sort_state == Some(new_state) {
            // Already sorted this way, toggle direction
            let toggled = SortState {
                column: new_state.column,
                direction: new_state.direction.toggle(),
            };
            self.sort_state = Some(toggled);
            self.context_tracks.sort_by(|a, b| toggled.compare(a, b));
        } else {
            self.sort_state = Some(new_state);
            self.context_tracks.sort_by(|a, b| new_state.compare(a, b));
        }
        self.selected_track = None;
    }

    fn open_spotify_link_from_clipboard(&mut self) {
        let text = {
            #[cfg(target_os = "macos")]
            {
                std::process::Command::new("pbpaste")
                    .output()
                    .ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .map(|s| s.trim().to_string())
            }
            #[cfg(target_os = "linux")]
            {
                std::process::Command::new("xclip")
                    .args(["-selection", "clipboard", "-o"])
                    .output()
                    .ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .map(|s| s.trim().to_string())
            }
            #[cfg(target_os = "windows")]
            {
                std::process::Command::new("powershell")
                    .args(["-command", "Get-Clipboard"])
                    .output()
                    .ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .map(|s| s.trim().to_string())
            }
        };

        let Some(text) = text else {
            self.toast("Could not read clipboard".to_string());
            return;
        };

        if !text.contains("open.spotify.com") {
            self.toast("No Spotify link in clipboard".to_string());
            return;
        }

        // Parse: https://open.spotify.com/{type}/{id}  or  spotify:{type}:{id}
        let (link_type, id_str) = if text.contains("open.spotify.com") {
            // URL format: https://open.spotify.com/track/abc123?si=...
            let after_domain = text.split("open.spotify.com/").nth(1).unwrap_or("");
            let parts: Vec<&str> = after_domain.split('/').collect();
            if parts.len() >= 2 {
                let id = parts[1].split('?').next().unwrap_or("");
                (parts[0], id.to_string())
            } else {
                self.toast("Invalid Spotify link format".to_string());
                return;
            }
        } else if text.starts_with("spotify:") {
            // URI format: spotify:track:abc123
            let parts: Vec<&str> = text.split(':').collect();
            if parts.len() >= 3 {
                (parts[1], parts[2].to_string())
            } else {
                self.toast("Invalid Spotify URI format".to_string());
                return;
            }
        } else {
            self.toast("Unrecognized Spotify link".to_string());
            return;
        };

        match link_type {
            "track" => {
                if let Ok(track_id) = rspotify::model::TrackId::from_id(&id_str) {
                    let playback = state::Playback::URIs(
                        vec![PlayableId::Track(track_id.into_static())],
                        None,
                    );
                    self.send_request(ClientRequest::Player(
                        PlayerRequest::StartPlayback(playback, None),
                    ));
                    self.toast("Playing track".to_string());
                } else {
                    self.toast("Invalid track ID".to_string());
                }
            }
            "album" => {
                if let Ok(album_id) = rspotify::model::AlbumId::from_id(&id_str) {
                    let album_id_static = album_id.into_static();
                    self.context_title = String::new();
                    self.context_tracks.clear();
                    self.sort_state = None;
                    self.selected_track = None;
                    self.current_context_id = Some(state::ContextId::Album(album_id_static.clone()));
                    self.send_request(ClientRequest::GetContext(
                        state::ContextId::Album(album_id_static),
                    ));
                    self.current_view = View::Tracks;
                    self.toast("Opening album".to_string());
                } else {
                    self.toast("Invalid album ID".to_string());
                }
            }
            "artist" => {
                if let Ok(artist_id) = rspotify::model::ArtistId::from_id(&id_str) {
                    let artist_id_static = artist_id.into_static();
                    self.artist_id = Some(artist_id_static.uri());
                    self.artist_context = None;
                    self.send_request(ClientRequest::GetContext(
                        state::ContextId::Artist(artist_id_static),
                    ));
                    self.current_view = View::Artist;
                    self.toast("Opening artist".to_string());
                } else {
                    self.toast("Invalid artist ID".to_string());
                }
            }
            "playlist" => {
                if let Ok(playlist_id) = rspotify::model::PlaylistId::from_id(&id_str) {
                    let playlist_id_static = playlist_id.into_static();
                    self.context_title = String::new();
                    self.context_tracks.clear();
                    self.sort_state = None;
                    self.selected_track = None;
                    self.current_context_id = Some(state::ContextId::Playlist(playlist_id_static.clone()));
                    self.send_request(ClientRequest::GetContext(
                        state::ContextId::Playlist(playlist_id_static),
                    ));
                    self.current_view = View::Tracks;
                    self.toast("Opening playlist".to_string());
                } else {
                    self.toast("Invalid playlist ID".to_string());
                }
            }
            "show" => {
                if let Ok(show_id) = rspotify::model::ShowId::from_id(&id_str) {
                    let show_id_static = show_id.into_static();
                    let ctx_id = state::ContextId::Show(show_id_static.clone());
                    self.podcast_context_id = Some(ctx_id.clone());
                    self.podcast_detail_show = None;
                    self.podcast_episodes.clear();
                    self.selected_podcast_episode = None;
                    self.send_request(ClientRequest::GetContext(ctx_id));
                    self.current_view = View::ShowDetail;
                    self.toast("Opening show".to_string());
                } else {
                    self.toast("Invalid show ID".to_string());
                }
            }
            _ => {
                self.toast(format!("Unsupported link type: {}", link_type));
            }
        }
    }

    fn update_context_tracks(&mut self) {
        let data = self.state.data.read();
        let player = self.state.player.read();
        if let Some(ref playback) = player.playback {
            if let Some(ref context) = playback.context {
                let uri = context.uri.clone();
                if let Some(ctx) = data.caches.context.get(&uri) {
                    match ctx {
                        state::Context::Playlist { tracks, playlist } => {
                            self.context_tracks = tracks.clone();
                            self.context_title = playlist.name.clone();
                        }
                        state::Context::Album { tracks, album } => {
                            self.context_tracks = tracks.clone();
                            self.context_title = album.name.clone();
                        }
                        state::Context::Tracks { tracks, desc } => {
                            self.context_tracks = tracks.clone();
                            self.context_title = desc.clone();
                        }
                        state::Context::Artist {
                            top_tracks,
                            artist,
                            ..
                        } => {
                            self.context_tracks = top_tracks.clone();
                            self.context_title = format!("{} — Top Tracks", artist.name);
                        }
                        state::Context::Show { .. } => {}
                    }
                    // Apply existing sort if any
                    if let Some(sort) = self.sort_state {
                        self.context_tracks.sort_by(|a, b| sort.compare(a, b));
                    }
                }
            }
        }
    }
}

impl eframe::App for SpotifyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Batch state lock acquisitions - read once, get all needed data
        let is_playing = {
            let player = self.state.player.read();
            player.playback.is_some()
        };
        let repaint_ms = if is_playing { 100 } else { 1000 };
        ctx.request_repaint_after(std::time::Duration::from_millis(repaint_ms));

        // Check if this is first launch (no auth token)
        if !self.onboarding_completed && !self.show_onboarding {
            let data = self.state.data.read();
            let is_empty = data.user_data.playlists.is_empty() && data.user_data.saved_albums.is_empty();
            // Show onboarding if library is empty and we haven't completed onboarding
            if is_empty {
                self.show_onboarding = true;
            }
            drop(data);
        }

        // Drain toast messages from background tasks - use try_lock to avoid blocking
        let toasts: Vec<String> = self.state.toast_queue.try_lock()
            .map(|mut q| q.drain(..).collect())
            .unwrap_or_default();
        for msg in toasts {
            self.toast(msg);
        }

        if self.current_view == View::Tracks && self.context_tracks.is_empty() {
            self.update_context_tracks();
        }

        // Update show detail episodes when viewing show detail page
        if self.current_view == View::ShowDetail && self.podcast_episodes.is_empty() {
            if let Some(ref ctx_id) = self.podcast_context_id {
                let data = self.state.data.read();
                if let Some(state::Context::Show { show, episodes }) = data.caches.context.get(&ctx_id.uri()) {
                    self.podcast_detail_show = Some(show.clone());
                    self.podcast_episodes = episodes.clone();
                }
            }
        }

        // Update artist context when viewing artist page
        if self.current_view == View::Artist && self.artist_context.is_none() {
            if let Some(ref uri) = self.artist_id {
                let data = self.state.data.read();
                if let Some(ctx) = data.caches.context.get(uri) {
                    self.artist_context = Some(ctx.clone());
                }
            }
        }

        // Bottom panel — playback bar
        egui::TopBottomPanel::bottom("playback_bar")
            .resizable(false)
            .exact_height(theme::PLAYBACK_BAR_HEIGHT)
            .show(ctx, |ui| {
                let bar_response = playback_bar::render(ui, &self.state, &self.client_pub, &mut self.image_cache, &mut self.waveform_cache);
                if let Some(view) = bar_response.navigate {
                    // L11: push to history when navigating from playback bar
                    self.navigate_to_view(view);
                }
                if bar_response.device_button_clicked {
                    if self.show_device_popup {
                        self.show_device_popup = false;
                    } else {
                        self.show_device_popup = true;
                        self.devices_fetched = false;
                    }
                }
            });

        // Device popup overlay
        if self.show_device_popup {
            if !self.devices_fetched {
                self.send_request(ClientRequest::GetDevices);
                self.devices_fetched = true;
            }

            let screen_rect = ctx.screen_rect();
            let popup_width = 320.0;
            let popup_max_height = 400.0;
            let popup_x = screen_rect.right() - popup_width - 24.0;
            let popup_y = screen_rect.bottom() - theme::PLAYBACK_BAR_HEIGHT - 8.0;

            let mut close_popup = false;

            egui::Area::new(egui::Id::new("device_popup"))
                .order(egui::Order::Foreground)
                .fixed_pos(egui::pos2(popup_x, popup_y))
                .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(0.0, 0.0))
                .show(ctx, |ui| {
                    let frame = theme::glass_frame();

                    frame.show(ui, |ui| {
                        ui.set_min_width(popup_width - 16.0);
                        ui.set_max_height(popup_max_height);

                        // Header
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Connect to a device")
                                    .size(14.0)
                                    .strong()
                                    .color(theme::text_primary()),
                            );
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("✕")
                                                .size(14.0)
                                                .color(theme::text_dim()),
                                        )
                                        .fill(egui::Color32::TRANSPARENT),
                                    )
                                    .clicked()
                                {
                                    close_popup = true;
                                }
                            });
                        });

                        ui.add_space(8.0);

                        // Divider
                        let div_rect = ui.allocate_space(egui::vec2(ui.available_width(), 1.0)).1;
                        ui.painter()
                            .rect_filled(div_rect, 0.0, theme::divider());

                        ui.add_space(8.0);

                        let player = self.state.player.read();
                        let devices = &player.devices;

                        if devices.is_empty() {
                            ui.label(
                                egui::RichText::new("No devices available")
                                    .size(12.0)
                                    .color(theme::text_dim()),
                            );
                        } else {
                            let active_device_id = player
                                .playback
                                .as_ref()
                                .and_then(|p| p.device.id.clone());

                            for device in devices.iter() {
                                let is_active = device.is_active
                                    || active_device_id
                                        .as_ref()
                                        .map(|id| id == &device.id)
                                        .unwrap_or(false);

                                let item_height = 52.0;
                                let (item_rect, item_response) = ui.allocate_exact_size(
                                    egui::vec2(ui.available_width(), item_height),
                                    egui::Sense::click(),
                                );

                                let bg = if item_response.hovered() && !is_active {
                                    theme::bg_hover()
                                } else {
                                    egui::Color32::TRANSPARENT
                                };

                                ui.painter()
                                    .rect_filled(item_rect, egui::CornerRadius::same(theme::RADIUS_MEDIUM), bg);

                                // Device icon
                                ui.painter().text(
                                    item_rect.left_center() + egui::vec2(12.0, 0.0),
                                    egui::Align2::LEFT_CENTER,
                                    device.device_icon(),
                                    egui::FontId::proportional(18.0),
                                    theme::text_dim(),
                                );

                                // Device name
                                let name_color = if is_active {
                                    theme::green()
                                } else {
                                    theme::text_primary()
                                };
                                ui.painter().text(
                                    item_rect.left_center() + egui::vec2(44.0, -8.0),
                                    egui::Align2::LEFT_CENTER,
                                    &device.name,
                                    egui::FontId::proportional(13.0),
                                    name_color,
                                );

                                // Device type + active indicator
                                if is_active {
                                    // Green dot
                                    ui.painter().circle_filled(
                                        item_rect.left_center() + egui::vec2(44.0, 12.0),
                                        3.0,
                                        theme::green(),
                                    );
                                    ui.painter().text(
                                        item_rect.left_center() + egui::vec2(54.0, 12.0),
                                        egui::Align2::LEFT_CENTER,
                                        format!("{} · Active", device.device_type),
                                        egui::FontId::proportional(11.0),
                                        theme::green(),
                                    );
                                } else {
                                    ui.painter().text(
                                        item_rect.left_center() + egui::vec2(44.0, 12.0),
                                        egui::Align2::LEFT_CENTER,
                                        device.device_type.to_string(),
                                        egui::FontId::proportional(11.0),
                                        theme::text_dim(),
                                    );
                                }

                                if item_response.clicked() && !is_active {
                                    self.send_request(ClientRequest::Player(
                                        PlayerRequest::TransferPlayback(device.id.clone(), true),
                                    ));
                                    close_popup = true;
                                }

                                ui.add_space(2.0);
                            }
                        }
                    });

                    // Close on click outside
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        close_popup = true;
                    }
                });

            // Close popup if clicking outside the popup area
            if close_popup {
                self.show_device_popup = false;
            } else {
                let click_outside = ctx.input(|i| {
                    i.pointer.any_pressed()
                        && i.pointer
                            .latest_pos()
                            .map(|pos| {
                                pos.y < popup_y
                                    || pos.y > popup_y + popup_max_height
                                    || pos.x < popup_x
                                    || pos.x > popup_x + popup_width
                            })
                            .unwrap_or(false)
                });
                if click_outside {
                    self.show_device_popup = false;
                }
            }
        }

        // Left panel — sidebar with responsive width
        let mut action = Action::None;
        let is_authenticated = {
            let data = self.state.data.read();
            !data.user_data.playlists.is_empty() || !data.user_data.saved_albums.is_empty()
        };
        let window_width = ctx.screen_rect().width();
        let (sidebar_width, _) = theme::responsive_sidebar_width(window_width);
        egui::SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(sidebar_width)
            .frame(egui::Frame::new().fill(theme::bg_black()).inner_margin(egui::Margin::ZERO))
            .show(ctx, |ui| {
                action = sidebar::render(ui, &self.current_view, &self.state, is_authenticated, window_width);
            });
        self.handle_action(action);

        // Top panel — back/forward navigation buttons
        egui::TopBottomPanel::top("header_bar")
            .resizable(false)
            .exact_height(48.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(24.0);
                    
                    // Back button
                    let can_go_back = !self.view_history.is_empty();
                    let back_btn_size = egui::vec2(32.0, 32.0);
                    let (back_rect, back_resp) = ui.allocate_exact_size(back_btn_size, egui::Sense::click());
                    let back_bg = if !can_go_back {
                        theme::bg_dark()
                    } else if back_resp.hovered() {
                        theme::bg_hover()
                    } else {
                        theme::bg_card()
                    };
                    ui.painter().rect_filled(back_rect, theme::RADIUS_MEDIUM, back_bg);
                    let back_icon_color = if can_go_back { theme::text_primary() } else { theme::text_muted() };
                    ui.painter().text(
                        back_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        theme::ICON_BACK,
                        egui::FontId::proportional(16.0),
                        back_icon_color,
                    );
                    if back_resp.clicked() && can_go_back {
                        self.go_back();
                    }
                    
                    ui.add_space(8.0);
                    
                    // Forward button
                    let can_go_forward = !self.forward_history.is_empty();
                    let fwd_btn_size = egui::vec2(32.0, 32.0);
                    let (fwd_rect, fwd_resp) = ui.allocate_exact_size(fwd_btn_size, egui::Sense::click());
                    let fwd_bg = if !can_go_forward {
                        theme::bg_dark()
                    } else if fwd_resp.hovered() {
                        theme::bg_hover()
                    } else {
                        theme::bg_card()
                    };
                    ui.painter().rect_filled(fwd_rect, theme::RADIUS_MEDIUM, fwd_bg);
                    let fwd_icon_color = if can_go_forward { theme::text_primary() } else { theme::text_muted() };
                    ui.painter().text(
                        fwd_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "\u{2192}", // Right arrow
                        egui::FontId::proportional(16.0),
                        fwd_icon_color,
                    );
                    if fwd_resp.clicked() && can_go_forward {
                        self.navigate_forward();
                    }
                });
            });

        // Keyboard shortcuts (disabled when text input is focused)
        if !ctx.wants_keyboard_input() {
            // Check for raw key events to feed into the key sequence state machine
            let mut key_event: Option<(egui::Key, egui::Modifiers)> = None;
            ctx.input(|i| {
                for event in &i.events {
                    if let egui::Event::Key {
                        key,
                        modifiers,
                        pressed: true,
                        ..
                    } = event
                    {
                        key_event = Some((*key, *modifiers));
                        break;
                    }
                }
            });

            // Process raw special keys that bypass the sequence state machine
            let raw_keys = ctx.input(|i| {
                (
                    i.key_pressed(egui::Key::Escape),
                    i.modifiers.ctrl && i.key_pressed(egui::Key::ArrowUp),
                    i.modifiers.ctrl && i.key_pressed(egui::Key::ArrowDown),
                )
            });

            if raw_keys.0 {
                // Escape handling
                if self.key_seq_state.is_pending() {
                    self.key_seq_state.reset();
                } else if self.show_command_palette {
                    self.show_command_palette = false;
                } else if self.show_in_page_search {
                    self.show_in_page_search = false;
                    self.in_page_search_query.clear();
                } else if self.show_create_playlist_popup {
                    self.show_create_playlist_popup = false;
                } else if self.show_add_to_playlist_popup {
                    self.show_add_to_playlist_popup = false;
                } else if self.show_theme_switcher {
                    self.show_theme_switcher = false;
                } else if self.show_device_popup {
                    self.show_device_popup = false;
                } else if self.show_browse_playlists_popup {
                    self.show_browse_playlists_popup = false;
                } else if self.show_browse_artists_popup {
                    self.show_browse_artists_popup = false;
                } else if self.show_browse_albums_popup {
                    self.show_browse_albums_popup = false;
                } else if self.context_menu.is_open() {
                    self.context_menu.close();
                } else {
                    match self.current_view {
                        View::Tracks
                        | View::Artist
                        | View::BrowseCategory { .. }
                        | View::Lyrics
                        | View::Queue
                        | View::ShowDetail
                        | View::Help
                        | View::Settings => {
                            self.go_back();
                        }
                        _ => {}
                    }
                }
            }

            // Volume controls via Ctrl+Arrow (keep for compatibility)
            if raw_keys.1 {
                let vol = self
                    .state
                    .player
                    .read()
                    .playback
                    .as_ref()
                    .and_then(|p| p.device.volume_percent)
                    .unwrap_or(50) as u8;
                let new_vol = vol.saturating_add(5).min(100);
                let _ = self
                    .client_pub
                    .send(ClientRequest::Player(PlayerRequest::Volume(new_vol)));
            }
            if raw_keys.2 {
                let vol = self
                    .state
                    .player
                    .read()
                    .playback
                    .as_ref()
                    .and_then(|p| p.device.volume_percent)
                    .unwrap_or(50) as u8;
                let new_vol = vol.saturating_sub(5);
                let _ = self
                    .client_pub
                    .send(ClientRequest::Player(PlayerRequest::Volume(new_vol)));
            }

            // Process through key sequence state machine
            if let Some((key, modifiers)) = key_event {
                let (result, count) =
                    self.key_seq_state.process_key(key, modifiers, &self.keybindings);
                match result {
                    KeySequenceResult::Complete(cmd_id) => {
                        let count = count.unwrap_or(1);
                        if let Some((cmd, _)) = command::resolve_command(&cmd_id, count) {
                            self.execute_command(&cmd, count, ctx);
                        }
                    }
                    KeySequenceResult::Pending(_) => {
                        // Will be shown as a hint
                    }
                    KeySequenceResult::CountPending(_, _) => {
                        // Accumulating count
                    }
                    KeySequenceResult::None => {
                        // Unrecognized key, already reset
                    }
                }
            }
        }

        // Central panel — main content
        // Request lyrics when on Lyrics view
        if self.current_view == View::Lyrics {
            let player = self.state.player.read();
            if let Some(ref playback) = player.playback {
                if let Some(ref item) = playback.item {
                    let track_uri = match item {
                        rspotify::model::PlayableItem::Track(t) => t.id.as_ref().map(|id| id.uri()),
                        _ => None,
                    };
                    if let Some(uri) = track_uri {
                        let has_lyrics = self.state.data.read().caches.lyrics.contains_key(&uri);
                        if !has_lyrics {
                            if let Ok(track_id) = rspotify::model::TrackId::from_uri(&uri) {
                                self.send_request(ClientRequest::GetLyrics {
                                    track_id: track_id.into_static(),
                                });
                            }
                        }
                    }
                }
            }
        }

        let mut action = Action::None;
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(theme::bg_black())
                    .inner_margin(egui::Margin::ZERO),
            )
            .show(ctx, |ui| match self.current_view {
                View::Library => {
                    action = views::render_library(ui, &self.state, &mut self.image_cache, &mut self.context_menu, self.library_sort_order);
                }
                View::Tracks => {
                    let sort_action = views::render_tracks(
                        ui,
                        &self.state,
                        &self.client_pub,
                        &self.context_title,
                        &self.context_tracks,
                        &mut self.selected_track,
                        &mut self.image_cache,
                        &mut self.context_menu,
                        None,
                        self.sort_state,
                        self.current_context_id.as_ref(),
                        self.scroll_to_selected,
                    );
                    self.scroll_to_selected = false;
                    match sort_action {
                        SortAction::Sort(new_state) => {
                            self.sort_state = Some(new_state);
                            self.context_tracks.sort_by(|a, b| new_state.compare(a, b));
                            self.selected_track = None;
                        }
                        SortAction::None => {}
                    }
                }
                View::Search => {
                    action = views::render_search(
                        ui,
                        &self.state,
                        &self.client_pub,
                        &mut self.search_query,
                        &mut self.selected_track,
                        &mut self.image_cache,
                        &mut self.context_menu,
                        &mut self.search_debounce_state,
                    );
                }
                View::Browse => {
                    action = views::render_browse(
                        ui,
                        &self.state,
                        &self.client_pub,
                        &mut self.image_cache,
                        &mut self.context_menu,
                    );
                }
                View::BrowseCategory { ref id, ref name } => {
                    action = views::render_browse_category_playlists(
                        ui,
                        &self.state,
                        id,
                        name,
                        &mut self.image_cache,
                        &mut self.context_menu,
                    );
                }
                View::Queue => {
                    views::render_queue(ui, &self.state, &self.client_pub, &mut self.image_cache);
                }
                View::Settings => {
                    let settings_action = views::render_settings(
                        ui,
                        &mut self.settings_tab,
                        &mut self.settings_editing,
                        &mut self.settings_dirty,
                        &mut self.settings_keybinding_search,
                        &mut self.settings_editing_keybinding,
                        &mut self.settings_editing_keybindings,
                        &self.current_theme_name,
                        &self.client_pub,
                    );
                    match settings_action {
                        views::SettingsAction::Save => self.save_settings(),
                        views::SettingsAction::Reset => {
                            self.settings_editing = self.settings_original.clone();
                            self.settings_dirty = false;
                        }
                        views::SettingsAction::None => {}
                    }
                }
                View::Lyrics => {
                    action = views::render_lyrics(ui, &self.state, &self.client_pub, &mut self.image_cache);
                }
                View::Artist => {
                    action = views::render_artist(
                        ui,
                        &self.state,
                        &self.client_pub,
                        &self.artist_context,
                        &mut self.selected_artist_track,
                        &mut self.image_cache,
                        &mut self.context_menu,
                    );
                }
                View::Shows => {
                    action = views::render_shows(
                        ui,
                        &self.state,
                        &self.client_pub,
                        &mut self.image_cache,
                        &mut self.context_menu,
                    );
                }
                View::ShowDetail => {
                    action = views::render_show_detail(
                        ui,
                        &self.state,
                        &self.client_pub,
                        &self.podcast_detail_show,
                        &self.podcast_episodes,
                        &self.podcast_context_id,
                        &mut self.selected_podcast_episode,
                        &mut self.image_cache,
                        &mut self.context_menu,
                    );
                }
                View::Help => {
                    views::render_help(ui, &self.keybindings, &mut self.help_search);
                }
                View::Logs => {
                    views::render_logs(ui, &self.state);
                }
            });
        self.handle_action(action);

        // Render context menu overlay and handle navigation
        if let Some(nav) = self.context_menu.render(ctx, &self.state, &self.client_pub) {
            match nav {
                context_menu::Navigation::GoToArtist(artist) => {
                    self.context_menu.close();
                    self.handle_action(Action::ContextMenuNavigateArtist(artist));
                }
                context_menu::Navigation::GoToAlbum(album) => {
                    self.context_menu.close();
                    self.handle_action(Action::ContextMenuNavigateAlbum(album));
                }
                context_menu::Navigation::GoToShow(show) => {
                    self.context_menu.close();
                    self.handle_action(Action::ContextMenuNavigateShow(show));
                }
                context_menu::Navigation::GoToRadio(track) => {
                    self.context_menu.close();
                    let artist_name = track.artists.first().map(|a| a.name.clone()).unwrap_or_default();
                    let query = format!("artist:{}", artist_name);
                    self.send_request(ClientRequest::Search(query));
                    self.navigate_to_view(View::Search);
                    self.toast("Searching for radio...".to_string());
                }
                context_menu::Navigation::OpenAddToPlaylist(playable_id) => {
                    self.context_menu.close();
                    self.show_add_to_playlist_popup = true;
                    self.add_to_playlist_track = Some(playable_id);
                    self.add_to_playlist_filter.clear();
                }
            }
        }

        // Render Settings confirm dialog
        if self.show_settings_confirm_dialog {
            self.render_settings_confirm_dialog(ctx);
        }

        // Render Create Playlist popup
        if self.show_create_playlist_popup {
            self.render_create_playlist_popup(ctx);
        }

        // Render Add to Playlist popup
        if self.show_add_to_playlist_popup {
            self.render_add_to_playlist_popup(ctx);
        }

        // Render Theme Switcher popup
        if self.show_theme_switcher {
            self.render_theme_switcher(ctx);
        }

        // Render Browse User Playlists popup
        if self.show_browse_playlists_popup {
            self.render_browse_playlists_popup(ctx);
        }

        // Render Browse Followed Artists popup
        if self.show_browse_artists_popup {
            self.render_browse_artists_popup(ctx);
        }

        // Render Browse Saved Albums popup
        if self.show_browse_albums_popup {
            self.render_browse_albums_popup(ctx);
        }

        // Render In-page Search overlay
        if self.show_in_page_search {
            self.render_in_page_search(ctx);
        }

        // Render Onboarding modal
        if self.show_onboarding {
            self.render_onboarding(ctx);
        }

        // Render Auth modal
        if self.show_auth_modal {
            self.render_auth_modal(ctx);
        }

        // Render toast message
        self.render_toast(ctx);

        // Render key sequence hint
        self.render_key_hint(ctx);

        // Command Palette: Ctrl+Shift+P trigger (works even when text input is focused)
        let cmd_palette_triggered = ctx.input(|i| {
            i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::P)
        });
        if cmd_palette_triggered {
            self.show_command_palette = !self.show_command_palette;
            if self.show_command_palette {
                self.command_palette.open();
            }
        }

        // Settings save: Ctrl+S (works even when text input is focused)
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::S))
            && self.current_view == View::Settings && self.settings_dirty
        {
            self.save_settings();
        }

        // Command Palette: ':' trigger (only when no text input focused)
        if !self.show_command_palette && !ctx.wants_keyboard_input() {
            let colon_triggered = ctx.input(|i| {
                i.key_pressed(egui::Key::Colon)
            });
            if colon_triggered {
                self.show_command_palette = true;
                self.command_palette.open();
            }
        }

        // Render command palette
        if self.show_command_palette {
            if let Some(cmd_id) = self.command_palette.render(ctx) {
                if let Some((cmd, count)) = crate::command::resolve_command(&cmd_id, 1) {
                    self.command_palette.record_usage(&cmd_id);
                    self.execute_command(&cmd, count, ctx);
                }
                self.show_command_palette = false;
            }
            // Close if flag was cleared (e.g. by Escape inside render)
            // We need to check if the render returned None but should still be showing
        }
    }
}

impl SpotifyApp {
    fn play_track_from_context(&self, track: &state::Track) {
        use rspotify::prelude::Id;
        let playback = match self.current_context_id {
            Some(ref ctx) => match ctx {
                state::ContextId::Playlist(_)
                | state::ContextId::Album(_)
                | state::ContextId::Artist(_)
                | state::ContextId::Show(_) => state::Playback::Context(
                    ctx.clone(),
                    Some(rspotify::model::Offset::Uri(track.id.uri())),
                ),
                state::ContextId::Tracks(_) => {
                    let uris: Vec<PlayableId<'static>> = self
                        .context_tracks
                        .iter()
                        .map(|t| PlayableId::Track(t.id.clone()))
                        .collect();
                    state::Playback::URIs(
                        uris,
                        Some(rspotify::model::Offset::Uri(track.id.uri())),
                    )
                }
            },
            None => state::Playback::URIs(vec![PlayableId::Track(track.id.clone())], None),
        };
        self.send_request(ClientRequest::Player(
            PlayerRequest::StartPlayback(playback, None),
        ));
    }

    fn render_settings_confirm_dialog(&mut self, ctx: &egui::Context) {
        let dialog_width = 360.0;
        let dialog_height = 180.0;
        let screen = ctx.screen_rect();
        let dialog_pos = egui::pos2(
            screen.center().x - dialog_width / 2.0,
            screen.center().y - dialog_height / 2.0,
        );

        let mut close = false;
        let mut save = false;
        let mut discard = false;

        // Overlay background
        egui::Area::new(egui::Id::new("settings_confirm_overlay"))
            .order(egui::Order::Foreground)
            .fixed_pos(screen.min)
            .interactable(false)
            .show(ctx, |ui| {
                let (overlay_rect, _) = ui.allocate_exact_size(screen.size(), egui::Sense::hover());
                ui.painter().rect_filled(
                    overlay_rect,
                    0,
                    theme::with_alpha(theme::bg_black(), 150),
                );
            });

        egui::Area::new(egui::Id::new("settings_confirm_dialog"))
            .order(egui::Order::Foreground)
            .fixed_pos(dialog_pos)
            .show(ctx, |ui| {
                let frame = theme::glass_frame()
                    .inner_margin(egui::Margin::same(24));

                frame.show(ui, |ui| {
                    ui.set_min_width(dialog_width - 48.0);

                    // Title
                    ui.label(
                        egui::RichText::new("Save changes?")
                            .size(18.0)
                            .strong()
                            .color(theme::text_primary()),
                    );
                    ui.add_space(8.0);

                    // Message
                    ui.label(
                        egui::RichText::new("You have unsaved changes in Settings.\nDo you want to save them before leaving?")
                            .size(13.0)
                            .color(theme::text_secondary()),
                    );
                    ui.add_space(24.0);

                    // Buttons
                    ui.horizontal(|ui| {
                        // Cancel button
                        let (cancel_rect, cancel_resp) = ui
                            .allocate_exact_size(egui::vec2(90.0, 36.0), egui::Sense::click());
                        let cancel_bg = if cancel_resp.hovered() {
                            theme::bg_hover()
                        } else {
                            theme::bg_card()
                        };
                        ui.painter().rect_filled(
                            cancel_rect,
                            egui::CornerRadius::same(theme::RADIUS_MEDIUM),
                            cancel_bg,
                        );
                        ui.painter().rect_stroke(
                            cancel_rect,
                            egui::CornerRadius::same(theme::RADIUS_MEDIUM),
                            egui::Stroke::new(1.0, theme::divider()),
                            egui::StrokeKind::Outside,
                        );
                        ui.painter().text(
                            cancel_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "Cancel",
                            egui::FontId::proportional(13.0),
                            theme::text_primary(),
                        );
                        if cancel_resp.clicked() {
                            close = true;
                        }

                        ui.add_space(12.0);

                        // Discard button
                        let (discard_rect, discard_resp) = ui
                            .allocate_exact_size(egui::vec2(90.0, 36.0), egui::Sense::click());
                        let discard_bg = if discard_resp.hovered() {
                            theme::bg_hover()
                        } else {
                            theme::bg_card()
                        };
                        ui.painter().rect_filled(
                            discard_rect,
                            egui::CornerRadius::same(theme::RADIUS_MEDIUM),
                            discard_bg,
                        );
                        ui.painter().rect_stroke(
                            discard_rect,
                            egui::CornerRadius::same(theme::RADIUS_MEDIUM),
                            egui::Stroke::new(1.0, theme::divider()),
                            egui::StrokeKind::Outside,
                        );
                        ui.painter().text(
                            discard_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "Discard",
                            egui::FontId::proportional(13.0),
                            theme::text_primary(),
                        );
                        if discard_resp.clicked() {
                            discard = true;
                        }

                        ui.add_space(12.0);

                        // Save button
                        let (save_rect, save_resp) = ui
                            .allocate_exact_size(egui::vec2(90.0, 36.0), egui::Sense::click());
                        let save_bg = if save_resp.hovered() {
                            theme::green_hover()
                        } else {
                            theme::green()
                        };
                        ui.painter().rect_filled(
                            save_rect,
                            egui::CornerRadius::same(theme::RADIUS_MEDIUM),
                            save_bg,
                        );
                        ui.painter().text(
                            save_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "Save",
                            egui::FontId::proportional(13.0),
                            theme::bg_black(),
                        );
                        if save_resp.clicked() {
                            save = true;
                        }
                    });
                });

                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    close = true;
                }
            });

        // Handle close
        if close {
            self.show_settings_confirm_dialog = false;
            self.pending_view_navigation = None;
        }

        // Handle discard
        if discard {
            self.settings_editing = self.settings_original.clone();
            self.settings_dirty = false;
            self.show_settings_confirm_dialog = false;
            if let Some(view) = self.pending_view_navigation.take() {
                self.navigate_to_view(view);
            }
        }

        // Handle save
        if save {
            self.save_settings();
            self.show_settings_confirm_dialog = false;
            if let Some(view) = self.pending_view_navigation.take() {
                self.navigate_to_view(view);
            }
        }
    }

    fn render_create_playlist_popup(&mut self, ctx: &egui::Context) {
        let popup_width = 380.0;
        let popup_height = 340.0;
        let screen = ctx.screen_rect();
        let popup_pos = egui::pos2(
            screen.center().x - popup_width / 2.0,
            screen.center().y - popup_height / 2.0,
        );

        let mut close = false;
        let mut create = false;

        // Overlay background
        egui::Area::new(egui::Id::new("create_playlist_overlay"))
            .order(egui::Order::Foreground)
            .fixed_pos(screen.min)
            .interactable(false)
            .show(ctx, |ui| {
                let (overlay_rect, _) = ui.allocate_exact_size(screen.size(), egui::Sense::hover());
                ui.painter().rect_filled(
                    overlay_rect,
                    0,
                    theme::with_alpha(theme::bg_black(), 120),
                );
            });

        egui::Area::new(egui::Id::new("create_playlist_popup"))
            .order(egui::Order::Foreground)
            .fixed_pos(popup_pos)
            .show(ctx, |ui| {
                let frame = theme::glass_frame()
                    .inner_margin(egui::Margin::same(20));

                frame.show(ui, |ui| {
                    ui.set_min_width(popup_width - 40.0);

                    ui.label(
                        egui::RichText::new("Create Playlist")
                            .size(18.0)
                            .strong()
                            .color(theme::text_primary()),
                    );
                    ui.add_space(16.0);

                    // Name input with validation
                    ui.label(
                        egui::RichText::new("Name")
                            .size(12.0)
                            .color(theme::text_dim()),
                    );
                    ui.add_space(4.0);
                    
                    // Real-time validation
                    let name_trimmed = self.create_playlist_name.trim();
                    if name_trimmed.is_empty() {
                        self.create_playlist_name_error = Some("Playlist name is required".to_string());
                    } else if name_trimmed.len() > 100 {
                        self.create_playlist_name_error = Some("Name must be 100 characters or less".to_string());
                    } else if name_trimmed.chars().any(|c| c.is_control()) {
                        self.create_playlist_name_error = Some("Name contains invalid characters".to_string());
                    } else {
                        self.create_playlist_name_error = None;
                    }
                    
                    let name_input = egui::TextEdit::singleline(&mut self.create_playlist_name)
                        .desired_width(f32::INFINITY)
                        .hint_text("Playlist name")
                        .font(egui::FontId::proportional(13.0))
                        .margin(egui::Margin::symmetric(10, 8))
                        .background_color(if self.create_playlist_name_error.is_some() {
                            theme::with_alpha(theme::error_color(), 30)
                        } else {
                            theme::bg_input()
                        });
                    let _name_response = ui.add(name_input);
                    
                    // Show validation error
                    if let Some(ref error) = self.create_playlist_name_error {
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(format!("⚠️ {}", error))
                                .size(11.0)
                                .color(theme::error_color()),
                        );
                    }
                    ui.add_space(10.0);

                    // Description input
                    ui.label(
                        egui::RichText::new("Description")
                            .size(12.0)
                            .color(theme::text_dim()),
                    );
                    ui.add_space(4.0);
                    let desc_input = egui::TextEdit::singleline(&mut self.create_playlist_desc)
                        .desired_width(f32::INFINITY)
                        .hint_text("Optional description")
                        .font(egui::FontId::proportional(13.0))
                        .margin(egui::Margin::symmetric(10, 8))
                        .background_color(theme::bg_input());
                    ui.add(desc_input);
                    ui.add_space(12.0);

                    // Toggles
                    ui.horizontal(|ui| {
                        // Public toggle
                        let toggle_size = egui::vec2(36.0, 20.0);
                        let (toggle_rect, toggle_resp) =
                            ui.allocate_exact_size(toggle_size, egui::Sense::click());
                        let toggle_bg = if self.create_playlist_public {
                            theme::green()
                        } else {
                            theme::bg_active()
                        };
                        ui.painter().rect_filled(
                            toggle_rect,
                            egui::CornerRadius::same(10),
                            toggle_bg,
                        );
                        let knob_x = if self.create_playlist_public {
                            toggle_rect.right() - 8.0
                        } else {
                            toggle_rect.left() + 8.0
                        };
                        ui.painter().circle_filled(
                            egui::pos2(knob_x, toggle_rect.center().y),
                            7.0,
                            theme::foreground(),
                        );
                        if toggle_resp.clicked() {
                            self.create_playlist_public = !self.create_playlist_public;
                        }
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new("Public")
                                .size(13.0)
                                .color(theme::text_secondary()),
                        );

                        ui.add_space(20.0);

                        // Collaborative toggle
                        let (toggle_rect2, toggle_resp2) =
                            ui.allocate_exact_size(toggle_size, egui::Sense::click());
                        let toggle_bg2 = if self.create_playlist_collab {
                            theme::green()
                        } else {
                            theme::bg_active()
                        };
                        ui.painter().rect_filled(
                            toggle_rect2,
                            egui::CornerRadius::same(10),
                            toggle_bg2,
                        );
                        let knob_x2 = if self.create_playlist_collab {
                            toggle_rect2.right() - 8.0
                        } else {
                            toggle_rect2.left() + 8.0
                        };
                        ui.painter().circle_filled(
                            egui::pos2(knob_x2, toggle_rect2.center().y),
                            7.0,
                            theme::foreground(),
                        );
                        if toggle_resp2.clicked() {
                            self.create_playlist_collab = !self.create_playlist_collab;
                        }
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new("Collaborative")
                                .size(13.0)
                                .color(theme::text_secondary()),
                        );
                    });

                    ui.add_space(20.0);

                    // Buttons
                    ui.horizontal(|ui| {
                        // Cancel button
                        let (cancel_rect, cancel_resp) = ui
                            .allocate_exact_size(egui::vec2(100.0, 36.0), egui::Sense::click());
                        let cancel_bg = if cancel_resp.hovered() {
                            theme::bg_hover()
                        } else {
                            theme::bg_card()
                        };
                        ui.painter().rect_filled(
                            cancel_rect,
                            egui::CornerRadius::same(theme::RADIUS_MEDIUM),
                            cancel_bg,
                        );
                        ui.painter().text(
                            cancel_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "Cancel",
                            egui::FontId::proportional(13.0),
                            theme::text_primary(),
                        );
                        if cancel_resp.clicked() {
                            close = true;
                        }

                        ui.add_space(12.0);

                        // Create button
                        let can_create = self.create_playlist_name_error.is_none() && !self.create_playlist_name.trim().is_empty();
                        let (create_rect, create_resp) = ui
                            .allocate_exact_size(egui::vec2(100.0, 36.0), egui::Sense::click());
                        let create_bg = if !can_create {
                            theme::bg_dark()
                        } else if create_resp.hovered() {
                            theme::green_hover()
                        } else {
                            theme::green()
                        };
                        ui.painter().rect_filled(
                            create_rect,
                            egui::CornerRadius::same(theme::RADIUS_MEDIUM),
                            create_bg,
                        );
                        let create_text_color = if can_create {
                            theme::bg_black()
                        } else {
                            theme::text_muted()
                        };
                        ui.painter().text(
                            create_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "Create",
                            egui::FontId::proportional(13.0),
                            create_text_color,
                        );
                        if create_resp.clicked() && can_create {
                            create = true;
                        }
                    });
                });

                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    close = true;
                }
            });

        // Handle click outside to close
        if !close {
            let click_outside = ctx.input(|i| {
                i.pointer.any_pressed()
                    && i.pointer
                        .latest_pos()
                        .map(|pos| {
                            pos.x < popup_pos.x
                                || pos.x > popup_pos.x + popup_width
                                || pos.y < popup_pos.y
                                || pos.y > popup_pos.y + popup_height
                        })
                        .unwrap_or(false)
            });
            if click_outside {
                close = true;
            }
        }

        if create {
            self.send_request(ClientRequest::CreatePlaylist {
                playlist_name: self.create_playlist_name.trim().to_string(),
                public: self.create_playlist_public,
                collab: self.create_playlist_collab,
                desc: self.create_playlist_desc.trim().to_string(),
            });
            self.toast("Playlist created".to_string());
            close = true;
        }

        if close {
            self.show_create_playlist_popup = false;
            self.create_playlist_name_error = None;
        }
    }

    fn render_add_to_playlist_popup(&mut self, ctx: &egui::Context) {
        let popup_width = 360.0;
        let popup_height = 420.0;
        let screen = ctx.screen_rect();
        let popup_pos = egui::pos2(
            screen.center().x - popup_width / 2.0,
            screen.center().y - popup_height / 2.0,
        );

        let mut close = false;
        let mut selected_playlist_id: Option<state::PlaylistId<'static>> = None;

        // Overlay background
        egui::Area::new(egui::Id::new("add_to_playlist_overlay"))
            .order(egui::Order::Foreground)
            .fixed_pos(screen.min)
            .interactable(false)
            .show(ctx, |ui| {
                let (overlay_rect, _) = ui.allocate_exact_size(screen.size(), egui::Sense::hover());
                ui.painter().rect_filled(
                    overlay_rect,
                    0,
                    theme::with_alpha(theme::bg_black(), 120),
                );
            });

        egui::Area::new(egui::Id::new("add_to_playlist_popup"))
            .order(egui::Order::Foreground)
            .fixed_pos(popup_pos)
            .show(ctx, |ui| {
                let frame = theme::glass_frame()
                    .inner_margin(egui::Margin::same(16));

                frame.show(ui, |ui| {
                    ui.set_min_width(popup_width - 32.0);

                    // Header
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Add to Playlist")
                                .size(16.0)
                                .strong()
                                .color(theme::text_primary()),
                        );
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("\u{2715}")
                                                .size(14.0)
                                                .color(theme::text_dim()),
                                        )
                                        .fill(egui::Color32::TRANSPARENT),
                                    )
                                    .clicked()
                                {
                                    close = true;
                                }
                            },
                        );
                    });
                    ui.add_space(10.0);

                    // Search/filter input
                    let filter_input = egui::TextEdit::singleline(&mut self.add_to_playlist_filter)
                        .desired_width(f32::INFINITY)
                        .hint_text("Search playlists...")
                        .font(egui::FontId::proportional(13.0))
                        .margin(egui::Margin::symmetric(10, 8))
                        .background_color(theme::bg_input());
                    ui.add(filter_input);
                    ui.add_space(8.0);

                    // Playlist list
                    let filter = self.add_to_playlist_filter.to_lowercase();
                    let data = self.state.data.read();
                    let playlists: Vec<_> = data
                        .user_data
                        .playlists
                        .iter()
                        .filter_map(|item| match item {
                            state::PlaylistFolderItem::Playlist(p) => {
                                if filter.is_empty()
                                    || p.name.to_lowercase().contains(&filter)
                                    || p.owner.0.to_lowercase().contains(&filter)
                                {
                                    Some(p.clone())
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        })
                        .collect();
                    drop(data);

                    egui::ScrollArea::vertical()
                        .id_salt("add_to_playlist_list")
                        .max_height(popup_height - 140.0)
                        .show(ui, |ui| {
                            if playlists.is_empty() {
                                ui.add_space(20.0);
                                ui.label(
                                    egui::RichText::new("No playlists found")
                                        .size(12.0)
                                        .color(theme::text_dim()),
                                );
                            }
                            for playlist in &playlists {
                                let (item_rect, item_resp) = ui
                                    .allocate_exact_size(
                                        egui::vec2(ui.available_width(), 44.0),
                                        egui::Sense::click(),
                                    );

                                let bg = if item_resp.hovered() {
                                    theme::bg_hover()
                                } else {
                                    egui::Color32::TRANSPARENT
                                };
                                ui.painter().rect_filled(
                                    item_rect,
                                    egui::CornerRadius::same(theme::RADIUS_SMALL),
                                    bg,
                                );

                                let text_color = if item_resp.hovered() {
                                    theme::text_primary()
                                } else {
                                    theme::text_secondary()
                                };

                                ui.painter().text(
                                    item_rect.left_center() + egui::vec2(8.0, -6.0),
                                    egui::Align2::LEFT_CENTER,
                                    &playlist.name,
                                    egui::FontId::proportional(13.0),
                                    text_color,
                                );
                                ui.painter().text(
                                    item_rect.left_center() + egui::vec2(8.0, 10.0),
                                    egui::Align2::LEFT_CENTER,
                                    &playlist.owner.0,
                                    egui::FontId::proportional(11.0),
                                    theme::text_dim(),
                                );

                                if item_resp.clicked() {
                                    selected_playlist_id = Some(playlist.id.clone());
                                }
                            }
                        });
                });

                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    close = true;
                }
            });

        // Handle click outside
        if !close {
            let click_outside = ctx.input(|i| {
                i.pointer.any_pressed()
                    && i.pointer
                        .latest_pos()
                        .map(|pos| {
                            pos.x < popup_pos.x
                                || pos.x > popup_pos.x + popup_width
                                || pos.y < popup_pos.y
                                || pos.y > popup_pos.y + popup_height
                        })
                        .unwrap_or(false)
            });
            if click_outside {
                close = true;
            }
        }

        if let Some(playlist_id) = selected_playlist_id {
            if let Some(ref playable_id) = self.add_to_playlist_track {
                self.send_request(ClientRequest::AddPlayableToPlaylist(
                    playlist_id,
                    playable_id.clone(),
                ));
                self.toast("Added to playlist".to_string());
            }
            close = true;
        }

        if close {
            self.show_add_to_playlist_popup = false;
            self.add_to_playlist_track = None;
            self.add_to_playlist_filter.clear();
        }
    }

    fn render_theme_switcher(&mut self, ctx: &egui::Context) {
        let popup_width = 340.0;
        let popup_height = 420.0;
        let screen = ctx.screen_rect();
        let popup_pos = egui::pos2(
            screen.center().x - popup_width / 2.0,
            screen.center().y - popup_height / 2.0,
        );

        let mut close = false;
        let mut selected_theme: Option<String> = None;

        // Collect built-in themes and custom themes
        let built_in = theme::built_in_themes();
        let config = crate::config::get_config();
        let custom_themes: Vec<_> = config.theme_config.themes.clone();

        // Overlay background
        egui::Area::new(egui::Id::new("theme_switcher_overlay"))
            .order(egui::Order::Foreground)
            .fixed_pos(screen.min)
            .interactable(false)
            .show(ctx, |ui| {
                let (overlay_rect, _) = ui.allocate_exact_size(screen.size(), egui::Sense::hover());
                ui.painter().rect_filled(
                    overlay_rect,
                    0,
                    theme::with_alpha(theme::bg_black(), 120),
                );
            });

        egui::Area::new(egui::Id::new("theme_switcher"))
            .order(egui::Order::Foreground)
            .fixed_pos(popup_pos)
            .show(ctx, |ui| {
                let frame = theme::glass_frame()
                    .inner_margin(egui::Margin::same(16));

                frame.show(ui, |ui| {
                    ui.set_min_width(popup_width - 32.0);

                    // Header
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Switch Theme")
                                .size(16.0)
                                .strong()
                                .color(theme::text_primary()),
                        );
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("\u{2715}")
                                                .size(14.0)
                                                .color(theme::text_dim()),
                                        )
                                        .fill(egui::Color32::TRANSPARENT),
                                    )
                                    .clicked()
                                {
                                    close = true;
                                }
                            },
                        );
                    });
                    ui.add_space(10.0);

                    // Search input
                    let filter_input = egui::TextEdit::singleline(&mut self.theme_search)
                        .desired_width(f32::INFINITY)
                        .hint_text("Search themes...")
                        .font(egui::FontId::proportional(13.0))
                        .margin(egui::Margin::symmetric(10, 8))
                        .background_color(theme::bg_input());
                    ui.add(filter_input);
                    ui.add_space(8.0);

                    // Current theme indicator
                    ui.label(
                        egui::RichText::new(format!("Current: {}", self.current_theme_name))
                            .size(11.0)
                            .color(theme::text_dim()),
                    );
                    ui.add_space(8.0);

                    // Theme list
                    let filter = self.theme_search.to_lowercase();
                    let list_max_height = popup_height - 160.0;

                    egui::ScrollArea::vertical()
                        .id_salt("theme_list")
                        .max_height(list_max_height)
                        .show(ui, |ui| {
                            // Built-in themes section
                            if filter.is_empty() || "built-in".contains(&filter) {
                                ui.label(
                                    egui::RichText::new("Built-in")
                                        .size(11.0)
                                        .color(theme::text_dim()),
                                );
                                ui.add_space(4.0);
                            }

                            for builtin in &built_in {
                                if !filter.is_empty()
                                    && !builtin.name.to_lowercase().contains(&filter)
                                {
                                    continue;
                                }
                                let is_current = self.current_theme_name.eq_ignore_ascii_case(builtin.name);
                                let (item_rect, item_resp) = ui
                                    .allocate_exact_size(
                                        egui::vec2(ui.available_width(), 36.0),
                                        egui::Sense::click(),
                                    );

                                let bg = if is_current {
                                    theme::bg_active()
                                } else if item_resp.hovered() {
                                    theme::bg_hover()
                                } else {
                                    egui::Color32::TRANSPARENT
                                };
                                ui.painter().rect_filled(
                                    item_rect,
                                    egui::CornerRadius::same(theme::RADIUS_SMALL),
                                    bg,
                                );

                                // Color preview swatches
                                let swatch_size = 10.0;
                                let swatch_y = item_rect.center().y - swatch_size / 2.0;
                                let gui_palette = theme::GuiPalette::from_config_palette(&builtin.palette);
                                let swatches = [
                                    gui_palette.accent,
                                    gui_palette.bg_dark,
                                    gui_palette.text_primary,
                                ];
                                for (j, color) in swatches.iter().enumerate() {
                                    let swatch_rect = egui::Rect::from_min_size(
                                        egui::pos2(item_rect.left() + 8.0 + j as f32 * 14.0, swatch_y),
                                        egui::vec2(swatch_size, swatch_size),
                                    );
                                    ui.painter().rect_filled(swatch_rect, 2.0, *color);
                                }

                                let name_color = if is_current {
                                    theme::green()
                                } else if item_resp.hovered() {
                                    theme::text_primary()
                                } else {
                                    theme::text_secondary()
                                };
                                ui.painter().text(
                                    item_rect.left_center() + egui::vec2(56.0, 0.0),
                                    egui::Align2::LEFT_CENTER,
                                    builtin.name,
                                    egui::FontId::proportional(13.0),
                                    name_color,
                                );

                                if is_current {
                                    ui.painter().text(
                                        item_rect.right_center() + egui::vec2(-12.0, 0.0),
                                        egui::Align2::RIGHT_CENTER,
                                        "\u{2713}",
                                        egui::FontId::proportional(14.0),
                                        theme::green(),
                                    );
                                }

                                if item_resp.clicked() && !is_current {
                                    selected_theme = Some(builtin.name.to_string());
                                }
                            }

                            // Custom themes section
                            if !custom_themes.is_empty() {
                                ui.add_space(8.0);
                                if filter.is_empty() || "custom".contains(&filter) {
                                    ui.label(
                                        egui::RichText::new("Custom")
                                            .size(11.0)
                                            .color(theme::text_dim()),
                                    );
                                    ui.add_space(4.0);
                                }

                                for custom_theme in &custom_themes {
                                    if !filter.is_empty()
                                        && !custom_theme.name.to_lowercase().contains(&filter)
                                    {
                                        continue;
                                    }
                                    let is_current = self.current_theme_name.eq_ignore_ascii_case(&custom_theme.name);
                                    let (item_rect, item_resp) = ui
                                        .allocate_exact_size(
                                            egui::vec2(ui.available_width(), 36.0),
                                            egui::Sense::click(),
                                        );

                                    let bg = if is_current {
                                        theme::bg_active()
                                    } else if item_resp.hovered() {
                                        theme::bg_hover()
                                    } else {
                                        egui::Color32::TRANSPARENT
                                    };
                                    ui.painter().rect_filled(
                                        item_rect,
                                        egui::CornerRadius::same(theme::RADIUS_SMALL),
                                        bg,
                                    );

                                    // Color preview swatches
                                    let swatch_size = 10.0;
                                    let swatch_y = item_rect.center().y - swatch_size / 2.0;
                                    let gui_palette = theme::GuiPalette::from_config_palette(&custom_theme.palette);
                                    let swatches = [
                                        gui_palette.accent,
                                        gui_palette.bg_dark,
                                        gui_palette.text_primary,
                                    ];
                                    for (j, color) in swatches.iter().enumerate() {
                                        let swatch_rect = egui::Rect::from_min_size(
                                            egui::pos2(item_rect.left() + 8.0 + j as f32 * 14.0, swatch_y),
                                            egui::vec2(swatch_size, swatch_size),
                                        );
                                        ui.painter().rect_filled(swatch_rect, 2.0, *color);
                                    }

                                    let name_color = if is_current {
                                        theme::green()
                                    } else if item_resp.hovered() {
                                        theme::text_primary()
                                    } else {
                                        theme::text_secondary()
                                    };
                                    ui.painter().text(
                                        item_rect.left_center() + egui::vec2(56.0, 0.0),
                                        egui::Align2::LEFT_CENTER,
                                        &custom_theme.name,
                                        egui::FontId::proportional(13.0),
                                        name_color,
                                    );

                                    if is_current {
                                        ui.painter().text(
                                            item_rect.right_center() + egui::vec2(-12.0, 0.0),
                                            egui::Align2::RIGHT_CENTER,
                                            "\u{2713}",
                                            egui::FontId::proportional(14.0),
                                            theme::green(),
                                        );
                                    }

                                    if item_resp.clicked() && !is_current {
                                        selected_theme = Some(custom_theme.name.clone());
                                    }
                                }
                            }
                        });
                });

                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    close = true;
                }
            });

        // Handle click outside
        if !close {
            let click_outside = ctx.input(|i| {
                i.pointer.any_pressed()
                    && i.pointer
                        .latest_pos()
                        .map(|pos| {
                            pos.x < popup_pos.x
                                || pos.x > popup_pos.x + popup_width
                                || pos.y < popup_pos.y
                                || pos.y > popup_pos.y + popup_height
                        })
                        .unwrap_or(false)
            });
            if click_outside {
                close = true;
            }
        }

        if let Some(theme_name) = selected_theme {
            // Check built-in first
            if let Some(builtin) = built_in.iter().find(|t| t.name.eq_ignore_ascii_case(&theme_name)) {
                theme::set_palette_from_config(&builtin.palette);
                self.current_theme_name = builtin.name.to_string();
            } else if let Some(custom) = custom_themes.iter().find(|t| t.name.eq_ignore_ascii_case(&theme_name)) {
                theme::set_palette_from_config(&custom.palette);
                self.current_theme_name = custom.name.clone();
            }
            // Re-apply theme to egui
            theme::setup_theme(ctx);
            self.toast(format!("Theme: {}", self.current_theme_name));
            close = true;
        }

        if close {
            self.show_theme_switcher = false;
            self.theme_search.clear();
        }
    }

    fn render_browse_playlists_popup(&mut self, ctx: &egui::Context) {
        let popup_width = 360.0;
        let popup_height = 420.0;
        let screen = ctx.screen_rect();
        let popup_pos = egui::pos2(
            screen.center().x - popup_width / 2.0,
            screen.center().y - popup_height / 2.0,
        );

        let mut close = false;
        let mut open_playlist_idx: Option<usize> = None;

        egui::Area::new(egui::Id::new("browse_playlists_overlay"))
            .order(egui::Order::Foreground)
            .fixed_pos(screen.min)
            .interactable(false)
            .show(ctx, |ui| {
                let (overlay_rect, _) = ui.allocate_exact_size(screen.size(), egui::Sense::hover());
                ui.painter().rect_filled(overlay_rect, 0, theme::with_alpha(theme::bg_black(), 120));
            });

        egui::Area::new(egui::Id::new("browse_playlists_popup"))
            .order(egui::Order::Foreground)
            .fixed_pos(popup_pos)
            .show(ctx, |ui| {
                let frame = theme::glass_frame().inner_margin(egui::Margin::same(16));
                frame.show(ui, |ui| {
                    ui.set_min_width(popup_width - 32.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Your Playlists").size(16.0).strong().color(theme::text_primary()));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(egui::Button::new(egui::RichText::new("\u{2715}").size(14.0).color(theme::text_dim())).fill(egui::Color32::TRANSPARENT)).clicked() {
                                close = true;
                            }
                        });
                    });
                    ui.add_space(10.0);
                    let filter_input = egui::TextEdit::singleline(&mut self.browse_popup_filter)
                        .desired_width(f32::INFINITY)
                        .hint_text("Filter playlists...")
                        .font(egui::FontId::proportional(13.0))
                        .margin(egui::Margin::symmetric(10, 8))
                        .background_color(theme::bg_input());
                    ui.add(filter_input);
                    ui.add_space(8.0);

                    let filter = self.browse_popup_filter.to_lowercase();
                    let data = self.state.data.read();
                    let playlists: Vec<_> = data.user_data.playlists.iter().enumerate()
                        .filter_map(|(i, item)| match item {
                            state::PlaylistFolderItem::Playlist(p) => {
                                if filter.is_empty() || p.name.to_lowercase().contains(&filter) {
                                    Some((i, p.clone()))
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        })
                        .collect();
                    drop(data);

                    egui::ScrollArea::vertical().id_salt("browse_playlists_list").max_height(popup_height - 140.0).show(ui, |ui| {
                        if playlists.is_empty() {
                            ui.add_space(20.0);
                            ui.label(egui::RichText::new("No playlists found").size(12.0).color(theme::text_dim()));
                        }
                        for (i, playlist) in &playlists {
                            let (item_rect, item_resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 40.0), egui::Sense::click());
                            let bg = if item_resp.hovered() { theme::bg_hover() } else { egui::Color32::TRANSPARENT };
                            ui.painter().rect_filled(item_rect, 4.0, bg);
                            ui.painter().text(item_rect.left_center() + egui::vec2(12.0, 0.0), egui::Align2::LEFT_CENTER, &playlist.name, egui::FontId::proportional(13.0), if item_resp.hovered() { theme::text_primary() } else { theme::text_secondary() });
                            if item_resp.clicked() {
                                open_playlist_idx = Some(*i);
                            }
                        }
                    });
                });
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) { close = true; }
            });

        if !close {
            let click_outside = ctx.input(|i| {
                i.pointer.any_pressed() && i.pointer.latest_pos().map(|pos| {
                    pos.x < popup_pos.x || pos.x > popup_pos.x + popup_width || pos.y < popup_pos.y || pos.y > popup_pos.y + popup_height
                }).unwrap_or(false)
            });
            if click_outside { close = true; }
        }

        if let Some(idx) = open_playlist_idx {
            self.handle_action(Action::OpenPlaylist(idx));
            close = true;
        }

        if close {
            self.show_browse_playlists_popup = false;
            self.browse_popup_filter.clear();
        }
    }

    fn render_browse_artists_popup(&mut self, ctx: &egui::Context) {
        let popup_width = 360.0;
        let popup_height = 420.0;
        let screen = ctx.screen_rect();
        let popup_pos = egui::pos2(screen.center().x - popup_width / 2.0, screen.center().y - popup_height / 2.0);

        let mut close = false;
        let mut open_artist: Option<state::Artist> = None;

        egui::Area::new(egui::Id::new("browse_artists_overlay"))
            .order(egui::Order::Foreground)
            .fixed_pos(screen.min).interactable(false)
            .show(ctx, |ui| {
                let (overlay_rect, _) = ui.allocate_exact_size(screen.size(), egui::Sense::hover());
                ui.painter().rect_filled(overlay_rect, 0, theme::with_alpha(theme::bg_black(), 120));
            });

        egui::Area::new(egui::Id::new("browse_artists_popup"))
            .order(egui::Order::Foreground)
            .fixed_pos(popup_pos)
            .show(ctx, |ui| {
                let frame = theme::glass_frame().inner_margin(egui::Margin::same(16));
                frame.show(ui, |ui| {
                    ui.set_min_width(popup_width - 32.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Followed Artists").size(16.0).strong().color(theme::text_primary()));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(egui::Button::new(egui::RichText::new("\u{2715}").size(14.0).color(theme::text_dim())).fill(egui::Color32::TRANSPARENT)).clicked() { close = true; }
                        });
                    });
                    ui.add_space(10.0);
                    let filter_input = egui::TextEdit::singleline(&mut self.browse_popup_filter)
                        .desired_width(f32::INFINITY).hint_text("Filter artists...").font(egui::FontId::proportional(13.0))
                        .margin(egui::Margin::symmetric(10, 8)).background_color(theme::bg_input());
                    ui.add(filter_input);
                    ui.add_space(8.0);

                    let filter = self.browse_popup_filter.to_lowercase();
                    let data = self.state.data.read();
                    let artists: Vec<_> = data.user_data.followed_artists.iter()
                        .filter(|a| filter.is_empty() || a.name.to_lowercase().contains(&filter))
                        .cloned().collect();
                    drop(data);

                    egui::ScrollArea::vertical().id_salt("browse_artists_list").max_height(popup_height - 140.0).show(ui, |ui| {
                        if artists.is_empty() {
                            ui.add_space(20.0);
                            ui.label(egui::RichText::new("No artists found").size(12.0).color(theme::text_dim()));
                        }
                        for artist in &artists {
                            let (item_rect, item_resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 40.0), egui::Sense::click());
                            let bg = if item_resp.hovered() { theme::bg_hover() } else { egui::Color32::TRANSPARENT };
                            ui.painter().rect_filled(item_rect, 4.0, bg);
                            ui.painter().text(item_rect.left_center() + egui::vec2(12.0, 0.0), egui::Align2::LEFT_CENTER, &artist.name, egui::FontId::proportional(13.0), if item_resp.hovered() { theme::text_primary() } else { theme::text_secondary() });
                            if item_resp.clicked() {
                                open_artist = Some(artist.clone());
                            }
                        }
                    });
                });
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) { close = true; }
            });

        if !close {
            let click_outside = ctx.input(|i| {
                i.pointer.any_pressed() && i.pointer.latest_pos().map(|pos| {
                    pos.x < popup_pos.x || pos.x > popup_pos.x + popup_width || pos.y < popup_pos.y || pos.y > popup_pos.y + popup_height
                }).unwrap_or(false)
            });
            if click_outside { close = true; }
        }

        if let Some(artist) = open_artist {
            self.handle_action(Action::OpenArtist(artist));
            close = true;
        }

        if close {
            self.show_browse_artists_popup = false;
            self.browse_popup_filter.clear();
        }
    }

    fn render_browse_albums_popup(&mut self, ctx: &egui::Context) {
        let popup_width = 360.0;
        let popup_height = 420.0;
        let screen = ctx.screen_rect();
        let popup_pos = egui::pos2(screen.center().x - popup_width / 2.0, screen.center().y - popup_height / 2.0);

        let mut close = false;
        let mut open_album_idx: Option<usize> = None;

        egui::Area::new(egui::Id::new("browse_albums_overlay"))
            .order(egui::Order::Foreground)
            .fixed_pos(screen.min).interactable(false)
            .show(ctx, |ui| {
                let (overlay_rect, _) = ui.allocate_exact_size(screen.size(), egui::Sense::hover());
                ui.painter().rect_filled(overlay_rect, 0, theme::with_alpha(theme::bg_black(), 120));
            });

        egui::Area::new(egui::Id::new("browse_albums_popup"))
            .order(egui::Order::Foreground)
            .fixed_pos(popup_pos)
            .show(ctx, |ui| {
                let frame = theme::glass_frame().inner_margin(egui::Margin::same(16));
                frame.show(ui, |ui| {
                    ui.set_min_width(popup_width - 32.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Saved Albums").size(16.0).strong().color(theme::text_primary()));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(egui::Button::new(egui::RichText::new("\u{2715}").size(14.0).color(theme::text_dim())).fill(egui::Color32::TRANSPARENT)).clicked() { close = true; }
                        });
                    });
                    ui.add_space(10.0);
                    let filter_input = egui::TextEdit::singleline(&mut self.browse_popup_filter)
                        .desired_width(f32::INFINITY).hint_text("Filter albums...").font(egui::FontId::proportional(13.0))
                        .margin(egui::Margin::symmetric(10, 8)).background_color(theme::bg_input());
                    ui.add(filter_input);
                    ui.add_space(8.0);

                    let filter = self.browse_popup_filter.to_lowercase();
                    let data = self.state.data.read();
                    let albums: Vec<_> = data.user_data.saved_albums.iter().enumerate()
                        .filter(|(_, a)| filter.is_empty() || a.name.to_lowercase().contains(&filter))
                        .map(|(i, a)| (i, a.clone()))
                        .collect();
                    drop(data);

                    egui::ScrollArea::vertical().id_salt("browse_albums_list").max_height(popup_height - 140.0).show(ui, |ui| {
                        if albums.is_empty() {
                            ui.add_space(20.0);
                            ui.label(egui::RichText::new("No albums found").size(12.0).color(theme::text_dim()));
                        }
                        for (i, album) in &albums {
                            let (item_rect, item_resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 40.0), egui::Sense::click());
                            let bg = if item_resp.hovered() { theme::bg_hover() } else { egui::Color32::TRANSPARENT };
                            ui.painter().rect_filled(item_rect, 4.0, bg);
                            let label = format!("{} — {}", album.name, album.artists.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", "));
                            ui.painter().text(item_rect.left_center() + egui::vec2(12.0, 0.0), egui::Align2::LEFT_CENTER, &label, egui::FontId::proportional(13.0), if item_resp.hovered() { theme::text_primary() } else { theme::text_secondary() });
                            if item_resp.clicked() {
                                open_album_idx = Some(*i);
                            }
                        }
                    });
                });
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) { close = true; }
            });

        if !close {
            let click_outside = ctx.input(|i| {
                i.pointer.any_pressed() && i.pointer.latest_pos().map(|pos| {
                    pos.x < popup_pos.x || pos.x > popup_pos.x + popup_width || pos.y < popup_pos.y || pos.y > popup_pos.y + popup_height
                }).unwrap_or(false)
            });
            if click_outside { close = true; }
        }

        if let Some(idx) = open_album_idx {
            self.handle_action(Action::OpenAlbum(idx));
            close = true;
        }

        if close {
            self.show_browse_albums_popup = false;
            self.browse_popup_filter.clear();
        }
    }

    fn render_in_page_search(&mut self, ctx: &egui::Context) {
        let search_width = 300.0;
        let search_height = 52.0;
        let screen = ctx.screen_rect();
        let search_pos = egui::pos2(screen.center().x - search_width / 2.0, screen.top() + 8.0);
        let search_rect = egui::Rect::from_min_size(search_pos, egui::vec2(search_width, search_height));

        let mut close_search = false;
        egui::Area::new(egui::Id::new("in_page_search"))
            .order(egui::Order::Foreground)
            .fixed_pos(search_pos)
            .show(ctx, |ui| {
                let frame = theme::glass_frame().inner_margin(egui::Margin::same(8));
                frame.show(ui, |ui| {
                    ui.set_min_width(search_width - 16.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("/").size(14.0).monospace().color(theme::green()));
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.in_page_search_query)
                                .desired_width(search_width - 60.0)
                                .hint_text("Search...")
                                .font(egui::FontId::proportional(13.0))
                                .frame(false),
                        );
                        response.request_focus();
                    });
                });

                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    close_search = true;
                }
                if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    close_search = true;
                    if !self.in_page_search_query.is_empty() {
                        let query = std::mem::take(&mut self.in_page_search_query);
                        self.send_request(ClientRequest::Search(query.clone()));
                        self.search_query = query;
                        self.navigate_to_view(View::Search);
                    } else {
                        self.in_page_search_query.clear();
                    }
                }
            });

        if close_search {
            self.show_in_page_search = false;
            self.in_page_search_query.clear();
        } else {
            let click_outside = ctx.input(|i| {
                i.pointer.any_pressed()
                    && i.pointer
                        .latest_pos()
                        .map(|pos| !search_rect.contains(pos))
                        .unwrap_or(false)
            });
            if click_outside {
                self.show_in_page_search = false;
                self.in_page_search_query.clear();
            }
        }
    }

    fn toast(&mut self, message: String) {
        self.toast_messages.push(ToastMessage::new(message, false));
        // Keep only last 10 messages
        if self.toast_messages.len() > 10 {
            self.toast_messages.remove(0);
        }
    }

    #[allow(dead_code)]
    fn toast_error(&mut self, message: String) {
        self.toast_messages.push(ToastMessage::new(message, true));
        if self.toast_messages.len() > 10 {
            self.toast_messages.remove(0);
        }
    }

    fn render_toast(&mut self, ctx: &egui::Context) {
        let now = std::time::Instant::now();
        
        // Remove expired toasts
        self.toast_messages.retain(|t| t.expires > now);
        
        if self.toast_messages.is_empty() {
            self.toast_show_all = false;
            return;
        }

        let toast_width = 320.0;
        let toast_height = 44.0;
        let screen = ctx.screen_rect();
        
        // Determine how many toasts to show
        let max_visible = if self.toast_show_all { self.toast_messages.len() } else { self.toast_messages.len().min(3) };
        let total_height = max_visible as f32 * (toast_height + 8.0);
        
        let start_y = screen.bottom() - 160.0 - total_height + toast_height;
        
        // Track which toasts to dismiss
        let mut dismiss_indices: Vec<usize> = Vec::new();
        
        // Accessibility: Track announcements for screen readers
        let mut announcements: Vec<String> = Vec::new();
        
        for i in 0..max_visible {
            if i >= self.toast_messages.len() {
                break;
            }
            let toast = &self.toast_messages[i];
            let y_pos = start_y + i as f32 * (toast_height + 8.0);
            let toast_pos = egui::pos2(
                screen.center().x - toast_width / 2.0,
                y_pos,
            );

            // Collect announcements for screen readers (aria-live region)
            if !toast.is_error {
                announcements.push(toast.message.clone());
            } else {
                announcements.push(format!("Error: {}", toast.message));
            }

            let mut dismissed = false;
            egui::Area::new(egui::Id::new(format!("toast_{}", i)))
                .order(egui::Order::Foreground)
                .fixed_pos(toast_pos)
                .interactable(true)
                .show(ctx, |ui| {
                    let frame = egui::Frame::new()
                        .fill(theme::with_alpha(theme::bg_dark(), 240))
                        .stroke(egui::Stroke::new(1.0, 
                            if toast.is_error { 
                                theme::with_alpha(theme::error_color(), 80)
                            } else {
                                theme::with_alpha(theme::accent(), 60)
                            }))
                        .corner_radius(egui::CornerRadius::same(theme::RADIUS_MEDIUM))
                        .inner_margin(egui::Margin::symmetric(12, 8));

                    frame.show(ui, |ui| {
                        ui.set_min_width(toast_width - 24.0);
                        ui.horizontal(|ui| {
                            // Icon with aria-label for screen readers
                            let icon = if toast.is_error { "⚠️" } else { "✓" };
                            let icon_label = if toast.is_error { "Error" } else { "Success" };
                            let icon_color = if toast.is_error { theme::error_color() } else { theme::green() };
                            ui.label(
                                egui::RichText::new(icon)
                                    .size(14.0)
                                    .color(icon_color),
                            )
                            .on_hover_text(icon_label);
                            ui.add_space(6.0);
                            
                            // Message with accessibility description
                            ui.label(
                                egui::RichText::new(&toast.message)
                                    .size(13.0)
                                    .color(theme::text_primary()),
                            );
                            
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                // Dismiss button with aria-label
                                if ui.add(
                                    egui::Button::new(
                                        egui::RichText::new("✕")
                                            .size(12.0)
                                            .color(theme::text_dim())
                                    )
                                    .fill(egui::Color32::TRANSPARENT)
                                    .frame(false)
                                )
                                .on_hover_text("Dismiss notification")
                                .clicked() {
                                    dismissed = true;
                                }
                            });
                        });
                    });
                });
            
            if dismissed {
                dismiss_indices.push(i);
            }
        }
        
        // Accessibility: Render live region for screen readers
        // This creates an invisible area that announces toasts to screen readers
        if !announcements.is_empty() {
            egui::Area::new(egui::Id::new("toast_live_region"))
                .order(egui::Order::Foreground)
                .fixed_pos(egui::pos2(-1000.0, -1000.0)) // Off-screen
                .interactable(false)
                .show(ctx, |ui| {
                    for announcement in announcements {
                        ui.label(
                            egui::RichText::new(announcement)
                                .size(1.0) // Tiny but readable by screen readers
                                .color(egui::Color32::TRANSPARENT),
                        );
                    }
                });
        }
        
        // Remove dismissed toasts (in reverse order to maintain indices)
        for &idx in dismiss_indices.iter().rev() {
            if idx < self.toast_messages.len() {
                self.toast_messages.remove(idx);
            }
        }
        
        // Show "Show All" button if there are more than 3 toasts
        if self.toast_messages.len() > 3 && !self.toast_show_all {
            let show_all_pos = egui::pos2(
                screen.center().x + toast_width / 2.0 + 8.0,
                screen.bottom() - 160.0 - total_height + toast_height / 2.0,
            );
            
            egui::Area::new(egui::Id::new("toast_show_all"))
                .order(egui::Order::Foreground)
                .fixed_pos(show_all_pos)
                .show(ctx, |ui| {
                    if ui.add(
                        egui::Button::new(
                            egui::RichText::new(format!("+{}", self.toast_messages.len() - 3))
                                .size(11.0)
                                .color(theme::text_secondary())
                        )
                        .fill(theme::with_alpha(theme::bg_dark(), 200))
                    ).clicked() {
                        self.toast_show_all = true;
                    }
                });
        }
    }


    fn render_key_hint(&mut self, ctx: &egui::Context) {
        if !self.key_seq_state.is_pending() {
            return;
        }
        let display = self.key_seq_state.pending_display();
        let hint_width = 120.0;
        let screen = ctx.screen_rect();
        let hint_pos = egui::pos2(
            screen.center().x - hint_width / 2.0,
            screen.bottom() - theme::PLAYBACK_BAR_HEIGHT - 40.0,
        );

        egui::Area::new(egui::Id::new("key_hint"))
            .order(egui::Order::Foreground)
            .fixed_pos(hint_pos)
            .interactable(false)
            .show(ctx, |ui| {
                let frame = egui::Frame::new()
                    .fill(theme::with_alpha(theme::bg_dark(), 220))
                    .stroke(egui::Stroke::new(1.0, theme::with_alpha(theme::accent(), 40)))
                    .corner_radius(egui::CornerRadius::same(theme::RADIUS_MEDIUM))
                    .inner_margin(egui::Margin::symmetric(12, 6));

                frame.show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(format!("- {} -", display))
                            .size(14.0)
                            .monospace()
                            .color(theme::green()),
                    );
                });
            });
    }

    fn save_settings(&mut self) {
        let needs_restart = self.settings_editing.client_id != self.settings_original.client_id
            || self.settings_editing.default_device != self.settings_original.default_device
            || self.settings_editing.device.name != self.settings_original.device.name
            || self.settings_editing.device.bitrate != self.settings_original.device.bitrate
            || self.settings_editing.theme != self.settings_original.theme;

        if let Err(err) = self.settings_editing.layout.check_values() {
            self.toast(format!("Invalid layout: {err}"));
            return;
        }

        match crate::config::get_config_folder_path() {
            Ok(config_folder) => {
                let file_path = config_folder.join("app.toml");
                let existing: toml::Value = std::fs::read_to_string(&file_path)
                    .ok()
                    .and_then(|s| toml::from_str(&s).ok())
                    .unwrap_or(toml::Value::Table(Default::default()));

                match toml::Value::try_from(&self.settings_editing) {
                    Ok(new_value) => {
                        let mut merged = existing;
                        if let (Some(t), Some(n)) = (merged.as_table_mut(), new_value.as_table()) {
                            for (key, value) in n {
                                t.insert(key.clone(), value.clone());
                            }
                        }
                        match toml::to_string_pretty(&merged) {
                            Ok(content) => {
                                let tmp_path = file_path.with_extension("toml.tmp");
                                let write_result = std::fs::write(&tmp_path, &content)
                                    .and_then(|()| std::fs::rename(&tmp_path, &file_path));
                                match write_result {
                                    Ok(()) => {
                                        if let Err(err) = crate::config::reload_config() {
                                            tracing::error!("Failed to reload config: {err:#}");
                                        }
                                        // C5: reload keybindings after settings save
                                        let mut bindings = default_keybindings();
                                        crate::config::get_config().keymap_config.apply_overrides(&mut bindings);
                                        self.keybindings = bindings;
                                        self.settings_dirty = false;
                                        self.settings_original = self.settings_editing.clone();
                                        if needs_restart {
                                            self.toast("Settings saved. Restart the app to apply changes.".to_string());
                                        } else {
                                            self.toast("Settings saved".to_string());
                                        }
                                    }
                                    Err(e) => {
                                        self.toast(format!("Failed to save: {}", e));
                                    }
                                }
                            }
                            Err(e) => {
                                self.toast(format!("Failed to serialize config: {}", e));
                            }
                        }
                    }
                    Err(e) => {
                        self.toast(format!("Failed to serialize config: {}", e));
                    }
                }
            }
            Err(e) => {
                self.toast(format!("Config folder not found: {}", e));
            }
        }
    }

    fn render_onboarding(&mut self, ctx: &egui::Context) {
        let popup_width = 480.0;
        let popup_height = 520.0;
        let screen = ctx.screen_rect();
        let popup_pos = egui::pos2(
            screen.center().x - popup_width / 2.0,
            screen.center().y - popup_height / 2.0,
        );

        let mut close = false;
        let mut start_auth = false;

        // Overlay background
        egui::Area::new(egui::Id::new("onboarding_overlay"))
            .order(egui::Order::Foreground)
            .fixed_pos(screen.min)
            .interactable(false)
            .show(ctx, |ui| {
                let (overlay_rect, _) = ui.allocate_exact_size(screen.size(), egui::Sense::hover());
                ui.painter().rect_filled(
                    overlay_rect,
                    0,
                    theme::with_alpha(theme::bg_black(), 180),
                );
            });

        egui::Area::new(egui::Id::new("onboarding_modal"))
            .order(egui::Order::Foreground)
            .fixed_pos(popup_pos)
            .show(ctx, |ui| {
                let frame = theme::glass_frame()
                    .inner_margin(egui::Margin::same(32));

                frame.show(ui, |ui| {
                    ui.set_min_width(popup_width - 64.0);

                    // Welcome header
                    ui.horizontal_centered(|ui| {
                        ui.add_space((popup_width - 64.0) / 2.0 - 30.0);
                        ui.label(
                            egui::RichText::new("🎵")
                                .size(48.0)
                        );
                    });
                    ui.add_space(8.0);
                    
                    ui.label(
                        egui::RichText::new("Welcome to Spotify Rust!")
                            .size(22.0)
                            .strong()
                            .color(theme::text_primary()),
                    );
                    ui.add_space(8.0);
                    
                    ui.label(
                        egui::RichText::new("A native Spotify client with a beautiful GUI")
                            .size(13.0)
                            .color(theme::text_secondary()),
                    );
                    ui.add_space(24.0);

                    // Step 1
                    ui.label(
                        egui::RichText::new("1. Authentication")
                            .size(15.0)
                            .strong()
                            .color(theme::text_primary()),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("To use this app, you need to authenticate with Spotify. You'll need a Spotify Client ID from the Spotify Developer Dashboard.")
                            .size(12.0)
                            .color(theme::text_secondary()),
                    );
                    ui.add_space(12.0);

                    // Step 2
                    ui.label(
                        egui::RichText::new("2. Keyboard Shortcuts")
                            .size(15.0)
                            .strong()
                            .color(theme::text_primary()),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("• Space: Play/Pause\n• N: Next track\n• P: Previous track\n• S: Search\n• L: Library\n• ?: Help\n• : Command palette")
                            .size(12.0)
                            .color(theme::text_secondary()),
                    );
                    ui.add_space(12.0);

                    // Step 3
                    ui.label(
                        egui::RichText::new("3. Getting Started")
                            .size(15.0)
                            .strong()
                            .color(theme::text_primary()),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("Click 'Get Started' to set up your Spotify authentication, or 'Skip' to browse in limited mode.")
                            .size(12.0)
                            .color(theme::text_secondary()),
                    );
                    ui.add_space(32.0);

                    // Buttons
                    ui.horizontal(|ui| {
                        // Skip button
                        let (skip_rect, skip_resp) = ui
                            .allocate_exact_size(egui::vec2(100.0, 40.0), egui::Sense::click());
                        let skip_bg = if skip_resp.hovered() {
                            theme::bg_hover()
                        } else {
                            theme::bg_card()
                        };
                        ui.painter().rect_filled(
                            skip_rect,
                            egui::CornerRadius::same(theme::RADIUS_MEDIUM),
                            skip_bg,
                        );
                        ui.painter().rect_stroke(
                            skip_rect,
                            egui::CornerRadius::same(theme::RADIUS_MEDIUM),
                            egui::Stroke::new(1.0, theme::text_muted()),
                            egui::StrokeKind::Outside,
                        );
                        ui.painter().text(
                            skip_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "Skip",
                            egui::FontId::proportional(14.0),
                            theme::text_primary(),
                        );
                        if skip_resp.clicked() {
                            close = true;
                        }

                        ui.add_space(16.0);

                        // Get Started button
                        let (start_rect, start_resp) = ui
                            .allocate_exact_size(egui::vec2(140.0, 40.0), egui::Sense::click());
                        let start_bg = if start_resp.hovered() {
                            theme::green_hover()
                        } else {
                            theme::green()
                        };
                        ui.painter().rect_filled(
                            start_rect,
                            egui::CornerRadius::same(theme::RADIUS_MEDIUM),
                            start_bg,
                        );
                        ui.painter().text(
                            start_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "Get Started",
                            egui::FontId::proportional(14.0),
                            theme::bg_black(),
                        );
                        if start_resp.clicked() {
                            start_auth = true;
                            close = true;
                        }
                    });
                });

                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    close = true;
                }
            });

        if close {
            self.show_onboarding = false;
            self.onboarding_completed = true;
        }
        
        if start_auth {
            self.show_auth_modal = true;
        }
    }

    fn render_auth_modal(&mut self, ctx: &egui::Context) {
        let popup_width = 420.0;
        let popup_height = 400.0;
        let screen = ctx.screen_rect();
        let popup_pos = egui::pos2(
            screen.center().x - popup_width / 2.0,
            screen.center().y - popup_height / 2.0,
        );

        let mut close = false;

        // Overlay background
        egui::Area::new(egui::Id::new("auth_overlay"))
            .order(egui::Order::Foreground)
            .fixed_pos(screen.min)
            .interactable(false)
            .show(ctx, |ui| {
                let (overlay_rect, _) = ui.allocate_exact_size(screen.size(), egui::Sense::hover());
                ui.painter().rect_filled(
                    overlay_rect,
                    0,
                    theme::with_alpha(theme::bg_black(), 180),
                );
            });

        egui::Area::new(egui::Id::new("auth_modal"))
            .order(egui::Order::Foreground)
            .fixed_pos(popup_pos)
            .show(ctx, |ui| {
                let frame = theme::glass_frame()
                    .inner_margin(egui::Margin::same(24));

                frame.show(ui, |ui| {
                    ui.set_min_width(popup_width - 48.0);

                    // Header
                    ui.label(
                        egui::RichText::new("Spotify Authentication")
                            .size(18.0)
                            .strong()
                            .color(theme::text_primary()),
                    );
                    ui.add_space(16.0);

                    // Step indicator
                    let step_text = match self.auth_step {
                        AuthStep::Idle | AuthStep::Step1 => "Step 1 of 2: Get Client ID",
                        AuthStep::Step2 => "Step 2 of 2: Authenticate",
                        AuthStep::Complete => "Authentication Complete",
                    };
                    ui.label(
                        egui::RichText::new(step_text)
                            .size(12.0)
                            .color(theme::green()),
                    );
                    ui.add_space(16.0);

                    match self.auth_step {
                        AuthStep::Idle | AuthStep::Step1 => {
                            ui.label(
                                egui::RichText::new("To use this app, you need a Spotify Client ID:")
                                    .size(13.0)
                                    .color(theme::text_secondary()),
                            );
                            ui.add_space(8.0);
                            
                            ui.label(
                                egui::RichText::new("1. Go to developer.spotify.com\n2. Create an app\n3. Add http://localhost:8888/callback as a redirect URI\n4. Copy your Client ID below")
                                    .size(12.0)
                                    .color(theme::text_dim()),
                            );
                            ui.add_space(16.0);

                            // Client ID input
                            ui.label(
                                egui::RichText::new("Client ID")
                                    .size(12.0)
                                    .color(theme::text_dim()),
                            );
                            ui.add_space(4.0);
                            let input = egui::TextEdit::singleline(&mut self.auth_client_id_input)
                                .desired_width(f32::INFINITY)
                                .hint_text("Paste your Client ID here...")
                                .font(egui::FontId::proportional(13.0))
                                .margin(egui::Margin::symmetric(10, 8))
                                .background_color(theme::bg_input());
                            ui.add(input);
                            ui.add_space(20.0);

                            // Buttons
                            ui.horizontal(|ui| {
                                // Cancel button
                                let (cancel_rect, cancel_resp) = ui
                                    .allocate_exact_size(egui::vec2(90.0, 36.0), egui::Sense::click());
                                let cancel_bg = if cancel_resp.hovered() {
                                    theme::bg_hover()
                                } else {
                                    theme::bg_card()
                                };
                                ui.painter().rect_filled(
                                    cancel_rect,
                                    egui::CornerRadius::same(theme::RADIUS_MEDIUM),
                                    cancel_bg,
                                );
                                ui.painter().rect_stroke(
                                    cancel_rect,
                                    egui::CornerRadius::same(theme::RADIUS_MEDIUM),
                                    egui::Stroke::new(1.0, theme::text_muted()),
                                    egui::StrokeKind::Outside,
                                );
                                ui.painter().text(
                                    cancel_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "Cancel",
                                    egui::FontId::proportional(13.0),
                                    theme::text_primary(),
                                );
                                if cancel_resp.clicked() {
                                    close = true;
                                }

                                ui.add_space(12.0);

                                // Open Browser button
                                let has_client_id = !self.auth_client_id_input.trim().is_empty();
                                let (browser_rect, browser_resp) = ui
                                    .allocate_exact_size(egui::vec2(140.0, 36.0), egui::Sense::click());
                                let browser_bg = if !has_client_id {
                                    theme::bg_dark()
                                } else if browser_resp.hovered() {
                                    theme::green_hover()
                                } else {
                                    theme::green()
                                };
                                ui.painter().rect_filled(
                                    browser_rect,
                                    egui::CornerRadius::same(theme::RADIUS_MEDIUM),
                                    browser_bg,
                                );
                                let browser_text_color = if has_client_id {
                                    theme::bg_black()
                                } else {
                                    theme::text_muted()
                                };
                                ui.painter().text(
                                    browser_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "Open Browser",
                                    egui::FontId::proportional(13.0),
                                    browser_text_color,
                                );
                                if browser_resp.clicked() && has_client_id {
                                    // Save client ID to config and open browser
                                    self.auth_step = AuthStep::Step2;
                                    self.toast("Opening browser for authentication...".to_string());
                                    // TODO: Trigger actual OAuth flow
                                }
                            });
                        }
                        AuthStep::Step2 => {
                            ui.label(
                                egui::RichText::new("Complete authentication in your browser...")
                                    .size(14.0)
                                    .color(theme::text_primary()),
                            );
                            ui.add_space(16.0);
                            
                            ui.horizontal_centered(|ui| {
                                ui.add_space((popup_width - 48.0) / 2.0 - 20.0);
                                ui.spinner();
                            });
                            ui.add_space(8.0);
                            
                            ui.label(
                                egui::RichText::new("Waiting for authentication...")
                                    .size(12.0)
                                    .color(theme::text_dim()),
                            );
                            ui.add_space(24.0);

                            // Cancel button
                            let (cancel_rect, cancel_resp) = ui
                                .allocate_exact_size(egui::vec2(100.0, 36.0), egui::Sense::click());
                            let cancel_bg = if cancel_resp.hovered() {
                                theme::bg_hover()
                            } else {
                                theme::bg_card()
                            };
                            ui.painter().rect_filled(
                                cancel_rect,
                                egui::CornerRadius::same(theme::RADIUS_MEDIUM),
                                cancel_bg,
                            );
                            ui.painter().rect_stroke(
                                cancel_rect,
                                egui::CornerRadius::same(theme::RADIUS_MEDIUM),
                                egui::Stroke::new(1.0, theme::text_muted()),
                                egui::StrokeKind::Outside,
                            );
                            ui.painter().text(
                                cancel_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "Cancel",
                                egui::FontId::proportional(13.0),
                                theme::text_primary(),
                            );
                            if cancel_resp.clicked() {
                                self.auth_step = AuthStep::Step1;
                            }
                        }
                        AuthStep::Complete => {
                            ui.label(
                                egui::RichText::new("✓ Authentication successful!")
                                    .size(16.0)
                                    .color(theme::green()),
                            );
                            ui.add_space(16.0);
                            
                            ui.label(
                                egui::RichText::new("You're all set. Enjoy your music!")
                                    .size(13.0)
                                    .color(theme::text_secondary()),
                            );
                            ui.add_space(24.0);

                            // Close button
                            let (close_rect, close_resp) = ui
                                .allocate_exact_size(egui::vec2(120.0, 36.0), egui::Sense::click());
                            let close_bg = if close_resp.hovered() {
                                theme::green_hover()
                            } else {
                                theme::green()
                            };
                            ui.painter().rect_filled(
                                close_rect,
                                egui::CornerRadius::same(theme::RADIUS_MEDIUM),
                                close_bg,
                            );
                            ui.painter().text(
                                close_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "Start Listening",
                                egui::FontId::proportional(13.0),
                                theme::bg_black(),
                            );
                            if close_resp.clicked() {
                                close = true;
                            }
                        }
                    }
                });

                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    close = true;
                }
            });

        if close {
            self.show_auth_modal = false;
            self.auth_step = AuthStep::Idle;
            self.auth_client_id_input.clear();
        }
    }
}
