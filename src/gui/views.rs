use eframe::egui;
use rspotify::prelude::Id;

use crate::client::ClientRequest;
use crate::gui::image_cache::{self, ImageCache};
use crate::gui::{theme, Action};
use crate::state::{self, SharedState};

pub fn render_library(
    ui: &mut egui::Ui,
    state: &SharedState,
    image_cache: &mut ImageCache,
) -> Action {
    let mut action = Action::None;

    theme::page_title(ui, "Your Library");

    egui::ScrollArea::vertical()
        .id_salt("library_scroll")
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(24.0);
            });

            // Quick access cards
            ui.horizontal(|ui| {
                ui.add_space(24.0);

                // Liked Tracks card
                let card_width = (ui.available_width() - 24.0) / 3.0;
                quick_card(ui, "♥", "Liked Tracks", "Your favorite songs", card_width, || {
                    action = Action::OpenLikedTracks;
                });
                ui.add_space(12.0);
                quick_card(ui, "⏰", "Recently Played", "What you listened to", card_width, || {
                    action = Action::OpenRecentlyPlayed;
                });
                ui.add_space(12.0);
                quick_card(ui, "⭐", "Top Tracks", "Your most played", card_width, || {
                    action = Action::OpenTopTracks;
                });
            });

            ui.add_space(32.0);

            // Playlists section
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.label(
                    egui::RichText::new("Playlists")
                        .size(22.0)
                        .strong()
                        .color(theme::TEXT_PRIMARY),
                );
            });
            ui.add_space(12.0);

            let data = state.data.read();
            let playlists: Vec<_> = data
                .user_data
                .playlists
                .iter()
                .filter_map(|item| match item {
                    state::PlaylistFolderItem::Playlist(p) => Some(p.clone()),
                    _ => None,
                })
                .collect();
            drop(data);

            ui.horizontal(|ui| {
                ui.add_space(24.0);
            });

            egui::Grid::new("playlists_grid")
                .num_columns(4)
                .spacing([16.0, 16.0])
                .show(ui, |ui| {
                    for (i, playlist) in playlists.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.add_space(24.0);
                            let cover_path = image_cache::playlist_cover_path(playlist);
                            if let (Some(path), Some(url)) = (&cover_path, &playlist.cover_url) {
                                if !path.exists() {
                                    image_cache.request_download(url, path);
                                }
                            }
                            grid_card(
                                ui,
                                &playlist.name,
                                &playlist.owner.0,
                                cover_path.as_deref(),
                                image_cache,
                                || {
                                    action = Action::OpenSearchResultPlaylist(playlist.clone());
                                },
                            );
                        });
                        if (i + 1) % 4 == 0 {
                            ui.end_row();
                        }
                    }
                });

            ui.add_space(32.0);

            // Albums section
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.label(
                    egui::RichText::new("Albums")
                        .size(22.0)
                        .strong()
                        .color(theme::TEXT_PRIMARY),
                );
            });
            ui.add_space(12.0);

            let data = state.data.read();
            let albums: Vec<_> = data.user_data.saved_albums.clone();
            drop(data);

            egui::Grid::new("albums_grid")
                .num_columns(4)
                .spacing([16.0, 16.0])
                .show(ui, |ui| {
                    for (i, album) in albums.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.add_space(24.0);
                            let sub = format!(
                                "{} · {}",
                                album
                                    .artists
                                    .iter()
                                    .map(|a| a.name.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", "),
                                album.year()
                            );
                            let cover_path = image_cache::album_cover_path(album);
                            if let (Some(path), Some(url)) = (&cover_path, &album.cover_url) {
                                if !path.exists() {
                                    image_cache.request_download(url, path);
                                }
                            }
                            grid_card(ui, &album.name, &sub, cover_path.as_deref(), image_cache, || {
                                action = Action::OpenSearchResultAlbum(album.clone());
                            });
                        });
                        if (i + 1) % 4 == 0 {
                            ui.end_row();
                        }
                    }
                });

            ui.add_space(24.0);
        });

    action
}

fn quick_card(ui: &mut egui::Ui, icon: &str, title: &str, desc: &str, width: f32, on_click: impl FnOnce()) {
    let card_height = 80.0;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, card_height), egui::Sense::click());

    let bg = if response.hovered() {
        theme::BG_HOVER
    } else {
        theme::BG_CARD
    };

    ui.painter().rect_filled(rect, 8.0, bg);

    // Icon circle
    let icon_rect = egui::Rect::from_center_size(
        rect.left_center() + egui::vec2(36.0, 0.0),
        egui::vec2(48.0, 48.0),
    );
    ui.painter().rect_filled(icon_rect, 24.0, theme::GREEN_DARK);
    ui.painter().text(
        icon_rect.center(),
        egui::Align2::CENTER_CENTER,
        icon,
        egui::FontId::proportional(22.0),
        theme::TEXT_PRIMARY,
    );

    // Title
    ui.painter().text(
        rect.left_top() + egui::vec2(72.0, 24.0),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(14.0),
        theme::TEXT_PRIMARY,
    );

    // Description
    ui.painter().text(
        rect.left_top() + egui::vec2(72.0, 46.0),
        egui::Align2::LEFT_CENTER,
        desc,
        egui::FontId::proportional(11.0),
        theme::TEXT_DIM,
    );

    if response.clicked() {
        on_click();
    }
}

