mod context_menu;
mod image_cache;
mod playback_bar;
mod sidebar;
mod theme;
mod views;

use eframe::egui;
use rspotify::prelude::Id;

use crate::client::{ClientRequest, PlayerRequest};
use crate::state::{self, SharedState};

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
            artist_context: None,
            artist_id: None,
            show_device_popup: false,
            devices_fetched: false,
            context_menu: context_menu::ContextMenu::new(),
            sort_state: None,
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
                        self.sort_state = None;
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
                self.sort_state = None;
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
                self.sort_state = None;
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
                self.sort_state = None;
                self.selected_track = None;
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
                    let frame = egui::Frame::new()
                        .fill(egui::Color32::from_rgb(17, 17, 17))
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(26, 26, 26)))
                        .corner_radius(egui::CornerRadius::same(8))
                        .inner_margin(egui::Margin::same(8));

                    frame.show(ui, |ui| {
                        ui.set_min_width(popup_width - 16.0);
                        ui.set_max_height(popup_max_height);

                        // Header
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Connect to a device")
                                    .size(14.0)
                                    .strong()
                                    .color(theme::TEXT_PRIMARY),
                            );
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("✕")
                                                .size(14.0)
                                                .color(theme::TEXT_DIM),
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
                            .rect_filled(div_rect, 0.0, egui::Color32::from_rgb(26, 26, 26));

                        ui.add_space(8.0);

                        let player = self.state.player.read();
                        let devices = &player.devices;

                        if devices.is_empty() {
                            ui.label(
                                egui::RichText::new("No devices available")
                                    .size(12.0)
                                    .color(theme::TEXT_DIM),
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
                                    egui::Color32::from_rgb(26, 26, 26)
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
                                    egui::Color32::from_rgb(100, 100, 100),
                                );

                                // Device name
                                let name_color = if is_active {
                                    theme::GREEN
                                } else {
                                    theme::TEXT_PRIMARY
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
                                        theme::GREEN,
                                    );
                                    ui.painter().text(
                                        item_rect.left_center() + egui::vec2(54.0, 12.0),
                                        egui::Align2::LEFT_CENTER,
                                        format!("{} · Active", device.device_type),
                                        egui::FontId::proportional(11.0),
                                        theme::GREEN,
                                    );
                                } else {
                                    ui.painter().text(
                                        item_rect.left_center() + egui::vec2(44.0, 12.0),
                                        egui::Align2::LEFT_CENTER,
                                        &device.device_type,
                                        egui::FontId::proportional(11.0),
                                        egui::Color32::from_rgb(136, 136, 136),
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
            .frame(egui::Frame::new().fill(theme::BG_DARK).inner_margin(egui::Margin::ZERO))
            .show(ctx, |ui| {
                action = sidebar::render(ui, &self.current_view, &self.state);
            });
        self.handle_action(action);

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
                    .fill(theme::BG_BLACK)
                    .inner_margin(egui::Margin::ZERO),
            )
            .show(ctx, |ui| match self.current_view {
                View::Library => {
                    action = views::render_library(ui, &self.state, &mut self.image_cache, &mut self.context_menu);
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
                    );
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
                    views::render_settings(ui);
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
            }
        }
    }
}
