use eframe::egui;
use rspotify::prelude::Id;

use crate::client::{ClientRequest, PlayerRequest};
use crate::gui::theme;
use crate::state::{self, Album, Artist, Episode, Item, ItemId, PlayableId, Playback, Playlist, SharedState, Show, Track};

#[derive(Clone, Debug)]
pub enum Navigation {
    GoToArtist(Artist),
    GoToAlbum(Album),
    GoToShow(Show),
    OpenAddToPlaylist(PlayableId<'static>),
}

#[derive(Clone, Debug)]
pub enum ContextTarget {
    Track {
        track: Track,
        index: usize,
        playlist_id: Option<state::PlaylistId<'static>>,
    },
    Album(Album),
    Artist(Artist),
    Playlist(Playlist),
    Show(Show),
    Episode {
        episode: Episode,
        show: Option<Show>,
    },
}

#[derive(Clone, Debug)]
pub struct MenuItem {
    pub icon: &'static str,
    pub label: &'static str,
    pub destructive: bool,
    pub action: MenuAction,
}

#[derive(Clone, Debug)]
pub enum MenuAction {
    AddToQueue(PlayableId<'static>),
    AddAlbumToQueue(state::AlbumId<'static>),
    AddToPlaylist(PlayableId<'static>),
    ToggleLiked(Track),
    AddAlbumToLibrary(Album),
    RemoveAlbumFromLibrary(state::AlbumId<'static>),
    AddPlaylistToLibrary(Playlist),
    PlayContext(Playback),
    GoToArtist(Artist),
    GoToAlbum(Album),
    GoToShow(Show),
    FollowArtist(Artist),
    UnfollowArtist(state::ArtistId<'static>),
    AddShowToLibrary(Show),
    RemoveShowFromLibrary(state::ShowId<'static>),
    CopyLink(String),
    DeleteFromPlaylist(state::PlaylistId<'static>, state::TrackId<'static>),
    None,
}

pub struct ContextMenu {
    pub target: Option<ContextTarget>,
    pub position: egui::Pos2,
    pub confirm_action: Option<MenuAction>,
}

impl ContextMenu {
    pub fn new() -> Self {
        Self {
            target: None,
            position: egui::Pos2::ZERO,
            confirm_action: None,
        }
    }

    pub fn is_open(&self) -> bool {
        self.target.is_some()
    }

    pub fn open(&mut self, target: ContextTarget, position: egui::Pos2) {
        self.target = Some(target);
        self.position = position;
        self.confirm_action = None;
    }

    pub fn close(&mut self) {
        self.target = None;
        self.confirm_action = None;
    }

    fn track_items(track: &Track, playlist_id: &Option<state::PlaylistId<'static>>) -> Vec<MenuItem> {
        let mut items = Vec::new();

        items.push(MenuItem {
            icon: "\u{2795}",
            label: "Add to Queue",
            destructive: false,
            action: MenuAction::AddToQueue(PlayableId::Track(track.id.clone())),
        });

        items.push(MenuItem {
            icon: "\u{1F4CB}",
            label: "Add to Playlist",
            destructive: false,
            action: MenuAction::AddToPlaylist(PlayableId::Track(track.id.clone())),
        });

        items.push(MenuItem {
            icon: "\u{2764}",
            label: "Like / Unlike",
            destructive: false,
            action: MenuAction::ToggleLiked(track.clone()),
        });

        if let Some(ref album) = track.album {
            items.push(MenuItem {
                icon: "\u{1F4BF}",
                label: "Go to Album",
                destructive: false,
                action: MenuAction::GoToAlbum(album.clone()),
            });
        }

        if let Some(artist) = track.artists.first() {
            items.push(MenuItem {
                icon: "\u{1F3A4}",
                label: "Go to Artist",
                destructive: false,
                action: MenuAction::GoToArtist(artist.clone()),
            });
        }

        let uri = format!("https://open.spotify.com/track/{}", track.id.id());
        items.push(MenuItem {
            icon: "\u{1F517}",
            label: "Copy Link",
            destructive: false,
            action: MenuAction::CopyLink(uri),
        });

        if let Some(playlist_id) = playlist_id {
            items.push(MenuItem {
                icon: "\u{1F5D1}",
                label: "Remove from Playlist",
                destructive: true,
                action: MenuAction::DeleteFromPlaylist(
                    playlist_id.clone(),
                    track.id.clone(),
                ),
            });
        }

        items
    }

    fn album_items(album: &Album) -> Vec<MenuItem> {
        let mut items = Vec::new();

        items.push(MenuItem {
            icon: "\u{25B6}",
            label: "Play",
            destructive: false,
            action: MenuAction::PlayContext(Playback::Context(
                state::ContextId::Album(album.id.clone()),
                None,
            )),
        });

        items.push(MenuItem {
            icon: "\u{2795}",
            label: "Add to Library",
            destructive: false,
            action: MenuAction::AddAlbumToLibrary(album.clone()),
        });

        items.push(MenuItem {
            icon: "\u{1F5D1}",
            label: "Remove from Library",
            destructive: true,
            action: MenuAction::RemoveAlbumFromLibrary(album.id.clone()),
        });

        if let Some(artist) = album.artists.first() {
            items.push(MenuItem {
                icon: "\u{1F3A4}",
                label: "Go to Artist",
                destructive: false,
                action: MenuAction::GoToArtist(artist.clone()),
            });
        }

        let uri = format!("https://open.spotify.com/album/{}", album.id.id());
        items.push(MenuItem {
            icon: "\u{1F517}",
            label: "Copy Link",
            destructive: false,
            action: MenuAction::CopyLink(uri),
        });

        items
    }

    fn artist_items(artist: &Artist) -> Vec<MenuItem> {
        let mut items = Vec::new();

        items.push(MenuItem {
            icon: "\u{2764}",
            label: "Follow / Unfollow",
            destructive: false,
            action: MenuAction::FollowArtist(artist.clone()),
        });

        let uri = format!("https://open.spotify.com/artist/{}", artist.id.id());
        items.push(MenuItem {
            icon: "\u{1F517}",
            label: "Copy Link",
            destructive: false,
            action: MenuAction::CopyLink(uri),
        });

        items
    }

    fn playlist_items(playlist: &Playlist) -> Vec<MenuItem> {
        let mut items = Vec::new();

        items.push(MenuItem {
            icon: "\u{25B6}",
            label: "Play",
            destructive: false,
            action: MenuAction::PlayContext(Playback::Context(
                state::ContextId::Playlist(playlist.id.clone()),
                None,
            )),
        });

        items.push(MenuItem {
            icon: "\u{2795}",
            label: "Add to Library",
            destructive: false,
            action: MenuAction::AddPlaylistToLibrary(playlist.clone()),
        });

        let uri = format!("https://open.spotify.com/playlist/{}", playlist.id.id());
        items.push(MenuItem {
            icon: "\u{1F517}",
            label: "Copy Link",
            destructive: false,
            action: MenuAction::CopyLink(uri),
        });

        items
    }

    fn show_items(show: &Show) -> Vec<MenuItem> {
        let mut items = Vec::new();

        items.push(MenuItem {
            icon: "\u{25B6}",
            label: "Play",
            destructive: false,
            action: MenuAction::PlayContext(Playback::Context(
                state::ContextId::Show(show.id.clone()),
                None,
            )),
        });

        items.push(MenuItem {
            icon: "\u{2795}",
            label: "Add to Library",
            destructive: false,
            action: MenuAction::AddShowToLibrary(show.clone()),
        });

        items.push(MenuItem {
            icon: "\u{1F50D}",
            label: "Go to Show",
            destructive: false,
            action: MenuAction::GoToShow(show.clone()),
        });

        items.push(MenuItem {
            icon: "\u{1F5D1}",
            label: "Unfollow",
            destructive: true,
            action: MenuAction::RemoveShowFromLibrary(show.id.clone()),
        });

        let uri = format!("https://open.spotify.com/show/{}", show.id.id());
        items.push(MenuItem {
            icon: "\u{1F517}",
            label: "Copy Link",
            destructive: false,
            action: MenuAction::CopyLink(uri),
        });

        items
    }

    fn episode_items(episode: &Episode, show: &Option<Show>) -> Vec<MenuItem> {
        let mut items = Vec::new();

        if let Some(show) = show {
            items.push(MenuItem {
                icon: "\u{25B6}",
                label: "Play",
                destructive: false,
                action: MenuAction::PlayContext(Playback::Context(
                    state::ContextId::Show(show.id.clone()),
                    Some(rspotify::model::Offset::Uri(episode.id.uri())),
                )),
            });
        }

        items.push(MenuItem {
            icon: "\u{2795}",
            label: "Add to Queue",
            destructive: false,
            action: MenuAction::AddToQueue(PlayableId::Episode(episode.id.clone())),
        });

        if let Some(show) = show {
            items.push(MenuItem {
                icon: "\u{1F399}",
                label: "Go to Show",
                destructive: false,
                action: MenuAction::GoToShow(show.clone()),
            });
        }

        let uri = format!("https://open.spotify.com/episode/{}", episode.id.id());
        items.push(MenuItem {
            icon: "\u{1F517}",
            label: "Copy Link",
            destructive: false,
            action: MenuAction::CopyLink(uri),
        });

        items
    }

    pub fn render(
        &mut self,
        ctx: &egui::Context,
        state: &SharedState,
        client_pub: &flume::Sender<ClientRequest>,
    ) -> Option<Navigation> {
        let target = match &self.target {
            Some(t) => t.clone(),
            None => return None,
        };

        let items = match &target {
            ContextTarget::Track { track, playlist_id, .. } => {
                Self::track_items(track, playlist_id)
            }
            ContextTarget::Album(album) => Self::album_items(album),
            ContextTarget::Artist(artist) => Self::artist_items(artist),
            ContextTarget::Playlist(playlist) => Self::playlist_items(playlist),
            ContextTarget::Show(show) => Self::show_items(show),
            ContextTarget::Episode { episode, show } => Self::episode_items(episode, show),
        };

        let menu_width = 220.0;
        let item_height = 36.0;
        let padding = 6.0;
        let menu_height = items.len() as f32 * item_height + padding * 2.0;

        let screen = ctx.screen_rect();
        let mut pos = self.position;

        if pos.x + menu_width > screen.right() {
            pos.x = screen.right() - menu_width - 8.0;
        }
        if pos.y + menu_height > screen.bottom() {
            pos.y = screen.bottom() - menu_height - 8.0;
        }
        if pos.x < 0.0 {
            pos.x = 8.0;
        }
        if pos.y < 0.0 {
            pos.y = 8.0;
        }

        let mut action_to_execute: Option<MenuAction> = None;
        let mut should_close = false;

        // Overlay background
        egui::Area::new(egui::Id::new("context_menu_overlay"))
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

        egui::Area::new(egui::Id::new("context_menu"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .interactable(true)
            .show(ctx, |ui| {
                let frame = theme::glass_frame();

                frame.show(ui, |ui| {
                    ui.set_min_width(menu_width - 12.0);

                    for item in &items {
                        let item_rect = ui
                            .allocate_exact_size(
                                egui::vec2(ui.available_width(), item_height),
                                egui::Sense::click(),
                            )
                            .0;

                        let response = ui.allocate_rect(item_rect, egui::Sense::click());

                        let bg = if response.hovered() {
                            theme::bg_hover()
                        } else {
                            egui::Color32::TRANSPARENT
                        };
                        ui.painter()
                            .rect_filled(item_rect, egui::CornerRadius::same(4), bg);

                        let text_color = if item.destructive {
                            theme::error_color()
                        } else if response.hovered() {
                            theme::text_primary()
                        } else {
                            theme::text_secondary()
                        };

                        ui.painter().text(
                            item_rect.left_center() + egui::vec2(12.0, 0.0),
                            egui::Align2::LEFT_CENTER,
                            item.icon,
                            egui::FontId::proportional(14.0),
                            text_color,
                        );

                        ui.painter().text(
                            item_rect.left_center() + egui::vec2(36.0, 0.0),
                            egui::Align2::LEFT_CENTER,
                            item.label,
                            egui::FontId::proportional(13.0),
                            text_color,
                        );

                        if response.clicked() {
                            if item.destructive {
                                self.confirm_action = Some(item.action.clone());
                            } else {
                                action_to_execute = Some(item.action.clone());
                                should_close = true;
                            }
                        }
                    }
                });

                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    should_close = true;
                }
            });

        let mut click_outside = false;
        ctx.input(|i| {
            if i.pointer.any_pressed() {
                if let Some(click_pos) = i.pointer.latest_pos() {
                    let menu_rect = egui::Rect::from_min_size(
                        pos,
                        egui::vec2(menu_width, menu_height),
                    );
                    if !menu_rect.contains(click_pos) {
                        click_outside = true;
                    }
                }
            }
        });

        if click_outside && self.confirm_action.is_none() {
            self.close();
            return None;
        }

        if should_close {
            self.close();
        }

        if let Some(action) = action_to_execute {
            let nav = self.execute_action(action, state, client_pub);
            return nav;
        }

        if let Some(confirm) = self.confirm_action.clone() {
            self.render_confirm_dialog(ctx, confirm, state, client_pub);
        }

        None
    }

    fn render_confirm_dialog(
        &mut self,
        ctx: &egui::Context,
        action: MenuAction,
        state: &SharedState,
        client_pub: &flume::Sender<ClientRequest>,
    ) {
        let (title, detail) = match &action {
            MenuAction::RemoveAlbumFromLibrary(_) => (
                "Remove from Library?",
                "This album will be removed from your library.",
            ),
            MenuAction::UnfollowArtist(_) => (
                "Unfollow Artist?",
                "You will no longer follow this artist.",
            ),
            MenuAction::RemoveShowFromLibrary(_) => (
                "Unfollow Show?",
                "This show will be removed from your library.",
            ),
            MenuAction::DeleteFromPlaylist(_, _) => (
                "Remove from Playlist?",
                "This track will be removed from the playlist.",
            ),
            _ => ("Confirm?", "Are you sure?"),
        };

        let dialog_width = 300.0;
        let dialog_height = 140.0;
        let screen = ctx.screen_rect();
        let dialog_pos = egui::pos2(
            screen.center().x - dialog_width / 2.0,
            screen.center().y - dialog_height / 2.0,
        );

        let mut close = false;
        let mut execute = false;

        // Overlay background
        egui::Area::new(egui::Id::new("context_confirm_overlay"))
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

        egui::Area::new(egui::Id::new("context_confirm"))
            .order(egui::Order::Foreground)
            .fixed_pos(dialog_pos)
            .show(ctx, |ui| {
                let frame = theme::glass_frame()
                    .inner_margin(egui::Margin::same(16));

                frame.show(ui, |ui| {
                    ui.set_min_width(dialog_width - 32.0);

                    ui.label(
                        egui::RichText::new(title)
                            .size(15.0)
                            .strong()
                            .color(theme::text_primary()),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(detail)
                            .size(12.0)
                            .color(theme::text_secondary()),
                    );
                    ui.add_space(16.0);

                    ui.horizontal(|ui| {
                        let cancel_rect = ui
                            .allocate_exact_size(egui::vec2(80.0, 32.0), egui::Sense::click())
                            .0;
                        let cancel_resp = ui.allocate_rect(cancel_rect, egui::Sense::click());
                        let cancel_bg = if cancel_resp.hovered() {
                            theme::bg_hover()
                        } else {
                            theme::bg_dark()
                        };
                        ui.painter().rect_filled(
                            cancel_rect,
                            egui::CornerRadius::same(4),
                            cancel_bg,
                        );
                        ui.painter().rect_stroke(
                            cancel_rect,
                            egui::CornerRadius::same(4),
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

                        let confirm_rect = ui
                            .allocate_exact_size(egui::vec2(100.0, 32.0), egui::Sense::click())
                            .0;
                        let confirm_resp = ui.allocate_rect(confirm_rect, egui::Sense::click());
                        let confirm_bg = if confirm_resp.hovered() {
                            theme::error_color()
                        } else {
                            theme::error_color()
                        };
                        ui.painter().rect_filled(
                            confirm_rect,
                            egui::CornerRadius::same(4),
                            confirm_bg,
                        );
                         ui.painter().text(
                            confirm_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "Remove",
                            egui::FontId::proportional(13.0),
                            theme::text_primary(),
                        );
                        if confirm_resp.clicked() {
                            execute = true;
                            close = true;
                        }
                    });
                });

                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    close = true;
                }
            });

        if close {
            self.confirm_action = None;
            self.target = None;
        }

        if execute {
            self.execute_action(action, state, client_pub);
        }
    }

    fn execute_action(
        &self,
        action: MenuAction,
        state: &SharedState,
        client_pub: &flume::Sender<ClientRequest>,
    ) -> Option<Navigation> {
        match action {
            MenuAction::AddToQueue(playable_id) => {
                let _ = client_pub.send(ClientRequest::AddPlayableToQueue(playable_id));
                None
            }
            MenuAction::AddAlbumToQueue(album_id) => {
                let _ = client_pub.send(ClientRequest::AddAlbumToQueue(album_id));
                None
            }
            MenuAction::AddToPlaylist(playable_id) => {
                Some(Navigation::OpenAddToPlaylist(playable_id))
            }
            MenuAction::ToggleLiked(track) => {
                let data = state.data.read();
                let is_liked = data.user_data.saved_tracks.contains_key(&track.id.uri());
                drop(data);

                if is_liked {
                    let _ = client_pub.send(ClientRequest::DeleteFromLibrary(ItemId::Track(
                        track.id.clone(),
                    )));
                } else {
                    let _ =
                        client_pub.send(ClientRequest::AddToLibrary(Item::Track(track)));
                }
                None
            }
            MenuAction::AddAlbumToLibrary(album) => {
                let _ = client_pub.send(ClientRequest::AddToLibrary(Item::Album(album)));
                None
            }
            MenuAction::RemoveAlbumFromLibrary(album_id) => {
                let _ = client_pub.send(ClientRequest::DeleteFromLibrary(ItemId::Album(
                    album_id,
                )));
                None
            }
            MenuAction::AddPlaylistToLibrary(playlist) => {
                let _ =
                    client_pub.send(ClientRequest::AddToLibrary(Item::Playlist(playlist)));
                None
            }
            MenuAction::PlayContext(playback) => {
                let _ = client_pub.send(ClientRequest::Player(PlayerRequest::StartPlayback(
                    playback, None,
                )));
                None
            }
            MenuAction::GoToArtist(artist) => {
                Some(Navigation::GoToArtist(artist))
            }
            MenuAction::GoToAlbum(album) => {
                Some(Navigation::GoToAlbum(album))
            }
            MenuAction::GoToShow(show) => {
                Some(Navigation::GoToShow(show))
            }
            MenuAction::FollowArtist(artist) => {
                let data = state.data.read();
                let is_followed = data
                    .user_data
                    .followed_artists
                    .iter()
                    .any(|a| a.id == artist.id);
                drop(data);

                if is_followed {
                    let _ = client_pub.send(ClientRequest::DeleteFromLibrary(
                        ItemId::Artist(artist.id),
                    ));
                } else {
                    let _ = client_pub.send(ClientRequest::AddToLibrary(Item::Artist(artist)));
                }
                None
            }
            MenuAction::UnfollowArtist(artist_id) => {
                let _ = client_pub.send(ClientRequest::DeleteFromLibrary(ItemId::Artist(
                    artist_id,
                )));
                None
            }
            MenuAction::AddShowToLibrary(show) => {
                let _ = client_pub.send(ClientRequest::AddToLibrary(Item::Show(show)));
                None
            }
            MenuAction::RemoveShowFromLibrary(show_id) => {
                let _ = client_pub.send(ClientRequest::DeleteFromLibrary(ItemId::Show(show_id)));
                None
            }
            MenuAction::CopyLink(link) => {
                #[cfg(target_os = "macos")]
                {
                    use std::process::Command;
                    let _ = Command::new("pbcopy")
                        .stdin(std::process::Stdio::piped())
                        .spawn()
                        .and_then(|mut child| {
                            if let Some(ref mut stdin) = child.stdin {
                                use std::io::Write;
                                let _ = stdin.write_all(link.as_bytes());
                            }
                            child.wait()
                        });
                }
                #[cfg(target_os = "linux")]
                {
                    use std::process::Command;
                    let _ = Command::new("xclip")
                        .args(["-selection", "clipboard"])
                        .stdin(std::process::Stdio::piped())
                        .spawn()
                        .and_then(|mut child| {
                            if let Some(ref mut stdin) = child.stdin {
                                use std::io::Write;
                                let _ = stdin.write_all(link.as_bytes());
                            }
                            child.wait()
                        });
                }
                #[cfg(target_os = "windows")]
                {
                    use std::process::Command;
                    let _ = Command::new("clip")
                        .stdin(std::process::Stdio::piped())
                        .spawn()
                        .and_then(|mut child| {
                            if let Some(ref mut stdin) = child.stdin {
                                use std::io::Write;
                                let _ = stdin.write_all(link.as_bytes());
                            }
                            child.wait()
                        });
                }
                None
            }
            MenuAction::DeleteFromPlaylist(playlist_id, track_id) => {
                let _ = client_pub.send(ClientRequest::DeleteTrackFromPlaylist(
                    playlist_id, track_id,
                ));
                None
            }
            MenuAction::None => None,
        }
    }
}