fn grid_card(
    ui: &mut egui::Ui,
    title: &str,
    subtitle: &str,
    cover_path: Option<&std::path::Path>,
    image_cache: &mut ImageCache,
    on_click: impl FnOnce(),
) {
    let width = 160.0;
    let height = 200.0;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());

    let bg = if response.hovered() {
        theme::BG_HOVER
    } else {
        theme::BG_CARD
    };

    ui.painter().rect_filled(rect, 8.0, bg);

    // Album art
    let art_size = width - 24.0;
    let art_rect = egui::Rect::from_min_size(
        rect.min + egui::vec2(12.0, 12.0),
        egui::vec2(art_size, art_size),
    );

    let mut art_drawn = false;
    if let Some(path) = cover_path {
        if let Some(texture) = image_cache.get_texture(ui.ctx(), path) {
            ui.painter().rect_filled(art_rect, theme::ART_CORNER_RADIUS, theme::BG_ACTIVE);
            egui::Image::new(texture)
                .corner_radius(theme::ART_CORNER_RADIUS)
                .paint_at(ui, art_rect);
            art_drawn = true;
        }
    }

    if !art_drawn {
        ui.painter().rect_filled(art_rect, theme::ART_CORNER_RADIUS, theme::BG_ACTIVE);
        if response.hovered() {
            let play_rect = egui::Rect::from_center_size(
                art_rect.center() + egui::vec2(0.0, 10.0),
                egui::vec2(40.0, 40.0),
            );
            ui.painter().rect_filled(play_rect, 20.0, theme::GREEN);
            ui.painter().text(
                play_rect.center(),
                egui::Align2::CENTER_CENTER,
                "\u{25B6}",
                egui::FontId::proportional(16.0),
                theme::BG_BLACK,
            );
        } else {
            ui.painter().text(
                art_rect.center(),
                egui::Align2::CENTER_CENTER,
                "\u{266B}",
                egui::FontId::proportional(28.0),
                theme::TEXT_MUTED,
            );
        }
    }

    // Play button overlay on hover (on top of cover art)
    if art_drawn && response.hovered() {
        let play_rect = egui::Rect::from_center_size(
            art_rect.center(),
            egui::vec2(40.0, 40.0),
        );
        // Semi-transparent dark overlay
        ui.painter().rect_filled(
            art_rect,
            theme::ART_CORNER_RADIUS,
            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 100),
        );
        ui.painter().rect_filled(play_rect, 20.0, theme::GREEN);
        ui.painter().text(
            play_rect.center(),
            egui::Align2::CENTER_CENTER,
            "\u{25B6}",
            egui::FontId::proportional(16.0),
            theme::BG_BLACK,
        );
    }

    // Title
    ui.painter().text(
        rect.left_top() + egui::vec2(12.0, art_size + 26.0),
        egui::Align2::LEFT_TOP,
        title,
        egui::FontId::proportional(13.0),
        theme::TEXT_PRIMARY,
    );

    // Subtitle
    ui.painter().text(
        rect.left_top() + egui::vec2(12.0, art_size + 46.0),
        egui::Align2::LEFT_TOP,
        subtitle,
        egui::FontId::proportional(11.0),
        theme::TEXT_DIM,
    );

    if response.clicked() {
        on_click();
    }
}

