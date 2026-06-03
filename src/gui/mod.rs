mod image_cache;
mod playback_bar;
mod sidebar;
mod theme;
mod views;

use eframe::egui;

use crate::client::ClientRequest;
use crate::state::{self, SharedState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum View {
    Library,
    Tracks,
    Search,
    Queue,
    Settings,
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
}

impl SpotifyApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        state: SharedState,
        client_pub: flume::Sender<ClientRequest>,
    ) -> Self {
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
        }
    }

    fn handle_action(&mut self, action: Action) {
        match action {
            Action::Navigate(view) => {
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
                    let _ = self
                        .client_pub
                        .send(ClientRequest::GetContext(state::ContextId::Album(id)));
                    self.current_view = View::Tracks;
                }
            }
            Action::OpenLikedTracks => {
                self.context_title = "Liked Tracks".to_string();
                self.context_tracks.clear();
                self.selected_track = None;
                let _ = self.client_pub.send(ClientRequest::GetContext(
                    state::ContextId::Tracks(state::TracksId::new(
                        state::USER_LIKED_TRACKS_URI,
                        "Liked Tracks",
                    )),
                ));
                self.current_view = View::Tracks;
            }
            Action::OpenRecentlyPlayed => {
                self.context_title = "Recently Played".to_string();
                self.context_tracks.clear();
                self.selected_track = None;
                let _ = self.client_pub.send(ClientRequest::GetContext(
                    state::ContextId::Tracks(state::TracksId::new(
                        state::USER_RECENTLY_PLAYED_TRACKS_URI,
                        "Recently Played",
                    )),
                ));
                self.current_view = View::Tracks;
            }
            Action::OpenTopTracks => {
                self.context_title = "Top Tracks".to_string();
                self.context_tracks.clear();
                self.selected_track = None;
                let _ = self.client_pub.send(ClientRequest::GetContext(
                    state::ContextId::Tracks(state::TracksId::new(
                        state::USER_TOP_TRACKS_URI,
                        "Top Tracks",
                    )),
                ));
                self.current_view = View::Tracks;
            }
            Action::OpenSearchResultPlaylist(playlist) => {
                self.context_title = playlist.name.clone();
                self.context_tracks.clear();
                self.selected_track = None;
                let _ = self.client_pub.send(ClientRequest::GetContext(
                    state::ContextId::Playlist(playlist.id),
                ));
                self.current_view = View::Tracks;
            }
            Action::OpenSearchResultAlbum(album) => {
                self.context_title = album.name.clone();
                self.context_tracks.clear();
                self.selected_track = None;
                let _ = self
                    .client_pub
                    .send(ClientRequest::GetContext(state::ContextId::Album(album.id)));
                self.current_view = View::Tracks;
            }
            Action::None => {}
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

        // Bottom panel — playback bar
        egui::TopBottomPanel::bottom("playback_bar")
            .resizable(false)
            .exact_height(theme::PLAYBACK_BAR_HEIGHT)
            .show(ctx, |ui| {
                playback_bar::render(ui, &self.state, &self.client_pub, &mut self.image_cache);
            });

        // Left panel — sidebar
        let mut action = Action::None;
        egui::SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(theme::SIDEBAR_WIDTH)
            .frame(egui::Frame::new().fill(theme::BG_DARK).inner_margin(egui::Margin::ZERO))
            .show(ctx, |ui| {
                action = sidebar::render(ui, &self.current_view, &self.state);
            });
        self.handle_action(action);

        // Central panel — main content
        let mut action = Action::None;
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(theme::BG_BLACK)
                    .inner_margin(egui::Margin::ZERO),
            )
            .show(ctx, |ui| match self.current_view {
                View::Library => {
                    action = views::render_library(ui, &self.state, &mut self.image_cache);
                }
                View::Tracks => {
                    views::render_tracks(
                        ui,
                        &self.state,
                        &self.client_pub,
                        &self.context_title,
                        &self.context_tracks,
                        &mut self.selected_track,
                        &mut self.image_cache,
                    );
                }
                View::Search => {
                    action = views::render_search(
                        ui,
                        &self.state,
                        &self.client_pub,
                        &mut self.search_query,
                        &mut self.selected_track,
                        &mut self.image_cache,
                    );
                }
                View::Queue => {
                    views::render_queue(ui, &self.state, &self.client_pub, &mut self.image_cache);
                }
                View::Settings => {
                    views::render_settings(ui);
                }
            });
        self.handle_action(action);
    }
}
