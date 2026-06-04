mod command_palette;
mod context_menu;
mod image_cache;
mod playback_bar;
mod sidebar;
mod theme;
mod views;

use eframe::egui;
use rspotify::prelude::Id;

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
            SortColumn::Title => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            SortColumn::Artist => a.artists_info().to_lowercase().cmp(&b.artists_info().to_lowercase()),
            SortColumn::Album => a.album_info().to_lowercase().cmp(&b.album_info().to_lowercase()),
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
    OpenShows,
    OpenShowDetail(state::Show),
    OpenShowFromSearch(state::Show),
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
    show_add_to_playlist_popup: bool,
    add_to_playlist_track: Option<state::PlayableId<'static>>,
    add_to_playlist_filter: String,
    toast_message: Option<String>,
    toast_expires: Option<std::time::Instant>,
    current_context_id: Option<state::ContextId>,
    key_seq_state: KeySequenceState,
    keybindings: Vec<CommandBinding>,
    help_search: String,
    view_history: Vec<View>,
    show_theme_switcher: bool,
    theme_search: String,
    current_theme_name: String,
    show_command_palette: bool,
    command_palette: command_palette::CommandPalette,
    settings_tab: views::SettingsTab,
    show_detail_show: Option<state::Show>,
    show_detail_episodes: Vec<state::Episode>,
    show_detail_context_id: Option<state::ContextId>,
    show_detail_selected_episode: Option<usize>,
    settings_editing: crate::config::AppConfig,
    settings_original: crate::config::AppConfig,
    settings_dirty: bool,
    settings_keybinding_search: String,
    settings_editing_keybinding: Option<usize>,
    library_sort_order: LibrarySortOrder,
    scroll_to_selected: bool,
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
        let mut current_theme_name = String::new();
        if let Some(t) = config.theme_config.find_theme(theme_name) {
            theme::set_palette_from_config(&t.palette);
            current_theme_name = t.name.clone();
        } else {
            theme::set_palette(theme_name);
            current_theme_name = theme_name.clone();
        }
        theme::setup_theme(&cc.egui_ctx);

        Self {
            state,
            client_pub,
            current_view: View::Library,
            search_query: String::new(),
            selected_track: None,
            context_tracks: Vec::new(),
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
            show_add_to_playlist_popup: false,
            add_to_playlist_track: None,
            add_to_playlist_filter: String::new(),
            toast_message: None,
            toast_expires: None,
            current_context_id: None,
            key_seq_state: KeySequenceState::new(),
            keybindings: default_keybindings(),
            help_search: String::new(),
            view_history: Vec::new(),
            show_theme_switcher: false,
            theme_search: String::new(),
            current_theme_name,
            show_command_palette: false,
            command_palette: command_palette::CommandPalette::new(),
            settings_tab: views::SettingsTab::General,
            show_detail_show: None,
            show_detail_episodes: Vec::new(),
            show_detail_context_id: None,
            show_detail_selected_episode: None,
            settings_editing: crate::config::get_config().app_config.clone(),
            settings_original: crate::config::get_config().app_config.clone(),
            settings_dirty: false,
            settings_keybinding_search: String::new(),
            settings_editing_keybinding: None,
            library_sort_order: LibrarySortOrder::Default,
            scroll_to_selected: false,
        }
    }

    fn handle_action(&mut self, action: Action) {
        match action {
            Action::Navigate(view) => {
                if view != self.current_view && view != View::Help {
                    self.view_history.push(self.current_view.clone());
                }
                self.current_view = view;
            }
            Action::OpenPlaylist(idx) => {
                let data = self.state.data.read();
                if let Some(item) = data.user_data.playlists.get(idx) {
                    if let state::PlaylistFolderItem::Playlist(playlist) = item {
                        let id = playlist.id.clone();
                        let name = playlist.name.clone();
                        drop(data);
                        self.context_title = name;
                        self.selected_track = None;
                        self.context_tracks.clear();
                        self.sort_state = None;
                    self.current_context_id = Some(state::ContextId::Playlist(id.clone()));
                    let _ = self.client_pub.send(ClientRequest::GetContext(
                        state::ContextId::Playlist(id),
                    ));
                    self.current_view = View::Tracks;
                    }
                }
            }
            Action::OpenAlbum(idx) => {
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
                self.context_title = "Liked Tracks".to_string();
                self.context_tracks.clear();
                self.sort_state = None;
                self.selected_track = None;
                let ctx_id = state::ContextId::Tracks(state::TracksId::new(
                    state::USER_LIKED_TRACKS_URI,
                    "Liked Tracks",
                ));
                self.current_context_id = Some(ctx_id.clone());
                let _ = self.client_pub.send(ClientRequest::GetContext(ctx_id));
                self.current_view = View::Tracks;
            }
            Action::OpenRecentlyPlayed => {
                self.context_title = "Recently Played".to_string();
                self.context_tracks.clear();
                self.sort_state = None;
                self.selected_track = None;
                let ctx_id = state::ContextId::Tracks(state::TracksId::new(
                    state::USER_RECENTLY_PLAYED_TRACKS_URI,
                    "Recently Played",
                ));
                self.current_context_id = Some(ctx_id.clone());
                let _ = self.client_pub.send(ClientRequest::GetContext(ctx_id));
                self.current_view = View::Tracks;
            }
            Action::OpenTopTracks => {
                self.context_title = "Top Tracks".to_string();
                self.context_tracks.clear();
                self.sort_state = None;
                self.selected_track = None;
                let ctx_id = state::ContextId::Tracks(state::TracksId::new(
                    state::USER_TOP_TRACKS_URI,
                    "Top Tracks",
                ));
                self.current_context_id = Some(ctx_id.clone());
                let _ = self.client_pub.send(ClientRequest::GetContext(ctx_id));
                self.current_view = View::Tracks;
            }
            Action::OpenSearchResultPlaylist(playlist) => {
                self.context_title = playlist.name.clone();
                self.context_tracks.clear();
                self.sort_state = None;
                self.selected_track = None;
                self.current_context_id = Some(state::ContextId::Playlist(playlist.id.clone()));
                let _ = self.client_pub.send(ClientRequest::GetContext(
                    state::ContextId::Playlist(playlist.id),
                ));
                self.current_view = View::Tracks;
            }
            Action::OpenSearchResultAlbum(album) => {
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
                self.artist_id = Some(artist.id.uri());
                self.artist_context = None;
                let _ = self
                    .client_pub
                    .send(ClientRequest::GetContext(state::ContextId::Artist(artist.id)));
                self.current_view = View::Artist;
            }
            Action::OpenBrowseCategory(id, name) => {
                let _ = self.client_pub.send(ClientRequest::GetBrowseCategoryPlaylists(
                    state::Category {
                        id: id.clone(),
                        name: name.clone(),
                        icon_url: None,
                    },
                ));
                self.current_view = View::BrowseCategory { id, name };
            }
            Action::OpenBrowsePlaylist(playlist) => {
                self.context_title = playlist.name.clone();
                self.context_tracks.clear();
                self.sort_state = None;
                self.selected_track = None;
                self.current_context_id = Some(state::ContextId::Playlist(playlist.id.clone()));
                let _ = self.client_pub.send(ClientRequest::GetContext(
                    state::ContextId::Playlist(playlist.id),
                ));
                self.current_view = View::Tracks;
            }
            Action::BackToBrowse => {
                self.current_view = View::Browse;
            }
            Action::ContextMenuNavigateArtist(artist) => {
                self.artist_id = Some(artist.id.uri());
                self.artist_context = None;
                let _ = self
                    .client_pub
                    .send(ClientRequest::GetContext(state::ContextId::Artist(artist.id)));
                self.current_view = View::Artist;
            }
            Action::ContextMenuNavigateAlbum(album) => {
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
                let ctx_id = state::ContextId::Show(show.id.clone());
                self.show_detail_context_id = Some(ctx_id.clone());
                self.show_detail_show = Some(show);
                self.show_detail_episodes.clear();
                self.show_detail_selected_episode = None;
                let _ = self.client_pub.send(ClientRequest::GetContext(ctx_id));
                self.current_view = View::ShowDetail;
            }
            Action::OpenCreatePlaylist => {
                self.show_create_playlist_popup = true;
                self.create_playlist_name.clear();
                self.create_playlist_desc.clear();
                self.create_playlist_public = true;
                self.create_playlist_collab = false;
            }
            Action::OpenShows => {
                let data = self.state.data.read();
                let shows_empty = data.user_data.saved_shows.is_empty();
                let shows_loading = data.shows_loading;
                drop(data);
                if shows_empty && !shows_loading {
                    self.state.data.write().shows_loading = true;
                    let _ = self.client_pub.send(ClientRequest::GetUserSavedShows);
                }
                self.navigate_to_view(View::Shows);
            }
            Action::OpenShowDetail(show) => {
                let ctx_id = state::ContextId::Show(show.id.clone());
                self.show_detail_context_id = Some(ctx_id.clone());
                self.show_detail_show = Some(show);
                self.show_detail_episodes.clear();
                self.show_detail_selected_episode = None;
                let _ = self.client_pub.send(ClientRequest::GetContext(ctx_id));
                self.current_view = View::ShowDetail;
            }
            Action::OpenShowFromSearch(show) => {
                let ctx_id = state::ContextId::Show(show.id.clone());
                self.show_detail_context_id = Some(ctx_id.clone());
                self.show_detail_show = Some(show);
                self.show_detail_episodes.clear();
                self.show_detail_selected_episode = None;
                let _ = self.client_pub.send(ClientRequest::GetContext(ctx_id));
                self.current_view = View::ShowDetail;
            }
            Action::None => {}
        }
    }

    fn navigate_to_view(&mut self, view: View) {
        if view != self.current_view {
            self.view_history.push(self.current_view.clone());
        }
        self.current_view = view;
    }

    fn go_back(&mut self) {
        if let Some(prev) = self.view_history.pop() {
            self.current_view = prev;
        }
    }

    fn execute_command(&mut self, cmd: &Command, count: usize) {
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
                    let page_size = 20;
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
                    let page_size = 20;
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
                NavCommand::Enter => {
                    if self.current_view == View::ShowDetail {
                        if let Some(idx) = self.show_detail_selected_episode {
                            if idx < self.show_detail_episodes.len() {
                                let episode = &self.show_detail_episodes[idx];
                                if let Some(ref ctx_id) = self.show_detail_context_id {
                                    let playback = state::Playback::Context(
                                        ctx_id.clone(),
                                        Some(rspotify::model::Offset::Uri(episode.id.uri())),
                                    );
                                    let _ = self.client_pub.send(ClientRequest::Player(
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
                    let _ = self.client_pub.send(ClientRequest::Player(PlayerRequest::ResumePause));
                }
                PlaybackCommand::NextTrack => {
                    let _ = self.client_pub.send(ClientRequest::Player(PlayerRequest::NextTrack));
                }
                PlaybackCommand::PrevTrack => {
                    let _ = self.client_pub.send(ClientRequest::Player(PlayerRequest::PreviousTrack));
                }
                PlaybackCommand::RefreshPlayback => {
                    let _ = self.client_pub.send(ClientRequest::GetCurrentPlayback);
                    self.toast("Refreshing playback...".to_string());
                }
                PlaybackCommand::RestartClient => {
                    #[cfg(feature = "streaming")]
                    {
                        let _ = self.client_pub.send(ClientRequest::RestartIntegratedClient);
                        self.toast("Restarting client...".to_string());
                    }
                }
                PlaybackCommand::MuteToggle => {
                    let _ = self.client_pub.send(ClientRequest::Player(PlayerRequest::ToggleMute));
                }
                PlaybackCommand::SeekToStart => {
                    let _ = self.client_pub.send(ClientRequest::Player(PlayerRequest::SeekTrack(
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
                    let _ = self.client_pub.send(ClientRequest::Player(PlayerRequest::SeekTrack(new_pos)));
                }
                PlaybackCommand::SeekBackward => {
                    let seek_secs = crate::config::get_config().app_config.seek_duration_secs as i64;
                    let player = self.state.player.read();
                    let current_pos = player.playback_progress()
                        .unwrap_or(chrono::Duration::zero());
                    drop(player);
                    let new_pos = (current_pos - chrono::Duration::seconds(seek_secs * count as i64))
                        .max(chrono::Duration::zero());
                    let _ = self.client_pub.send(ClientRequest::Player(PlayerRequest::SeekTrack(new_pos)));
                }
                PlaybackCommand::PlayRandom => {
                    if !self.context_tracks.is_empty() {
                        let idx = rand::random_range(0..self.context_tracks.len());
                        let track = self.context_tracks[idx].clone();
                        self.play_track_from_context(&track);
                    }
                }
                PlaybackCommand::Shuffle => {
                    let _ = self.client_pub.send(ClientRequest::Player(PlayerRequest::Shuffle));
                }
                PlaybackCommand::Repeat => {
                    let _ = self.client_pub.send(ClientRequest::Player(PlayerRequest::Repeat));
                }
                PlaybackCommand::VolumeUp => {
                    let vol = self.state.player.read()
                        .current_playback()
                        .and_then(|p| p.device.volume_percent)
                        .unwrap_or(50) as u8;
                    let new_vol = vol.saturating_add(5).min(100);
                    let _ = self.client_pub.send(ClientRequest::Player(PlayerRequest::Volume(new_vol)));
                }
                PlaybackCommand::VolumeDown => {
                    let vol = self.state.player.read()
                        .current_playback()
                        .and_then(|p| p.device.volume_percent)
                        .unwrap_or(50) as u8;
                    let new_vol = vol.saturating_sub(5);
                    let _ = self.client_pub.send(ClientRequest::Player(PlayerRequest::Volume(new_vol)));
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
                        self.context_tracks.sort_by(|a, b| a.added_at.cmp(&b.added_at));
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
                    self.toast("Logs page not yet implemented".to_string());
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
                        if let Some(ref item) = playback.item {
                            if let rspotify::model::PlayableItem::Track(t) = item {
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
                }
                ActionCommand::ShowActionsOnContext => {
                    self.toast("Context actions".to_string());
                }
                ActionCommand::AddToQueue => {
                    if let Some(idx) = self.selected_track {
                        if idx < self.context_tracks.len() {
                            let track = &self.context_tracks[idx];
                            let _ = self.client_pub.send(ClientRequest::AddPlayableToQueue(
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
        // Try to read clipboard and open a spotify link
        // On macOS we can use pbpaste
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            if let Ok(output) = Command::new("pbpaste").output() {
                if let Ok(text) = String::from_utf8(output.stdout) {
                    let text = text.trim();
                    if text.contains("open.spotify.com") {
                        self.toast(format!("Opening: {}", text));
                        // TODO: Parse and navigate to the link
                    } else {
                        self.toast("No Spotify link in clipboard".to_string());
                    }
                }
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
        ctx.request_repaint_after(std::time::Duration::from_millis(100));

        if self.current_view == View::Tracks && self.context_tracks.is_empty() {
            self.update_context_tracks();
        }

        // Update show detail episodes when viewing show detail page
        if self.current_view == View::ShowDetail && self.show_detail_episodes.is_empty() {
            if let Some(ref ctx_id) = self.show_detail_context_id {
                let data = self.state.data.read();
                if let Some(ctx) = data.caches.context.get(&ctx_id.uri()) {
                    if let state::Context::Show { show, episodes } = ctx {
                        self.show_detail_show = Some(show.clone());
                        self.show_detail_episodes = episodes.clone();
                    }
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
                let bar_response = playback_bar::render(ui, &self.state, &self.client_pub, &mut self.image_cache);
                if let Some(view) = bar_response.navigate {
                    self.current_view = view;
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
                let _ = self.client_pub.send(ClientRequest::GetDevices);
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
                                    .rect_filled(item_rect, egui::CornerRadius::same(6), bg);

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
                                        &device.device_type,
                                        egui::FontId::proportional(11.0),
                                        theme::text_dim(),
                                    );
                                }

                                if item_response.clicked() && !is_active {
                                    let _ = self.client_pub.send(ClientRequest::Player(
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
                let popup_rect = ctx.screen_rect();
                let click_outside = ctx.input(|i| {
                    i.pointer.any_pressed()
                        && i.pointer
                            .latest_pos()
                            .map(|pos| {
                                // Check if click is outside popup area (approximate)
                                pos.y < popup_rect.bottom() - theme::PLAYBACK_BAR_HEIGHT - 420.0
                                    || pos.y > popup_rect.bottom() - theme::PLAYBACK_BAR_HEIGHT
                                    || pos.x < popup_x - popup_width
                                    || pos.x > popup_x
                            })
                            .unwrap_or(false)
                });
                if click_outside {
                    self.show_device_popup = false;
                }
            }
        }

        // Left panel — sidebar
        let mut action = Action::None;
        egui::SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(theme::SIDEBAR_WIDTH)
            .frame(egui::Frame::new().fill(theme::bg_dark()).inner_margin(egui::Margin::ZERO))
            .show(ctx, |ui| {
                action = sidebar::render(ui, &self.current_view, &self.state);
            });
        self.handle_action(action);

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
                } else if self.show_create_playlist_popup {
                    self.show_create_playlist_popup = false;
                } else if self.show_add_to_playlist_popup {
                    self.show_add_to_playlist_popup = false;
                } else if self.show_theme_switcher {
                    self.show_theme_switcher = false;
                } else if self.show_device_popup {
                    self.show_device_popup = false;
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
                            self.execute_command(&cmd, count);
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
                                let _ = self.client_pub.send(ClientRequest::GetLyrics {
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
                            if self.sort_state == Some(new_state) {
                                // Already sorted this way, do nothing
                            } else {
                                self.sort_state = Some(new_state);
                                self.context_tracks.sort_by(|a, b| new_state.compare(a, b));
                                self.selected_track = None;
                            }
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
                        &self.keybindings,
                        &self.current_theme_name,
                        &self.client_pub,
                    );
                    match settings_action {
                        views::SettingsAction::Save => self.save_settings(),
                        views::SettingsAction::Reset => {
                            self.settings_editing = crate::config::AppConfig::default();
                            self.settings_dirty = true;
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
                        &self.show_detail_show,
                        &self.show_detail_episodes,
                        &self.show_detail_context_id,
                        &mut self.show_detail_selected_episode,
                        &mut self.image_cache,
                        &mut self.context_menu,
                    );
                }
                View::Help => {
                    views::render_help(ui, &self.keybindings, &mut self.help_search);
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
                context_menu::Navigation::OpenAddToPlaylist(playable_id) => {
                    self.context_menu.close();
                    self.show_add_to_playlist_popup = true;
                    self.add_to_playlist_track = Some(playable_id);
                    self.add_to_playlist_filter.clear();
                }
            }
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
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::S)) {
            if self.current_view == View::Settings && self.settings_dirty {
                self.save_settings();
            }
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
                    self.execute_command(&cmd, count);
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
        let _ = self.client_pub.send(ClientRequest::Player(
            PlayerRequest::StartPlayback(playback, None),
        ));
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
                    egui::Color32::from_black_alpha(120),
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

                    // Name input
                    ui.label(
                        egui::RichText::new("Name")
                            .size(12.0)
                            .color(theme::text_dim()),
                    );
                    ui.add_space(4.0);
                    let name_input = egui::TextEdit::singleline(&mut self.create_playlist_name)
                        .desired_width(f32::INFINITY)
                        .hint_text("Playlist name")
                        .font(egui::FontId::proportional(13.0))
                        .margin(egui::Margin::symmetric(10, 8))
                        .background_color(theme::bg_input());
                    ui.add(name_input);
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
                        let cancel_rect = ui
                            .allocate_exact_size(egui::vec2(100.0, 36.0), egui::Sense::click())
                            .0;
                        let cancel_resp = ui.allocate_rect(cancel_rect, egui::Sense::click());
                        let cancel_bg = if cancel_resp.hovered() {
                            theme::bg_hover()
                        } else {
                            theme::bg_card()
                        };
                        ui.painter().rect_filled(
                            cancel_rect,
                            egui::CornerRadius::same(6),
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
                        let can_create = !self.create_playlist_name.trim().is_empty();
                        let create_rect = ui
                            .allocate_exact_size(egui::vec2(100.0, 36.0), egui::Sense::click())
                            .0;
                        let create_resp = ui.allocate_rect(create_rect, egui::Sense::click());
                        let create_bg = if !can_create {
                            theme::bg_dark()
                        } else if create_resp.hovered() {
                            theme::green_hover()
                        } else {
                            theme::green()
                        };
                        ui.painter().rect_filled(
                            create_rect,
                            egui::CornerRadius::same(6),
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
            let _ = self.client_pub.send(ClientRequest::CreatePlaylist {
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
                    egui::Color32::from_black_alpha(120),
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
                                let item_rect = ui
                                    .allocate_exact_size(
                                        egui::vec2(ui.available_width(), 44.0),
                                        egui::Sense::click(),
                                    )
                                    .0;
                                let item_resp =
                                    ui.allocate_rect(item_rect, egui::Sense::click());

                                let bg = if item_resp.hovered() {
                                    theme::bg_hover()
                                } else {
                                    egui::Color32::TRANSPARENT
                                };
                                ui.painter().rect_filled(
                                    item_rect,
                                    egui::CornerRadius::same(4),
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
                let _ = self.client_pub.send(ClientRequest::AddPlayableToPlaylist(
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
                    egui::Color32::from_black_alpha(120),
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
                                let item_rect = ui
                                    .allocate_exact_size(
                                        egui::vec2(ui.available_width(), 36.0),
                                        egui::Sense::click(),
                                    )
                                    .0;
                                let item_resp = ui.allocate_rect(item_rect, egui::Sense::click());

                                let bg = if is_current {
                                    theme::bg_active()
                                } else if item_resp.hovered() {
                                    theme::bg_hover()
                                } else {
                                    egui::Color32::TRANSPARENT
                                };
                                ui.painter().rect_filled(
                                    item_rect,
                                    egui::CornerRadius::same(4),
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
                                    let item_rect = ui
                                        .allocate_exact_size(
                                            egui::vec2(ui.available_width(), 36.0),
                                            egui::Sense::click(),
                                        )
                                        .0;
                                    let item_resp = ui.allocate_rect(item_rect, egui::Sense::click());

                                    let bg = if is_current {
                                        theme::bg_active()
                                    } else if item_resp.hovered() {
                                        theme::bg_hover()
                                    } else {
                                        egui::Color32::TRANSPARENT
                                    };
                                    ui.painter().rect_filled(
                                        item_rect,
                                        egui::CornerRadius::same(4),
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

    fn toast(&mut self, message: String) {
        self.toast_message = Some(message);
        self.toast_expires = Some(
            std::time::Instant::now() + std::time::Duration::from_secs(2),
        );
    }

    fn render_toast(&mut self, ctx: &egui::Context) {
        let now = std::time::Instant::now();
        if let Some(expires) = self.toast_expires {
            if now >= expires {
                self.toast_message = None;
                self.toast_expires = None;
                return;
            }
        }
        let message = match &self.toast_message {
            Some(m) => m.clone(),
            None => return,
        };

        let toast_width = 260.0;
        let screen = ctx.screen_rect();
        let toast_pos = egui::pos2(
            screen.center().x - toast_width / 2.0,
            screen.bottom() - 160.0,
        );

        egui::Area::new(egui::Id::new("toast"))
            .order(egui::Order::Foreground)
            .fixed_pos(toast_pos)
            .interactable(false)
            .show(ctx, |ui| {
                let frame = egui::Frame::new()
                    .fill(theme::with_alpha(theme::bg_dark(), 220))
                    .stroke(egui::Stroke::new(1.0, theme::with_alpha(theme::accent(), 60)))
                    .corner_radius(egui::CornerRadius::same(8))
                    .inner_margin(egui::Margin::symmetric(16, 10));

                frame.show(ui, |ui| {
                    ui.set_min_width(toast_width - 32.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("\u{2713}")
                                .size(14.0)
                                .color(theme::green()),
                        );
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new(message)
                                .size(13.0)
                                .color(theme::text_primary()),
                        );
                    });
                });
            });
    }

    fn render_key_hint(&self, ctx: &egui::Context) {
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
                    .corner_radius(egui::CornerRadius::same(6))
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
            || self.settings_editing.client_port != self.settings_original.client_port
            || self.settings_editing.default_device != self.settings_original.default_device
            || self.settings_editing.device.name != self.settings_original.device.name
            || self.settings_editing.device.bitrate != self.settings_original.device.bitrate;

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
                            Ok(content) => match std::fs::write(&file_path, content) {
                                Ok(()) => {
                                    self.settings_dirty = false;
                                    self.settings_original = self.settings_editing.clone();
                                    if needs_restart {
                                        self.toast("Settings saved. Some changes require restart.".to_string());
                                    } else {
                                        self.toast("Settings saved".to_string());
                                    }
                                }
                                Err(e) => {
                                    self.toast(format!("Failed to save: {}", e));
                                }
                            },
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
}