pub fn render_tracks(
    ui: &mut egui::Ui,
    state: &SharedState,
    _client_pub: &flume::Sender<ClientRequest>,
    title: &str,
    tracks: &[state::Track],
    selected_track: &mut Option<usize>,
    image_cache: &mut ImageCache,
) {
    use rspotify::prelude::Id;

    theme::page_title(ui, title);

    let player = state.player.read();
    let current_track_uri: Option<String> = player.playback.as_ref().and_then(|p| {
        p.item.as_ref().map(|item| match item {
            rspotify::model::PlayableItem::Track(t) => {
                t.id.as_ref().map(|id| id.uri()).unwrap_or_default()
            }
            rspotify::model::PlayableItem::Episode(e) => e.id.uri(),
            _ => String::new(),
        })
    });
    drop(player);

    // Table header
    ui.horizontal(|ui| {
        ui.add_space(24.0);
        let header_rect = ui.allocate_space(egui::vec2(ui.available_width() - 24.0, 32.0)).1;

        ui.painter().text(
            header_rect.left_center() + egui::vec2(0.0, 0.0),
            egui::Align2::LEFT_CENTER,
            "#",
            egui::FontId::monospace(12.0),
            theme::TEXT_MUTED,
        );
        ui.painter().text(
            header_rect.left_center() + egui::vec2(92.0, 0.0),
            egui::Align2::LEFT_CENTER,
            "TITLE",
            egui::FontId::monospace(12.0),
            theme::TEXT_MUTED,
        );
        ui.painter().text(
            header_rect.right_center() + egui::vec2(-50.0, 0.0),
            egui::Align2::LEFT_CENTER,
            "TIME",
            egui::FontId::monospace(12.0),
            theme::TEXT_MUTED,
        );
    });

    // Divider
    let div_rect = ui
        .allocate_space(egui::vec2(ui.available_width() - 24.0, 1.0))
        .1;
    ui.painter()
        .rect_filled(
            egui::Rect::from_min_size(
                div_rect.min + egui::vec2(24.0, 0.0),
                div_rect.size(),
            ),
            0.0,
            theme::DIVIDER,
        );

        egui::ScrollArea::vertical()
        .id_salt("tracks_scroll")
        .show(ui, |ui| {
            for (i, track) in tracks.iter().enumerate() {
                let is_playing = current_track_uri
                    .as_ref()
                    .map_or(false, |uri| uri == &track.id.uri());
                let is_selected = *selected_track == Some(i);

                let row_height = 48.0;
                let (row_rect, response) = ui
                    .allocate_exact_size(
                        egui::vec2(ui.available_width(), row_height),
                        egui::Sense::click(),
                    );

                let bg = if is_selected {
                    theme::BG_HOVER
                } else if response.hovered() {
                    theme::BG_CARD
                } else {
                    egui::Color32::TRANSPARENT
                };

                ui.painter().rect_filled(row_rect, 4.0, bg);

                // Number
                let num_color = if is_playing {
                    theme::GREEN
                } else {
                    theme::TEXT_MUTED
                };
                let num_str = if is_playing { "\u{25B6}".to_string() } else { format!("{}", i + 1) };
                ui.painter().text(
                    row_rect.left_center() + egui::vec2(28.0, 0.0),
                    egui::Align2::CENTER_CENTER,
                    &num_str,
                    egui::FontId::monospace(12.0),
                    num_color,
                );

                // Thumbnail
                let thumb_rect = egui::Rect::from_min_size(
                    row_rect.left_center() + egui::vec2(48.0, -theme::TRACK_THUMB_SIZE / 2.0),
                    egui::vec2(theme::TRACK_THUMB_SIZE, theme::TRACK_THUMB_SIZE),
                );
                let mut thumb_drawn = false;
                if let Some(ref album) = track.album {
                    if let Some(path) = image_cache::album_cover_path(album) {
                        if let Some(texture) = image_cache.get_texture(ui.ctx(), &path) {
                            ui.painter().rect_filled(
                                thumb_rect,
                                theme::ART_CORNER_RADIUS,
                                theme::BG_ACTIVE,
                            );
                            egui::Image::new(texture)
                                .corner_radius(theme::ART_CORNER_RADIUS)
                                .paint_at(ui, thumb_rect);
                            thumb_drawn = true;
                        }
                    }
                }
                if !thumb_drawn {
                    ui.painter().rect_filled(
                        thumb_rect,
                        theme::ART_CORNER_RADIUS,
                        theme::BG_ACTIVE,
                    );
                    ui.painter().text(
                        thumb_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "\u{266B}",
                        egui::FontId::proportional(14.0),
                        theme::TEXT_MUTED,
                    );
                }

                // Track info
                let title_color = if is_playing {
                    theme::GREEN
                } else {
                    theme::TEXT_PRIMARY
                };
                ui.painter().text(
                    row_rect.left_center() + egui::vec2(92.0, -7.0),
                    egui::Align2::LEFT_CENTER,
                    &track.name,
                    egui::FontId::proportional(14.0),
                    title_color,
                );
                ui.painter().text(
                    row_rect.left_center() + egui::vec2(92.0, 10.0),
                    egui::Align2::LEFT_CENTER,
                    track.artists_info(),
                    egui::FontId::proportional(12.0),
                    theme::TEXT_DIM,
                );

                // Album name (middle)
                let album_name = track.album_info();
                if !album_name.is_empty() {
                    ui.painter().text(
                        row_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        &album_name,
                        egui::FontId::proportional(12.0),
                        theme::TEXT_DIM,
                    );
                }

                // Duration
                let duration = track.duration;
                let dur_str = theme::format_duration_secs(duration.as_secs());
                ui.painter().text(
                    row_rect.right_center() + egui::vec2(-24.0, 0.0),
                    egui::Align2::RIGHT_CENTER,
                    &dur_str,
                    egui::FontId::monospace(12.0),
                    theme::TEXT_DIM,
                );

                // Play button on hover
                if response.hovered() && !is_playing {
                    let play_btn_rect = egui::Rect::from_center_size(
                        row_rect.left_center() + egui::vec2(28.0, 0.0),
                        egui::vec2(24.0, 24.0),
                    );
                    ui.painter()
                        .rect_filled(play_btn_rect, 12.0, theme::GREEN);
                    ui.painter().text(
                        play_btn_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "\u{25B6}",
                        egui::FontId::proportional(10.0),
                        theme::BG_BLACK,
                    );
                }

                if response.clicked() {
                    *selected_track = Some(i);
                }

                // Row divider
                let div = egui::Rect::from_min_size(
                    row_rect.left_bottom() + egui::vec2(24.0, 0.0),
                    egui::vec2(row_rect.width() - 48.0, 1.0),
                );
                ui.painter().rect_filled(div, 0.0, theme::DIVIDER);
            }
        });
}

pub fn render_search(
    ui: &mut egui::Ui,
    state: &SharedState,
    client_pub: &flume::Sender<ClientRequest>,
    search_query: &mut String,
    _selected_track: &mut Option<usize>,
    image_cache: &mut ImageCache,
) -> Action {
    let action = Action::None;

    theme::page_title(ui, "Search");

    ui.horizontal(|ui| {
        ui.add_space(24.0);

        let search_width = (ui.available_width() - 48.0).min(600.0);
        let search_rect = ui
            .allocate_exact_size(egui::vec2(search_width, 44.0), egui::Sense::click())
            .0;

        // Search input background
        ui.painter().rect_filled(search_rect, 22.0, theme::BG_INPUT);

        // Search icon
        ui.painter().text(
            search_rect.left_center() + egui::vec2(16.0, 0.0),
            egui::Align2::CENTER_CENTER,
            "🔍",
            egui::FontId::proportional(16.0),
            theme::TEXT_DIM,
        );

        // Text input
        let text_rect = egui::Rect::from_min_size(
            search_rect.min + egui::vec2(44.0, 4.0),
            egui::vec2(search_rect.width() - 80.0, search_rect.height() - 8.0),
        );
        let response = ui.put(
            text_rect,
            egui::TextEdit::singleline(search_query)
                .hint_text(
                    egui::RichText::new("What do you want to listen to?")
                        .color(theme::TEXT_MUTED),
                )
                .frame(false),
        );

        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            if !search_query.is_empty() {
                let _ = client_pub.send(ClientRequest::Search(search_query.clone()));
            }
        }

        // Search button
        if theme::secondary_button(ui, "Search").clicked() {
            if !search_query.is_empty() {
                let _ = client_pub.send(ClientRequest::Search(search_query.clone()));
            }
        }
    });

    ui.add_space(24.0);

    let data = state.data.read();
    if let Some(results) = data.caches.search.get(search_query) {
        if !results.tracks.is_empty() {
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.label(
                    egui::RichText::new("Songs")
                        .size(20.0)
                        .strong()
                        .color(theme::TEXT_PRIMARY),
                );
            });
            ui.add_space(8.0);

            egui::ScrollArea::vertical()
                .id_salt("search_tracks")
                .max_height(300.0)
                .show(ui, |ui| {
                    for (i, track) in results.tracks.iter().enumerate() {
                        let row_height = 48.0;
                        let (row_rect, response) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), row_height),
                            egui::Sense::click(),
                        );

                        let bg = if response.hovered() {
                            theme::BG_CARD
                        } else {
                            egui::Color32::TRANSPARENT
                        };
                        ui.painter().rect_filled(row_rect, 4.0, bg);

                        ui.painter().text(
                            row_rect.left_center() + egui::vec2(28.0, 0.0),
                            egui::Align2::CENTER_CENTER,
                            format!("{}", i + 1),
                            egui::FontId::monospace(12.0),
                            theme::TEXT_MUTED,
                        );

                        // Thumbnail
                        let thumb_rect = egui::Rect::from_min_size(
                            row_rect.left_center() + egui::vec2(48.0, -theme::TRACK_THUMB_SIZE / 2.0),
                            egui::vec2(theme::TRACK_THUMB_SIZE, theme::TRACK_THUMB_SIZE),
                        );
                        let mut thumb_drawn = false;
                        if let Some(ref album) = track.album {
                            if let Some(path) = image_cache::album_cover_path(album) {
                                if let Some(texture) = image_cache.get_texture(ui.ctx(), &path) {
                                    ui.painter().rect_filled(
                                        thumb_rect,
                                        theme::ART_CORNER_RADIUS,
                                        theme::BG_ACTIVE,
                                    );
                                    egui::Image::new(texture)
                                        .corner_radius(theme::ART_CORNER_RADIUS)
                                        .paint_at(ui, thumb_rect);
                                    thumb_drawn = true;
                                }
                            }
                        }
                        if !thumb_drawn {
                            ui.painter().rect_filled(
                                thumb_rect,
                                theme::ART_CORNER_RADIUS,
                                theme::BG_ACTIVE,
                            );
                            ui.painter().text(
                                thumb_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "\u{266B}",
                                egui::FontId::proportional(14.0),
                                theme::TEXT_MUTED,
                            );
                        }

                        ui.painter().text(
                            row_rect.left_center() + egui::vec2(92.0, -7.0),
                            egui::Align2::LEFT_CENTER,
                            &track.name,
                            egui::FontId::proportional(14.0),
                            theme::TEXT_PRIMARY,
                        );
                        ui.painter().text(
                            row_rect.left_center() + egui::vec2(92.0, 10.0),
                            egui::Align2::LEFT_CENTER,
                            track.artists_info(),
                            egui::FontId::proportional(12.0),
                            theme::TEXT_DIM,
                        );
                        ui.painter().text(
                            row_rect.right_center() + egui::vec2(-24.0, 0.0),
                            egui::Align2::RIGHT_CENTER,
                            theme::format_duration_secs(track.duration.as_secs()),
                            egui::FontId::monospace(12.0),
                            theme::TEXT_DIM,
                        );

                        if response.hovered() {
                            let play_btn_rect = egui::Rect::from_center_size(
                                row_rect.left_center() + egui::vec2(28.0, 0.0),
                                egui::vec2(24.0, 24.0),
                            );
                            ui.painter().rect_filled(play_btn_rect, 12.0, theme::GREEN);
                            ui.painter().text(
                                play_btn_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "\u{25B6}",
                                egui::FontId::proportional(10.0),
                                theme::BG_BLACK,
                            );
                        }

                        let div = egui::Rect::from_min_size(
                            row_rect.left_bottom() + egui::vec2(24.0, 0.0),
                            egui::vec2(row_rect.width() - 48.0, 1.0),
                        );
                        ui.painter().rect_filled(div, 0.0, theme::DIVIDER);
                    }
                });

            ui.add_space(16.0);
        }

        if !results.artists.is_empty() {
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.label(
                    egui::RichText::new("Artists")
                        .size(20.0)
                        .strong()
                        .color(theme::TEXT_PRIMARY),
                );
            });
            ui.add_space(8.0);

            egui::ScrollArea::horizontal()
                .id_salt("search_artists_h")
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(24.0);
                        for artist in results.artists.iter() {
                            let (rect, response) = ui.allocate_exact_size(
                                egui::vec2(160.0, 200.0),
                                egui::Sense::click(),
                            );

                            let bg = if response.hovered() {
                                theme::BG_HOVER
                            } else {
                                theme::BG_CARD
                            };
                            ui.painter().rect_filled(rect, 8.0, bg);

                            // Artist circle
                            let circle_rect = egui::Rect::from_center_size(
                                rect.center() + egui::vec2(0.0, -30.0),
                                egui::vec2(100.0, 100.0),
                            );
                            ui.painter().rect_filled(circle_rect, 50.0, theme::BG_ACTIVE);
                            ui.painter().text(
                                circle_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "🎤",
                                egui::FontId::proportional(28.0),
                                theme::TEXT_MUTED,
                            );

                            ui.painter().text(
                                rect.center() + egui::vec2(0.0, 40.0),
                                egui::Align2::CENTER_CENTER,
                                &artist.name,
                                egui::FontId::proportional(13.0),
                                theme::TEXT_PRIMARY,
                            );
                            ui.painter().text(
                                rect.center() + egui::vec2(0.0, 58.0),
                                egui::Align2::CENTER_CENTER,
                                "Artist",
                                egui::FontId::proportional(11.0),
                                theme::TEXT_DIM,
                            );

                            ui.add_space(12.0);
                        }
                    });
                });
            ui.add_space(16.0);
        }

        if !results.albums.is_empty() {
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.label(
                    egui::RichText::new("Albums")
                        .size(20.0)
                        .strong()
                        .color(theme::TEXT_PRIMARY),
                );
            });
            ui.add_space(8.0);

            egui::ScrollArea::horizontal()
                .id_salt("search_albums_h")
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(24.0);
                        for album in results.albums.iter() {
                            let sub = format!(
                                "{} · {}",
                                album.artists.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", "),
                                album.year()
                            );
                            let cover_path = image_cache::album_cover_path(album);
                            if let (Some(path), Some(url)) = (&cover_path, &album.cover_url) {
                                if !path.exists() {
                                    image_cache.request_download(url, path);
                                }
                            }
                            search_grid_card(ui, &album.name, &sub, cover_path.as_deref(), image_cache);
                            ui.add_space(12.0);
                        }
                    });
                });
        }

        if !results.playlists.is_empty() {
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.label(
                    egui::RichText::new("Playlists")
                        .size(20.0)
                        .strong()
                        .color(theme::TEXT_PRIMARY),
                );
            });
            ui.add_space(8.0);

            egui::ScrollArea::horizontal()
                .id_salt("search_playlists_h")
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(24.0);
                        for playlist in results.playlists.iter() {
                            let cover_path = image_cache::playlist_cover_path(playlist);
                            if let (Some(path), Some(url)) = (&cover_path, &playlist.cover_url) {
                                if !path.exists() {
                                    image_cache.request_download(url, path);
                                }
                            }
                            search_grid_card(ui, &playlist.name, &playlist.owner.0, cover_path.as_deref(), image_cache);
                            ui.add_space(12.0);
                        }
                    });
                });
        }
    } else if search_query.is_empty() {
        ui.allocate_space(egui::vec2(ui.available_width(), 80.0));
        ui.horizontal(|ui| {
            ui.add_space(ui.available_width() / 2.0 - 100.0);
            ui.label(
                egui::RichText::new("🔍")
                    .size(48.0)
                    .color(theme::TEXT_MUTED),
            );
        });
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            ui.add_space(ui.available_width() / 2.0 - 120.0);
            ui.label(
                egui::RichText::new("Search for your favorite music")
                    .size(16.0)
                    .color(theme::TEXT_DIM),
            );
        });
    } else {
        ui.horizontal(|ui| {
            ui.add_space(ui.available_width() / 2.0 - 30.0);
            ui.spinner();
        });
    }

    action
}

fn search_grid_card(
    ui: &mut egui::Ui,
    title: &str,
    subtitle: &str,
    cover_path: Option<&std::path::Path>,
    image_cache: &mut ImageCache,
) {
    let width = 160.0;
    let height = 200.0;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());

    let bg = if response.hovered() {
        theme::BG_HOVER
    } else {
        theme::BG_CARD
    };
    ui.painter().rect_filled(rect, 8.0, bg);

    let art_size = width - 24.0;
    let art_rect = egui::Rect::from_min_size(
        rect.min + egui::vec2(12.0, 12.0),
        egui::vec2(art_size, art_size),
    );

    let mut art_drawn = false;
    if let Some(path) = cover_path {
        if let Some(texture) = image_cache.get_texture(ui.ctx(), path) {
            ui.painter().rect_filled(art_rect, theme::ART_CORNER_RADIUS, theme::BG_ACTIVE);
            egui::Image::new(texture)
                .corner_radius(theme::ART_CORNER_RADIUS)
                .paint_at(ui, art_rect);
            art_drawn = true;
        }
    }

    if !art_drawn {
        ui.painter().rect_filled(art_rect, theme::ART_CORNER_RADIUS, theme::BG_ACTIVE);
        if response.hovered() {
            let play_rect = egui::Rect::from_center_size(
                art_rect.center() + egui::vec2(0.0, 10.0),
                egui::vec2(40.0, 40.0),
            );
            ui.painter().rect_filled(play_rect, 20.0, theme::GREEN);
            ui.painter().text(
                play_rect.center(),
                egui::Align2::CENTER_CENTER,
                "\u{25B6}",
                egui::FontId::proportional(16.0),
                theme::BG_BLACK,
            );
        } else {
            ui.painter().text(
                art_rect.center(),
                egui::Align2::CENTER_CENTER,
                "\u{266B}",
                egui::FontId::proportional(28.0),
                theme::TEXT_MUTED,
            );
        }
    }

    if art_drawn && response.hovered() {
        ui.painter().rect_filled(
            art_rect,
            theme::ART_CORNER_RADIUS,
            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 100),
        );
        let play_rect = egui::Rect::from_center_size(
            art_rect.center(),
            egui::vec2(40.0, 40.0),
        );
        ui.painter().rect_filled(play_rect, 20.0, theme::GREEN);
        ui.painter().text(
            play_rect.center(),
            egui::Align2::CENTER_CENTER,
            "\u{25B6}",
            egui::FontId::proportional(16.0),
            theme::BG_BLACK,
        );
    }

    ui.painter().text(
        rect.left_top() + egui::vec2(12.0, art_size + 26.0),
        egui::Align2::LEFT_TOP,
        title,
        egui::FontId::proportional(13.0),
        theme::TEXT_PRIMARY,
    );
    ui.painter().text(
        rect.left_top() + egui::vec2(12.0, art_size + 46.0),
        egui::Align2::LEFT_TOP,
        subtitle,
        egui::FontId::proportional(11.0),
        theme::TEXT_DIM,
    );
}

pub fn render_queue(
    ui: &mut egui::Ui,
    state: &SharedState,
    _client_pub: &flume::Sender<ClientRequest>,
    _image_cache: &mut ImageCache,
) {
    

    theme::page_title(ui, "Queue");

    let player = state.player.read();

    // Now Playing
    if let Some(ref playback) = player.playback {
        if let Some(ref item) = playback.item {
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.label(
                    egui::RichText::new("Now Playing")
                        .size(14.0)
                        .strong()
                        .color(theme::TEXT_DIM),
                );
            });
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.add_space(24.0);
                let width = ui.available_width() - 48.0;
                let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 64.0), egui::Sense::hover());

                ui.painter().rect_filled(rect, 8.0, theme::BG_CARD);

                // Green left accent
                ui.painter().rect_filled(
                    egui::Rect::from_min_size(rect.min, egui::vec2(3.0, rect.height())),
                    1.5,
                    theme::GREEN,
                );

                match item {
                    rspotify::model::PlayableItem::Track(track) => {
                        ui.painter().text(
                            rect.left_center() + egui::vec2(24.0, -8.0),
                            egui::Align2::LEFT_CENTER,
                            &track.name,
                            egui::FontId::proportional(15.0),
                            theme::GREEN,
                        );
                        let artists: Vec<_> = track.artists.iter().map(|a| a.name.as_str()).collect();
                        ui.painter().text(
                            rect.left_center() + egui::vec2(24.0, 10.0),
                            egui::Align2::LEFT_CENTER,
                            artists.join(", "),
                            egui::FontId::proportional(12.0),
                            theme::TEXT_DIM,
                        );
                    }
                    rspotify::model::PlayableItem::Episode(ep) => {
                        ui.painter().text(
                            rect.left_center() + egui::vec2(24.0, -8.0),
                            egui::Align2::LEFT_CENTER,
                            &ep.name,
                            egui::FontId::proportional(15.0),
                            theme::GREEN,
                        );
                        ui.painter().text(
                            rect.left_center() + egui::vec2(24.0, 10.0),
                            egui::Align2::LEFT_CENTER,
                            &ep.show.name,
                            egui::FontId::proportional(12.0),
                            theme::TEXT_DIM,
                        );
                    }
                    _ => {}
                }
            });

            ui.add_space(24.0);
        }
    }

    // Queue
    if let Some(ref queue) = player.queue {
        ui.horizontal(|ui| {
            ui.add_space(24.0);
            ui.label(
                egui::RichText::new("Next Up")
                    .size(14.0)
                    .strong()
                    .color(theme::TEXT_DIM),
            );
        });
        ui.add_space(8.0);

        egui::ScrollArea::vertical()
            .id_salt("queue_scroll")
            .show(ui, |ui| {
                for (i, item) in queue.queue.iter().enumerate() {
                    let row_height = 48.0;
                    let (row_rect, response) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), row_height),
                        egui::Sense::click(),
                    );

                    let bg = if response.hovered() {
                        theme::BG_CARD
                    } else {
                        egui::Color32::TRANSPARENT
                    };
                    ui.painter().rect_filled(row_rect, 4.0, bg);

                    ui.painter().text(
                        row_rect.left_center() + egui::vec2(40.0, 0.0),
                        egui::Align2::CENTER_CENTER,
                        format!("{}", i + 1),
                        egui::FontId::monospace(12.0),
                        theme::TEXT_MUTED,
                    );

                    match item {
                        rspotify::model::PlayableItem::Track(track) => {
                            ui.painter().text(
                                row_rect.left_center() + egui::vec2(80.0, -7.0),
                                egui::Align2::LEFT_CENTER,
                                &track.name,
                                egui::FontId::proportional(14.0),
                                theme::TEXT_PRIMARY,
                            );
                            let artists: Vec<_> = track.artists.iter().map(|a| a.name.as_str()).collect();
                            ui.painter().text(
                                row_rect.left_center() + egui::vec2(80.0, 10.0),
                                egui::Align2::LEFT_CENTER,
                                artists.join(", "),
                                egui::FontId::proportional(12.0),
                                theme::TEXT_DIM,
                            );
                            let dur = theme::format_duration_secs(track.duration.num_seconds() as u64);
                            ui.painter().text(
                                row_rect.right_center() + egui::vec2(-24.0, 0.0),
                                egui::Align2::RIGHT_CENTER,
                                &dur,
                                egui::FontId::monospace(12.0),
                                theme::TEXT_DIM,
                            );
                        }
                        rspotify::model::PlayableItem::Episode(ep) => {
                            ui.painter().text(
                                row_rect.left_center() + egui::vec2(80.0, -7.0),
                                egui::Align2::LEFT_CENTER,
                                &ep.name,
                                egui::FontId::proportional(14.0),
                                theme::TEXT_PRIMARY,
                            );
                            ui.painter().text(
                                row_rect.left_center() + egui::vec2(80.0, 10.0),
                                egui::Align2::LEFT_CENTER,
                                &ep.show.name,
                                egui::FontId::proportional(12.0),
                                theme::TEXT_DIM,
                            );
                        }
                        _ => {}
                    }

                    let div = egui::Rect::from_min_size(
                        row_rect.left_bottom() + egui::vec2(24.0, 0.0),
                        egui::vec2(row_rect.width() - 48.0, 1.0),
                    );
                    ui.painter().rect_filled(div, 0.0, theme::DIVIDER);
                }
            });
    } else {
        ui.add_space(40.0);
        ui.horizontal(|ui| {
            ui.add_space(ui.available_width() / 2.0 - 60.0);
            ui.label(
                egui::RichText::new("Queue is empty")
                    .size(16.0)
                    .color(theme::TEXT_DIM),
            );
        });
    }
}

pub fn render_settings(ui: &mut egui::Ui) {
    theme::page_title(ui, "Settings");

    ui.horizontal(|ui| {
        ui.add_space(24.0);
        let width = ui.available_width() - 48.0;
        theme::card(ui, |ui| {
            ui.set_width(width - 32.0);
            ui.label(
                egui::RichText::new("Configuration")
                    .size(18.0)
                    .strong()
                    .color(theme::TEXT_PRIMARY),
            );
            ui.add_space(12.0);
            ui.label(
                egui::RichText::new("Edit the config files to customize the app:")
                    .color(theme::TEXT_SECONDARY),
            );
            ui.add_space(12.0);

            let paths = [
                ("~/.config/spotify-player/app.toml", "Application settings"),
                ("~/.config/spotify-player/theme.toml", "Theme configuration"),
                ("~/.config/spotify-player/keymap.toml", "Key bindings"),
            ];

            for (path, desc) in &paths {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(*path)
                            .monospace()
                            .size(13.0)
                            .color(theme::GREEN),
                    );
                    ui.label(
                        egui::RichText::new(format!("— {desc}"))
                            .size(12.0)
                            .color(theme::TEXT_DIM),
                    );
                });
                ui.add_space(4.0);
            }
        });
    });
}

pub fn render_lyrics(
    ui: &mut egui::Ui,
    state: &SharedState,
    _client_pub: &flume::Sender<ClientRequest>,
    _image_cache: &mut ImageCache,
) -> Action {
    // Background fill
    let full_rect = ui.max_rect();
    ui.painter().rect_filled(full_rect, 0.0, theme::LYRICS_BG);

    let player = state.player.read();
    let playback = player.current_playback();

    let (track_name, artists_str, progress_ms, _duration_ms) = if let Some(ref pb) = playback {
        if let Some(ref item) = pb.item {
            let (name, artists) = match item {
                rspotify::model::PlayableItem::Track(t) => {
                    let a: Vec<_> = t.artists.iter().map(|a| a.name.as_str()).collect();
                    (t.name.clone(), a.join(", "))
                }
                rspotify::model::PlayableItem::Episode(e) => {
                    (e.name.clone(), e.show.name.clone())
                }
                _ => (String::new(), String::new()),
            };
            let dur = match item {
                rspotify::model::PlayableItem::Track(t) => t.duration,
                rspotify::model::PlayableItem::Episode(e) => e.duration,
                _ => chrono::Duration::zero(),
            };
            let prog = pb.progress.unwrap_or(chrono::Duration::zero());
            (name, artists, prog.num_milliseconds().max(0) as u64, dur.num_milliseconds().max(0) as u64)
        } else {
            (String::new(), String::new(), 0, 0)
        }
    } else {
        (String::new(), String::new(), 0, 0)
    };

    let track_uri: Option<String> = playback.as_ref().and_then(|pb| {
        pb.item.as_ref().and_then(|item| match item {
            rspotify::model::PlayableItem::Track(t) => t.id.as_ref().map(|id| id.uri()),
            _ => None,
        })
    });

    drop(player);

    // Header
    ui.add_space(24.0);
    ui.horizontal(|ui| {
        ui.add_space(32.0);
        ui.label(
            egui::RichText::new("Lyrics")
                .size(28.0)
                .strong()
                .color(theme::TEXT_PRIMARY),
        );
    });
    ui.add_space(16.0);

    // Track info
    if !track_name.is_empty() {
        ui.horizontal(|ui| {
            ui.add_space(32.0);
            ui.label(
                egui::RichText::new(&track_name)
                    .size(18.0)
                    .strong()
                    .color(theme::TEXT_PRIMARY),
            );
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add_space(32.0);
            ui.label(
                egui::RichText::new(&artists_str)
                    .size(14.0)
                    .color(theme::TEXT_DIM),
            );
        });
    }
    ui.add_space(20.0);

    // Divider
    let div_rect = ui.allocate_space(egui::vec2(ui.available_width(), 1.0)).1;
    ui.painter().rect_filled(div_rect, 0.0, theme::DIVIDER);
    ui.add_space(16.0);

    // Lyrics content
    let data = state.data.read();
    let lyrics = track_uri.as_ref().and_then(|uri| data.caches.lyrics.get(uri));

    match lyrics {
        Some(Some(lyrics)) if !lyrics.lines.is_empty() => {
            let lines = &lyrics.lines;

            // Find current line index
            let current_idx = {
                let mut idx = 0;
                for (i, (ts, _)) in lines.iter().enumerate() {
                    if ts.num_milliseconds() as u64 <= progress_ms {
                        idx = i;
                    } else {
                        break;
                    }
                }
                idx
            };

            egui::ScrollArea::vertical()
                .id_salt("lyrics_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    // Top padding to allow scrolling first line to center
                    let viewport_height = ui.available_height().max(200.0);
                    ui.add_space(viewport_height / 2.0 - 40.0);

                    for (i, (_ts, text)) in lines.iter().enumerate() {
                        let (color, size, is_bold) = if i < current_idx {
                            (theme::LYRICS_PLAYED, 16.0, false)
                        } else if i == current_idx {
                            (theme::LYRICS_CURRENT, 20.0, true)
                        } else {
                            (theme::LYRICS_UPCOMING, 16.0, false)
                        };

                        let font_id = if is_bold {
                            egui::FontId::proportional(size)
                        } else {
                            egui::FontId::proportional(size)
                        };

                        let line_height = size * 1.6;

                        let (line_rect, _response) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), line_height),
                            egui::Sense::hover(),
                        );

                        let rich_text = egui::RichText::new(text)
                            .size(size)
                            .color(color);

                        if is_bold {
                            let rich_text = rich_text.strong();
                            ui.painter().text(
                                line_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                rich_text.text(),
                                font_id,
                                color,
                            );
                        } else {
                            ui.painter().text(
                                line_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                text,
                                font_id,
                                color,
                            );
                        }
                    }

                    // Bottom padding
                    ui.add_space(viewport_height / 2.0 - 40.0);
                });
        }
        Some(Some(_)) => {
            // Empty lyrics
            render_no_lyrics(ui);
        }
        Some(None) => {
            render_no_lyrics(ui);
        }
        None => {
            // Still loading or not requested
            if track_name.is_empty() {
                ui.add_space(80.0);
                ui.horizontal(|ui| {
                    ui.add_space(ui.available_width() / 2.0 - 100.0);
                    ui.label(
                        egui::RichText::new("No track playing")
                            .size(16.0)
                            .color(theme::TEXT_DIM),
                    );
                });
            } else {
                ui.add_space(80.0);
                ui.horizontal(|ui| {
                    ui.add_space(ui.available_width() / 2.0 - 30.0);
                    ui.spinner();
                });
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.add_space(ui.available_width() / 2.0 - 60.0);
                    ui.label(
                        egui::RichText::new("Loading lyrics...")
                            .size(14.0)
                            .color(theme::TEXT_DIM),
                    );
                });
            }
        }
    }

    Action::None
}

fn render_no_lyrics(ui: &mut egui::Ui) {
    ui.add_space(80.0);
    ui.horizontal(|ui| {
        ui.add_space(ui.available_width() / 2.0 - 16.0);
        ui.label(
            egui::RichText::new("🎤")
                .size(48.0)
                .color(theme::TEXT_MUTED),
        );
    });
    ui.add_space(16.0);
    ui.horizontal(|ui| {
        ui.add_space(ui.available_width() / 2.0 - 90.0);
        ui.label(
            egui::RichText::new("No lyrics available")
                .size(16.0)
                .color(theme::TEXT_DIM),
        );
    });
}
