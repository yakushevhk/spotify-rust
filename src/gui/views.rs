use eframe::egui;
use rspotify::prelude::Id;

use crate::client::{ClientRequest, PlayerRequest};
use crate::gui::context_menu::{self, ContextTarget};
use crate::gui::image_cache::{self, ImageCache};
use crate::gui::{theme, Action, SortAction, SortColumn, SortDirection, SortState, View};
use crate::state::{self, PlayableId, SharedState};

pub fn render_library(
    ui: &mut egui::Ui,
    state: &SharedState,
    image_cache: &mut ImageCache,
    context_menu: &mut context_menu::ContextMenu,
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
                        .color(theme::text_primary()),
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
                            let response = grid_card(
                                ui,
                                &playlist.name,
                                &playlist.owner.0,
                                cover_path.as_deref(),
                                image_cache,
                                || {
                                    action = Action::OpenSearchResultPlaylist(playlist.clone());
                                },
                            );
                            if response.secondary_clicked() {
                                if let Some(click_pos) = response.interact_pointer_pos() {
                                    context_menu.open(
                                        ContextTarget::Playlist(playlist.clone()),
                                        click_pos,
                                    );
                                }
                            }
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
                        .color(theme::text_primary()),
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
                            let response = grid_card(ui, &album.name, &sub, cover_path.as_deref(), image_cache, || {
                                action = Action::OpenSearchResultAlbum(album.clone());
                            });
                            if response.secondary_clicked() {
                                if let Some(click_pos) = response.interact_pointer_pos() {
                                    context_menu.open(
                                        ContextTarget::Album(album.clone()),
                                        click_pos,
                                    );
                                }
                            }
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

pub fn render_shows(
    ui: &mut egui::Ui,
    state: &SharedState,
    client_pub: &flume::Sender<ClientRequest>,
    image_cache: &mut ImageCache,
    context_menu: &mut context_menu::ContextMenu,
) -> Action {
    let mut action = Action::None;

    theme::page_title(ui, "Your Shows");

    let data = state.data.read();
    let shows: Vec<_> = data.user_data.saved_shows.clone();
    drop(data);

    if shows.is_empty() {
        // Request shows from API
        let _ = client_pub.send(ClientRequest::GetUserSavedShows);
        ui.add_space(60.0);
        ui.horizontal(|ui| {
            ui.add_space(ui.available_width() / 2.0 - 30.0);
            ui.spinner();
        });
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            ui.add_space(ui.available_width() / 2.0 - 80.0);
            ui.label(
                egui::RichText::new("Loading shows...")
                    .size(16.0)
                    .color(theme::text_dim()),
            );
        });
    } else {
        egui::ScrollArea::vertical()
            .id_salt("shows_scroll")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(24.0);
                });

                egui::Grid::new("shows_grid")
                    .num_columns(4)
                    .spacing([16.0, 16.0])
                    .show(ui, |ui| {
                        for (i, show) in shows.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.add_space(24.0);
                                let cover_path = image_cache::show_cover_path(show);
                                if let (Some(path), Some(url)) = (&cover_path, &show.cover_url) {
                                    if !path.exists() {
                                        image_cache.request_download(url, path);
                                    }
                                }
                                let response = grid_card(
                                    ui,
                                    &show.name,
                                    &show.publisher,
                                    cover_path.as_deref(),
                                    image_cache,
                                    || {
                                        action = Action::OpenShowDetail(show.clone());
                                    },
                                );
                                if response.secondary_clicked() {
                                    if let Some(click_pos) = response.interact_pointer_pos() {
                                        context_menu.open(
                                            context_menu::ContextTarget::Show(show.clone()),
                                            click_pos,
                                        );
                                    }
                                }
                            });
                            if (i + 1) % 4 == 0 {
                                ui.end_row();
                            }
                        }
                    });

                ui.add_space(24.0);
            });
    }

    action
}

pub fn render_show_detail(
    ui: &mut egui::Ui,
    state: &SharedState,
    client_pub: &flume::Sender<ClientRequest>,
    show: &Option<state::Show>,
    episodes: &[state::Episode],
    context_id: &Option<state::ContextId>,
    selected_episode: &mut Option<usize>,
    image_cache: &mut ImageCache,
    context_menu: &mut context_menu::ContextMenu,
) -> Action {
    let mut action = Action::None;

    // Back button
    ui.add_space(16.0);
    ui.horizontal(|ui| {
        ui.add_space(24.0);
        let back_rect = ui.allocate_exact_size(egui::vec2(80.0, 32.0), egui::Sense::click());
        let bg = if back_rect.1.hovered() {
            theme::bg_hover()
        } else {
            theme::bg_card()
        };
        ui.painter().rect_filled(back_rect.0, 6.0, bg);
        ui.painter().text(
            back_rect.0.center(),
            egui::Align2::CENTER_CENTER,
            "\u{2190} Back",
            egui::FontId::proportional(13.0),
            theme::text_primary(),
        );
        if back_rect.1.clicked() {
            action = Action::Navigate(View::Shows);
        }
    });

    let Some(show) = show else {
        ui.add_space(60.0);
        ui.horizontal(|ui| {
            ui.add_space(ui.available_width() / 2.0 - 30.0);
            ui.spinner();
        });
        return action;
    };

    // Show header with cover art
    let header_height = 220.0;
    let (header_rect, _) = ui
        .allocate_exact_size(egui::vec2(ui.available_width(), header_height), egui::Sense::hover());

    // Cover art
    let art_size = 160.0;
    let art_rect = egui::Rect::from_min_size(
        header_rect.min + egui::vec2(24.0, 20.0),
        egui::vec2(art_size, art_size),
    );

    let mut art_drawn = false;
    if let Some(path) = image_cache::show_cover_path(show) {
        if let (Some(path_ref), Some(url)) = (Some(&path), &show.cover_url) {
            if !path_ref.exists() {
                image_cache.request_download(url, path_ref);
            }
        }
        if let Some(texture) = image_cache.get_texture(ui.ctx(), &path) {
            ui.painter().rect_filled(art_rect, theme::ART_CORNER_RADIUS, theme::bg_active());
            egui::Image::new(texture)
                .corner_radius(theme::ART_CORNER_RADIUS)
                .paint_at(ui, art_rect);
            art_drawn = true;
        }
    }

    if !art_drawn {
        ui.painter().rect_filled(art_rect, theme::ART_CORNER_RADIUS, theme::bg_active());
        ui.painter().text(
            art_rect.center(),
            egui::Align2::CENTER_CENTER,
            "\u{1F399}",
            egui::FontId::proportional(48.0),
            theme::text_muted(),
        );
    }

    // Show info
    let info_x = art_rect.right() + 24.0;
    ui.painter().text(
        egui::pos2(info_x, header_rect.top() + 30.0),
        egui::Align2::LEFT_TOP,
        &show.name,
        egui::FontId::proportional(28.0),
        theme::text_primary(),
    );

    ui.painter().text(
        egui::pos2(info_x, header_rect.top() + 68.0),
        egui::Align2::LEFT_TOP,
        &show.publisher,
        egui::FontId::proportional(14.0),
        theme::text_dim(),
    );

    // Description (truncated)
    let desc_lines: Vec<&str> = show.description.lines().take(3).collect();
    let desc_text = desc_lines.join(" ");
    let truncated = if desc_text.len() > 200 {
        format!("{}...", &desc_text[..200])
    } else {
        desc_text
    };
    ui.painter().text(
        egui::pos2(info_x, header_rect.top() + 92.0),
        egui::Align2::LEFT_TOP,
        &truncated,
        egui::FontId::proportional(12.0),
        theme::text_secondary(),
    );

    // Follow/Unfollow button
    let is_followed = {
        let data = state.data.read();
        data.user_data.saved_shows.iter().any(|s| s.id == show.id)
    };

    let btn_text = if is_followed { "Following" } else { "Follow" };
    let btn_width = 100.0;
    let btn_rect = egui::Rect::from_min_size(
        egui::pos2(info_x, header_rect.bottom() - 50.0),
        egui::vec2(btn_width, 32.0),
    );
    let btn_resp = ui.allocate_rect(btn_rect, egui::Sense::click());
    let btn_bg = if is_followed {
        if btn_resp.hovered() {
            theme::bg_hover()
        } else {
            egui::Color32::TRANSPARENT
        }
    } else {
        if btn_resp.hovered() {
            theme::green_hover()
        } else {
            theme::green()
        }
    };
    ui.painter().rect_filled(btn_rect, 16.0, btn_bg);
    if is_followed {
        ui.painter().rect_stroke(
            btn_rect,
            16.0,
            egui::Stroke::new(1.0, theme::text_muted()),
            egui::StrokeKind::Outside,
        );
    }
    let btn_text_color = if is_followed {
        theme::text_primary()
    } else {
        egui::Color32::BLACK
    };
    ui.painter().text(
        btn_rect.center(),
        egui::Align2::CENTER_CENTER,
        btn_text,
        egui::FontId::proportional(13.0),
        btn_text_color,
    );
    if btn_resp.clicked() {
        if is_followed {
            let _ = client_pub.send(ClientRequest::DeleteFromLibrary(state::ItemId::Show(
                show.id.clone(),
            )));
        } else {
            let _ = client_pub.send(ClientRequest::AddToLibrary(state::Item::Show(show.clone())));
        }
    }

    ui.add_space(8.0);

    // Episodes list
    let player = state.player.read();
    let current_track_uri: Option<String> = player.playback.as_ref().and_then(|p| {
        p.item.as_ref().map(|item| match item {
            rspotify::model::PlayableItem::Episode(e) => e.id.uri(),
            _ => String::new(),
        })
    });
    drop(player);

    if episodes.is_empty() {
        ui.add_space(40.0);
        ui.horizontal(|ui| {
            ui.add_space(ui.available_width() / 2.0 - 30.0);
            ui.spinner();
        });
    } else {
        theme::page_title(ui, &format!("Episodes ({})", episodes.len()));

        egui::ScrollArea::vertical()
            .id_salt("show_episodes_scroll")
            .show(ui, |ui| {
                for (i, episode) in episodes.iter().enumerate() {
                    let is_playing = current_track_uri
                        .as_ref()
                        .map_or(false, |uri| uri == &episode.id.uri());
                    let is_selected = *selected_episode == Some(i);

                    let row_height = 64.0;
                    let (row_rect, response) = ui
                        .allocate_exact_size(
                            egui::vec2(ui.available_width(), row_height),
                            egui::Sense::click(),
                        );

                    let bg = if is_selected {
                        theme::bg_selected()
                    } else if response.hovered() {
                        theme::bg_card()
                    } else {
                        egui::Color32::TRANSPARENT
                    };
                    ui.painter().rect_filled(row_rect, 4.0, bg);

                    // Green left accent for playing episode
                    if is_playing {
                        ui.painter().rect_filled(
                            egui::Rect::from_min_size(row_rect.min, egui::vec2(3.0, row_rect.height())),
                            1.5,
                            theme::green(),
                        );
                    }

                    // Episode number / play indicator
                    let num_color = if is_playing {
                        theme::green()
                    } else {
                        theme::text_muted()
                    };
                    let num_str = if is_playing {
                        "\u{25B6}".to_string()
                    } else {
                        format!("{}", i + 1)
                    };
                    ui.painter().text(
                        row_rect.left_center() + egui::vec2(28.0, 0.0),
                        egui::Align2::CENTER_CENTER,
                        &num_str,
                        egui::FontId::monospace(12.0),
                        num_color,
                    );

                    // Episode name
                    let title_color = if is_playing {
                        theme::green()
                    } else {
                        theme::text_primary()
                    };
                    ui.painter().text(
                        row_rect.left_center() + egui::vec2(60.0, -12.0),
                        egui::Align2::LEFT_CENTER,
                        &episode.name,
                        egui::FontId::proportional(14.0),
                        title_color,
                    );

                    // Episode description (truncated)
                    let desc = if episode.description.len() > 120 {
                        format!("{}...", &episode.description[..120])
                    } else {
                        episode.description.clone()
                    };
                    ui.painter().text(
                        row_rect.left_center() + egui::vec2(60.0, 8.0),
                        egui::Align2::LEFT_CENTER,
                        &desc,
                        egui::FontId::proportional(11.0),
                        theme::text_dim(),
                    );

                    // Release date
                    ui.painter().text(
                        row_rect.right_center() + egui::vec2(-140.0, -8.0),
                        egui::Align2::RIGHT_CENTER,
                        &episode.release_date,
                        egui::FontId::proportional(11.0),
                        theme::text_dim(),
                    );

                    // Duration
                    let dur_str = theme::format_duration_secs(episode.duration.as_secs());
                    ui.painter().text(
                        row_rect.right_center() + egui::vec2(-52.0, -8.0),
                        egui::Align2::RIGHT_CENTER,
                        &dur_str,
                        egui::FontId::monospace(12.0),
                        theme::text_dim(),
                    );

                    // "..." button on hover
                    let more_btn_rect = egui::Rect::from_center_size(
                        row_rect.right_center() + egui::vec2(-16.0, -8.0),
                        egui::vec2(24.0, 24.0),
                    );
                    if response.hovered() {
                        let more_resp = ui.allocate_rect(more_btn_rect, egui::Sense::click());
                        let more_bg = if more_resp.hovered() {
                            egui::Color32::from_rgb(40, 40, 40)
                        } else {
                            egui::Color32::TRANSPARENT
                        };
                        ui.painter().rect_filled(more_btn_rect, 12.0, more_bg);
                        ui.painter().text(
                            more_btn_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "\u{22EF}",
                            egui::FontId::proportional(14.0),
                            theme::text_dim(),
                        );
                        if more_resp.clicked() {
                            context_menu.open(
                                context_menu::ContextTarget::Episode {
                                    episode: episode.clone(),
                                    show: show.clone().into(),
                                },
                                more_btn_rect.left_bottom(),
                            );
                        }
                    }

                    // Play button on hover
                    if response.hovered() && !is_playing {
                        let play_btn_rect = egui::Rect::from_center_size(
                            row_rect.left_center() + egui::vec2(28.0, 0.0),
                            egui::vec2(24.0, 24.0),
                        );
                        ui.painter()
                            .rect_filled(play_btn_rect, 12.0, theme::green());
                        ui.painter().text(
                            play_btn_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "\u{25B6}",
                            egui::FontId::proportional(10.0),
                            theme::bg_black(),
                        );
                    }

                    // Play on click
                    if response.double_clicked() {
                        *selected_episode = Some(i);
                        if let Some(ref ctx_id) = context_id {
                            let playback = state::Playback::Context(
                                ctx_id.clone(),
                                Some(rspotify::model::Offset::Uri(episode.id.uri())),
                            );
                            let _ = client_pub.send(ClientRequest::Player(
                                PlayerRequest::StartPlayback(playback, None),
                            ));
                        }
                    } else if response.clicked() {
                        *selected_episode = Some(i);
                    }

                    // Right-click context menu
                    if response.secondary_clicked() {
                        if let Some(click_pos) = response.interact_pointer_pos() {
                            context_menu.open(
                                context_menu::ContextTarget::Episode {
                                    episode: episode.clone(),
                                    show: Some(show.clone()),
                                },
                                click_pos,
                            );
                        }
                    }

                    // Row divider
                    let div = egui::Rect::from_min_size(
                        row_rect.left_bottom() + egui::vec2(24.0, 0.0),
                        egui::vec2(row_rect.width() - 48.0, 1.0),
                    );
                    ui.painter().rect_filled(div, 0.0, theme::divider());
                }
            });
    }

    action
}

fn quick_card(ui: &mut egui::Ui, icon: &str, title: &str, desc: &str, width: f32, on_click: impl FnOnce()) {
    let card_height = 80.0;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, card_height), egui::Sense::click());

    let bg = if response.hovered() {
        theme::bg_hover()
    } else {
        theme::bg_card()
    };

    ui.painter().rect_filled(rect, 8.0, bg);

    // Icon circle
    let icon_rect = egui::Rect::from_center_size(
        rect.left_center() + egui::vec2(36.0, 0.0),
        egui::vec2(48.0, 48.0),
    );
    ui.painter().rect_filled(icon_rect, 24.0, theme::green_dark());
    ui.painter().text(
        icon_rect.center(),
        egui::Align2::CENTER_CENTER,
        icon,
        egui::FontId::proportional(22.0),
        theme::text_primary(),
    );

    // Title
    ui.painter().text(
        rect.left_top() + egui::vec2(72.0, 24.0),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(14.0),
        theme::text_primary(),
    );

    // Description
    ui.painter().text(
        rect.left_top() + egui::vec2(72.0, 46.0),
        egui::Align2::LEFT_CENTER,
        desc,
        egui::FontId::proportional(11.0),
        theme::text_dim(),
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
) -> egui::Response {
    let width = 160.0;
    let height = 200.0;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());

    let bg = if response.hovered() {
        theme::bg_hover()
    } else {
        theme::bg_card()
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
            ui.painter().rect_filled(art_rect, theme::ART_CORNER_RADIUS, theme::bg_active());
            egui::Image::new(texture)
                .corner_radius(theme::ART_CORNER_RADIUS)
                .paint_at(ui, art_rect);
            art_drawn = true;
        }
    }

    if !art_drawn {
        ui.painter().rect_filled(art_rect, theme::ART_CORNER_RADIUS, theme::bg_active());
        if response.hovered() {
            let play_rect = egui::Rect::from_center_size(
                art_rect.center() + egui::vec2(0.0, 10.0),
                egui::vec2(40.0, 40.0),
            );
            ui.painter().rect_filled(play_rect, 20.0, theme::green());
            ui.painter().text(
                play_rect.center(),
                egui::Align2::CENTER_CENTER,
                "\u{25B6}",
                egui::FontId::proportional(16.0),
                theme::bg_black(),
            );
        } else {
            ui.painter().text(
                art_rect.center(),
                egui::Align2::CENTER_CENTER,
                "\u{266B}",
                egui::FontId::proportional(28.0),
                theme::text_muted(),
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
        ui.painter().rect_filled(play_rect, 20.0, theme::green());
        ui.painter().text(
            play_rect.center(),
            egui::Align2::CENTER_CENTER,
            "\u{25B6}",
            egui::FontId::proportional(16.0),
            theme::bg_black(),
        );
    }

    // Title
    ui.painter().text(
        rect.left_top() + egui::vec2(12.0, art_size + 26.0),
        egui::Align2::LEFT_TOP,
        title,
        egui::FontId::proportional(13.0),
        theme::text_primary(),
    );

    // Subtitle
    ui.painter().text(
        rect.left_top() + egui::vec2(12.0, art_size + 46.0),
        egui::Align2::LEFT_TOP,
        subtitle,
        egui::FontId::proportional(11.0),
        theme::text_dim(),
    );

    if response.clicked() {
        on_click();
    }

    response
}

pub fn render_tracks(
    ui: &mut egui::Ui,
    state: &SharedState,
    client_pub: &flume::Sender<ClientRequest>,
    title: &str,
    tracks: &[state::Track],
    selected_track: &mut Option<usize>,
    image_cache: &mut ImageCache,
    context_menu: &mut context_menu::ContextMenu,
    playlist_id: Option<&state::PlaylistId<'static>>,
    sort_state: Option<SortState>,
    context_id: Option<&state::ContextId>,
) -> SortAction {
    use rspotify::prelude::Id;

    let mut sort_action = SortAction::None;

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

    // Table header — clickable columns
    let header_color_default = egui::Color32::from_rgb(136, 136, 136); // #888888
    let header_color_hover = egui::Color32::from_rgb(204, 204, 204); // #CCCCCC
    let header_color_active = theme::green(); // #1DB954

    ui.horizontal(|ui| {
        ui.add_space(24.0);
        let header_rect = ui.allocate_space(egui::vec2(ui.available_width() - 24.0, 32.0)).1;

        // "#" column (non-sortable)
        ui.painter().text(
            header_rect.left_center() + egui::vec2(28.0, 0.0),
            egui::Align2::CENTER_CENTER,
            "#",
            egui::FontId::monospace(12.0),
            header_color_default,
        );

        // TITLE column
        let title_x = 92.0;
        let title_active = sort_state.map_or(false, |s| s.column == SortColumn::Title);
        let title_label = if title_active {
            format!("TITLE {}", sort_state.unwrap().direction.arrow())
        } else {
            "TITLE".to_string()
        };
        let title_color = if title_active { header_color_active } else { header_color_default };
        let title_rect = egui::Rect::from_min_size(
            header_rect.left_top() + egui::vec2(title_x - 4.0, 0.0),
            egui::vec2(180.0, header_rect.height()),
        );
        let title_resp = ui.allocate_rect(title_rect, egui::Sense::click());
        let title_color = if title_resp.hovered() && !title_active {
            header_color_hover
        } else {
            title_color
        };
        ui.painter().text(
            title_rect.left_center() + egui::vec2(0.0, 0.0),
            egui::Align2::LEFT_CENTER,
            &title_label,
            egui::FontId::monospace(12.0),
            title_color,
        );
        if title_resp.clicked() {
            let dir = if title_active {
                sort_state.unwrap().direction.toggle()
            } else {
                SortDirection::Ascending
            };
            sort_action = SortAction::Sort(SortState { column: SortColumn::Title, direction: dir });
        }

        // ARTIST column
        let artist_x = 300.0;
        let artist_active = sort_state.map_or(false, |s| s.column == SortColumn::Artist);
        let artist_label = if artist_active {
            format!("ARTIST {}", sort_state.unwrap().direction.arrow())
        } else {
            "ARTIST".to_string()
        };
        let artist_color = if artist_active { header_color_active } else { header_color_default };
        let artist_rect = egui::Rect::from_min_size(
            header_rect.left_top() + egui::vec2(artist_x - 4.0, 0.0),
            egui::vec2(160.0, header_rect.height()),
        );
        let artist_resp = ui.allocate_rect(artist_rect, egui::Sense::click());
        let artist_color = if artist_resp.hovered() && !artist_active {
            header_color_hover
        } else {
            artist_color
        };
        ui.painter().text(
            artist_rect.left_center() + egui::vec2(0.0, 0.0),
            egui::Align2::LEFT_CENTER,
            &artist_label,
            egui::FontId::monospace(12.0),
            artist_color,
        );
        if artist_resp.clicked() {
            let dir = if artist_active {
                sort_state.unwrap().direction.toggle()
            } else {
                SortDirection::Ascending
            };
            sort_action = SortAction::Sort(SortState { column: SortColumn::Artist, direction: dir });
        }

        // ALBUM column (centered)
        let album_active = sort_state.map_or(false, |s| s.column == SortColumn::Album);
        let album_label = if album_active {
            format!("ALBUM {}", sort_state.unwrap().direction.arrow())
        } else {
            "ALBUM".to_string()
        };
        let album_color = if album_active { header_color_active } else { header_color_default };
        let album_rect = egui::Rect::from_center_size(
            header_rect.center(),
            egui::vec2(200.0, header_rect.height()),
        );
        let album_resp = ui.allocate_rect(album_rect, egui::Sense::click());
        let album_color = if album_resp.hovered() && !album_active {
            header_color_hover
        } else {
            album_color
        };
        ui.painter().text(
            album_rect.center(),
            egui::Align2::CENTER_CENTER,
            &album_label,
            egui::FontId::monospace(12.0),
            album_color,
        );
        if album_resp.clicked() {
            let dir = if album_active {
                sort_state.unwrap().direction.toggle()
            } else {
                SortDirection::Ascending
            };
            sort_action = SortAction::Sort(SortState { column: SortColumn::Album, direction: dir });
        }

        // TIME column (right-aligned)
        let time_active = sort_state.map_or(false, |s| s.column == SortColumn::Duration);
        let time_label = if time_active {
            format!("TIME {}", sort_state.unwrap().direction.arrow())
        } else {
            "TIME".to_string()
        };
        let time_color = if time_active { header_color_active } else { header_color_default };
        let time_rect = egui::Rect::from_min_size(
            header_rect.right_top() + egui::vec2(-120.0, 0.0),
            egui::vec2(120.0, header_rect.height()),
        );
        let time_resp = ui.allocate_rect(time_rect, egui::Sense::click());
        let time_color = if time_resp.hovered() && !time_active {
            header_color_hover
        } else {
            time_color
        };
        ui.painter().text(
            time_rect.right_center() + egui::vec2(-2.0, 0.0),
            egui::Align2::RIGHT_CENTER,
            &time_label,
            egui::FontId::monospace(12.0),
            time_color,
        );
        if time_resp.clicked() {
            let dir = if time_active {
                sort_state.unwrap().direction.toggle()
            } else {
                SortDirection::Ascending
            };
            sort_action = SortAction::Sort(SortState { column: SortColumn::Duration, direction: dir });
        }
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
            theme::divider(),
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
                    theme::bg_selected()
                } else if response.hovered() {
                    theme::bg_card()
                } else {
                    egui::Color32::TRANSPARENT
                };

                ui.painter().rect_filled(row_rect, 4.0, bg);

                // Green left accent for playing track
                if is_playing {
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(row_rect.min, egui::vec2(3.0, row_rect.height())),
                        1.5,
                        theme::green(),
                    );
                }

                // Number
                let num_color = if is_playing {
                    theme::green()
                } else {
                    theme::text_muted()
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
                                theme::bg_active(),
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
                        theme::bg_active(),
                    );
                    ui.painter().text(
                        thumb_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "\u{266B}",
                        egui::FontId::proportional(14.0),
                        theme::text_muted(),
                    );
                }

                // Track info
                let title_color = if is_playing {
                    theme::green()
                } else {
                    theme::text_primary()
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
                    theme::text_dim(),
                );

                // Album name (middle)
                let album_name = track.album_info();
                if !album_name.is_empty() {
                    ui.painter().text(
                        row_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        &album_name,
                        egui::FontId::proportional(12.0),
                        theme::text_dim(),
                    );
                }

                // Duration
                let duration = track.duration;
                let dur_str = theme::format_duration_secs(duration.as_secs());
                ui.painter().text(
                    row_rect.right_center() + egui::vec2(-52.0, 0.0),
                    egui::Align2::RIGHT_CENTER,
                    &dur_str,
                    egui::FontId::monospace(12.0),
                    theme::text_dim(),
                );

                // "..." button on hover
                let more_btn_rect = egui::Rect::from_center_size(
                    row_rect.right_center() + egui::vec2(-16.0, 0.0),
                    egui::vec2(24.0, 24.0),
                );
                if response.hovered() {
                    let more_resp = ui.allocate_rect(more_btn_rect, egui::Sense::click());
                    let more_bg = if more_resp.hovered() {
                        egui::Color32::from_rgb(40, 40, 40)
                    } else {
                        egui::Color32::TRANSPARENT
                    };
                    ui.painter().rect_filled(more_btn_rect, 12.0, more_bg);
                    ui.painter().text(
                        more_btn_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "\u{22EF}",
                        egui::FontId::proportional(14.0),
                        theme::text_dim(),
                    );
                    if more_resp.clicked() {
                        context_menu.open(
                            ContextTarget::Track {
                                track: track.clone(),
                                index: i,
                                playlist_id: playlist_id.cloned(),
                            },
                            more_btn_rect.left_bottom(),
                        );
                    }
                }

                // Play button on hover
                if response.hovered() && !is_playing {
                    let play_btn_rect = egui::Rect::from_center_size(
                        row_rect.left_center() + egui::vec2(28.0, 0.0),
                        egui::vec2(24.0, 24.0),
                    );
                    ui.painter()
                        .rect_filled(play_btn_rect, 12.0, theme::green());
                    ui.painter().text(
                        play_btn_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "\u{25B6}",
                        egui::FontId::proportional(10.0),
                        theme::bg_black(),
                    );
                }

                if response.double_clicked() {
                    *selected_track = Some(i);
                    if let Some(ctx) = context_id {
                        let playback = match ctx {
                            state::ContextId::Playlist(_)
                            | state::ContextId::Album(_)
                            | state::ContextId::Artist(_)
                            | state::ContextId::Show(_) => state::Playback::Context(
                                ctx.clone(),
                                Some(rspotify::model::Offset::Uri(track.id.uri())),
                            ),
                            state::ContextId::Tracks(_) => {
                                let uris: Vec<PlayableId<'static>> = tracks
                                    .iter()
                                    .map(|t| PlayableId::Track(t.id.clone()))
                                    .collect();
                                state::Playback::URIs(
                                    uris,
                                    Some(rspotify::model::Offset::Uri(track.id.uri())),
                                )
                            }
                        };
                        let _ = client_pub.send(ClientRequest::Player(
                            PlayerRequest::StartPlayback(playback, None),
                        ));
                    }
                } else if response.clicked() {
                    *selected_track = Some(i);
                }

                if response.secondary_clicked() {
                    if let Some(click_pos) = response.interact_pointer_pos() {
                        context_menu.open(
                            ContextTarget::Track {
                                track: track.clone(),
                                index: i,
                                playlist_id: playlist_id.cloned(),
                            },
                            click_pos,
                        );
                    }
                }

                // Row divider
                let div = egui::Rect::from_min_size(
                    row_rect.left_bottom() + egui::vec2(24.0, 0.0),
                    egui::vec2(row_rect.width() - 48.0, 1.0),
                );
                ui.painter().rect_filled(div, 0.0, theme::divider());
            }
        });

    sort_action
}

pub fn render_search(
    ui: &mut egui::Ui,
    state: &SharedState,
    client_pub: &flume::Sender<ClientRequest>,
    search_query: &mut String,
    selected_track: &mut Option<usize>,
    image_cache: &mut ImageCache,
    context_menu: &mut context_menu::ContextMenu,
) -> Action {
    let mut action = Action::None;

    theme::page_title(ui, "Search");

    ui.horizontal(|ui| {
        ui.add_space(24.0);

        let search_width = (ui.available_width() - 48.0).min(600.0);
        let search_rect = ui
            .allocate_exact_size(egui::vec2(search_width, 44.0), egui::Sense::click())
            .0;

        // Search input background
        ui.painter().rect_filled(search_rect, 22.0, theme::bg_input());

        // Search icon
        ui.painter().text(
            search_rect.left_center() + egui::vec2(16.0, 0.0),
            egui::Align2::CENTER_CENTER,
            "🔍",
            egui::FontId::proportional(16.0),
            theme::text_dim(),
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
                        .color(theme::text_muted()),
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
                        .color(theme::text_primary()),
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

                        let is_selected = *selected_track == Some(i);
                        let bg = if is_selected {
                            theme::bg_selected()
                        } else if response.hovered() {
                            theme::bg_card()
                        } else {
                            egui::Color32::TRANSPARENT
                        };
                        ui.painter().rect_filled(row_rect, 4.0, bg);

                        ui.painter().text(
                            row_rect.left_center() + egui::vec2(28.0, 0.0),
                            egui::Align2::CENTER_CENTER,
                            format!("{}", i + 1),
                            egui::FontId::monospace(12.0),
                            theme::text_muted(),
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
                                        theme::bg_active(),
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
                                theme::bg_active(),
                            );
                            ui.painter().text(
                                thumb_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "\u{266B}",
                                egui::FontId::proportional(14.0),
                                theme::text_muted(),
                            );
                        }

                        ui.painter().text(
                            row_rect.left_center() + egui::vec2(92.0, -7.0),
                            egui::Align2::LEFT_CENTER,
                            &track.name,
                            egui::FontId::proportional(14.0),
                            theme::text_primary(),
                        );
                        ui.painter().text(
                            row_rect.left_center() + egui::vec2(92.0, 10.0),
                            egui::Align2::LEFT_CENTER,
                            track.artists_info(),
                            egui::FontId::proportional(12.0),
                            theme::text_dim(),
                        );
                        ui.painter().text(
                            row_rect.right_center() + egui::vec2(-52.0, 0.0),
                            egui::Align2::RIGHT_CENTER,
                            theme::format_duration_secs(track.duration.as_secs()),
                            egui::FontId::monospace(12.0),
                            theme::text_dim(),
                        );

                        // "..." button on hover
                        let more_btn_rect = egui::Rect::from_center_size(
                            row_rect.right_center() + egui::vec2(-16.0, 0.0),
                            egui::vec2(24.0, 24.0),
                        );
                        if response.hovered() {
                            let more_resp = ui.allocate_rect(more_btn_rect, egui::Sense::click());
                            let more_bg = if more_resp.hovered() {
                                egui::Color32::from_rgb(40, 40, 40)
                            } else {
                                egui::Color32::TRANSPARENT
                            };
                            ui.painter().rect_filled(more_btn_rect, 12.0, more_bg);
                            ui.painter().text(
                                more_btn_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "\u{22EF}",
                                egui::FontId::proportional(14.0),
                                theme::text_dim(),
                            );
                            if more_resp.clicked() {
                                context_menu.open(
                                    ContextTarget::Track {
                                        track: track.clone(),
                                        index: i,
                                        playlist_id: None,
                                    },
                                    more_btn_rect.left_bottom(),
                                );
                            }
                        }

                        if response.secondary_clicked() {
                            if let Some(click_pos) = response.interact_pointer_pos() {
                                context_menu.open(
                                    ContextTarget::Track {
                                        track: track.clone(),
                                        index: i,
                                        playlist_id: None,
                                    },
                                    click_pos,
                                );
                            }
                        }

                        if response.hovered() {
                            let play_btn_rect = egui::Rect::from_center_size(
                                row_rect.left_center() + egui::vec2(28.0, 0.0),
                                egui::vec2(24.0, 24.0),
                            );
                            ui.painter().rect_filled(play_btn_rect, 12.0, theme::green());
                            ui.painter().text(
                                play_btn_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "\u{25B6}",
                                egui::FontId::proportional(10.0),
                                theme::bg_black(),
                            );
                        }

                        if response.double_clicked() {
                            *selected_track = Some(i);
                            let _ = client_pub.send(ClientRequest::Player(
                                PlayerRequest::StartPlayback(
                                    state::Playback::URIs(
                                        vec![PlayableId::Track(track.id.clone())],
                                        None,
                                    ),
                                    None,
                                ),
                            ));
                        } else if response.clicked() {
                            *selected_track = Some(i);
                        }

                        let div = egui::Rect::from_min_size(
                            row_rect.left_bottom() + egui::vec2(24.0, 0.0),
                            egui::vec2(row_rect.width() - 48.0, 1.0),
                        );
                        ui.painter().rect_filled(div, 0.0, theme::divider());
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
                        .color(theme::text_primary()),
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
                                theme::bg_hover()
                            } else {
                                theme::bg_card()
                            };
                            ui.painter().rect_filled(rect, 8.0, bg);

                            // Artist circle
                            let circle_rect = egui::Rect::from_center_size(
                                rect.center() + egui::vec2(0.0, -30.0),
                                egui::vec2(100.0, 100.0),
                            );
                            ui.painter().rect_filled(circle_rect, 50.0, theme::bg_active());
                            ui.painter().text(
                                circle_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "🎤",
                                egui::FontId::proportional(28.0),
                                theme::text_muted(),
                            );

                            ui.painter().text(
                                rect.center() + egui::vec2(0.0, 40.0),
                                egui::Align2::CENTER_CENTER,
                                &artist.name,
                                egui::FontId::proportional(13.0),
                                theme::text_primary(),
                            );
                            ui.painter().text(
                                rect.center() + egui::vec2(0.0, 58.0),
                                egui::Align2::CENTER_CENTER,
                                "Artist",
                                egui::FontId::proportional(11.0),
                                theme::text_dim(),
                            );

                            if response.secondary_clicked() {
                                if let Some(click_pos) = response.interact_pointer_pos() {
                                    context_menu.open(
                                        ContextTarget::Artist(artist.clone()),
                                        click_pos,
                                    );
                                }
                            }

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
                        .color(theme::text_primary()),
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
                            let response = search_grid_card(ui, &album.name, &sub, cover_path.as_deref(), image_cache);
                            if response.secondary_clicked() {
                                if let Some(click_pos) = response.interact_pointer_pos() {
                                    context_menu.open(
                                        ContextTarget::Album(album.clone()),
                                        click_pos,
                                    );
                                }
                            }
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
                        .color(theme::text_primary()),
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
                            let response = search_grid_card(ui, &playlist.name, &playlist.owner.0, cover_path.as_deref(), image_cache);
                            if response.secondary_clicked() {
                                if let Some(click_pos) = response.interact_pointer_pos() {
                                    context_menu.open(
                                        ContextTarget::Playlist(playlist.clone()),
                                        click_pos,
                                    );
                                }
                            }
                            ui.add_space(12.0);
                        }
                    });
                });
        }

        if !results.shows.is_empty() {
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.label(
                    egui::RichText::new("Shows")
                        .size(20.0)
                        .strong()
                        .color(theme::text_primary()),
                );
            });
            ui.add_space(8.0);

            egui::ScrollArea::horizontal()
                .id_salt("search_shows_h")
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(24.0);
                        for show in results.shows.iter() {
                            let cover_path = image_cache::show_cover_path(show);
                            if let (Some(path), Some(url)) = (&cover_path, &show.cover_url) {
                                if !path.exists() {
                                    image_cache.request_download(url, path);
                                }
                            }
                            let response = search_grid_card(ui, &show.name, &show.publisher, cover_path.as_deref(), image_cache);
                            if response.secondary_clicked() {
                                if let Some(click_pos) = response.interact_pointer_pos() {
                                    context_menu.open(
                                        ContextTarget::Show(show.clone()),
                                        click_pos,
                                    );
                                }
                            }
                            if response.clicked() {
                                action = Action::OpenShowFromSearch(show.clone());
                            }
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
                    .color(theme::text_muted()),
            );
        });
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            ui.add_space(ui.available_width() / 2.0 - 120.0);
            ui.label(
                egui::RichText::new("Search for your favorite music")
                    .size(16.0)
                    .color(theme::text_dim()),
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
) -> egui::Response {
    let width = 160.0;
    let height = 200.0;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());

    let bg = if response.hovered() {
        theme::bg_hover()
    } else {
        theme::bg_card()
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
            ui.painter().rect_filled(art_rect, theme::ART_CORNER_RADIUS, theme::bg_active());
            egui::Image::new(texture)
                .corner_radius(theme::ART_CORNER_RADIUS)
                .paint_at(ui, art_rect);
            art_drawn = true;
        }
    }

    if !art_drawn {
        ui.painter().rect_filled(art_rect, theme::ART_CORNER_RADIUS, theme::bg_active());
        if response.hovered() {
            let play_rect = egui::Rect::from_center_size(
                art_rect.center() + egui::vec2(0.0, 10.0),
                egui::vec2(40.0, 40.0),
            );
            ui.painter().rect_filled(play_rect, 20.0, theme::green());
            ui.painter().text(
                play_rect.center(),
                egui::Align2::CENTER_CENTER,
                "\u{25B6}",
                egui::FontId::proportional(16.0),
                theme::bg_black(),
            );
        } else {
            ui.painter().text(
                art_rect.center(),
                egui::Align2::CENTER_CENTER,
                "\u{266B}",
                egui::FontId::proportional(28.0),
                theme::text_muted(),
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
        ui.painter().rect_filled(play_rect, 20.0, theme::green());
        ui.painter().text(
            play_rect.center(),
            egui::Align2::CENTER_CENTER,
            "\u{25B6}",
            egui::FontId::proportional(16.0),
            theme::bg_black(),
        );
    }

    ui.painter().text(
        rect.left_top() + egui::vec2(12.0, art_size + 26.0),
        egui::Align2::LEFT_TOP,
        title,
        egui::FontId::proportional(13.0),
        theme::text_primary(),
    );
    ui.painter().text(
        rect.left_top() + egui::vec2(12.0, art_size + 46.0),
        egui::Align2::LEFT_TOP,
        subtitle,
        egui::FontId::proportional(11.0),
        theme::text_dim(),
    );

    response
}

pub fn render_browse(
    ui: &mut egui::Ui,
    state: &SharedState,
    client_pub: &flume::Sender<ClientRequest>,
    image_cache: &mut ImageCache,
    _context_menu: &mut context_menu::ContextMenu,
) -> Action {
    let mut action = Action::None;

    theme::page_title(ui, "Browse");

    let data = state.data.read();
    let categories = data.browse.categories.clone();
    drop(data);

    if categories.is_empty() {
        let _ = client_pub.send(ClientRequest::GetBrowseCategories);
        ui.add_space(80.0);
        ui.horizontal(|ui| {
            ui.add_space(ui.available_width() / 2.0 - 30.0);
            ui.spinner();
        });
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            ui.add_space(ui.available_width() / 2.0 - 80.0);
            ui.label(
                egui::RichText::new("Loading categories...")
                    .size(16.0)
                    .color(theme::text_dim()),
            );
        });
    } else {
        egui::ScrollArea::vertical()
            .id_salt("browse_scroll")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(24.0);
                });

                egui::Grid::new("categories_grid")
                    .num_columns(5)
                    .spacing([16.0, 16.0])
                    .show(ui, |ui| {
                        for (i, category) in categories.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.add_space(24.0);
                                let cover_path = image_cache::category_icon_path(category);
                                if let (Some(path), Some(url)) = (&cover_path, &category.icon_url) {
                                    if !path.exists() {
                                        image_cache.request_download(url, path);
                                    }
                                }
                                category_card(
                                    ui,
                                    &category.name,
                                    cover_path.as_deref(),
                                    image_cache,
                                    || {
                                        action = Action::OpenBrowseCategory(
                                            category.id.clone(),
                                            category.name.clone(),
                                        );
                                    },
                                );
                            });
                            if (i + 1) % 5 == 0 {
                                ui.end_row();
                            }
                        }
                    });

                ui.add_space(24.0);
            });
    }

    action
}

fn category_card(
    ui: &mut egui::Ui,
    name: &str,
    icon_path: Option<&std::path::Path>,
    image_cache: &mut ImageCache,
    on_click: impl FnOnce(),
) {
    let width = 160.0;
    let height = 180.0;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());

    let bg = if response.hovered() {
        egui::Color32::from_rgb(26, 26, 26)
    } else {
        egui::Color32::from_rgb(17, 17, 17)
    };

    ui.painter().rect_filled(rect, 8.0, bg);

    // Icon area
    let icon_size = width - 40.0;
    let icon_rect = egui::Rect::from_center_size(
        rect.center() + egui::vec2(0.0, -20.0),
        egui::vec2(icon_size, icon_size),
    );

    let mut icon_drawn = false;
    if let Some(path) = icon_path {
        if let Some(texture) = image_cache.get_texture(ui.ctx(), path) {
            ui.painter().rect_filled(icon_rect, 8.0, theme::bg_active());
            egui::Image::new(texture)
                .corner_radius(8.0)
                .paint_at(ui, icon_rect);
            icon_drawn = true;
        }
    }

    if !icon_drawn {
        ui.painter().rect_filled(icon_rect, 8.0, theme::green_dark());
        ui.painter().text(
            icon_rect.center(),
            egui::Align2::CENTER_CENTER,
            "\u{1F3B5}",
            egui::FontId::proportional(36.0),
            theme::text_primary(),
        );
    }

    // Category name
    ui.painter().text(
        rect.center() + egui::vec2(0.0, 55.0),
        egui::Align2::CENTER_CENTER,
        name,
        egui::FontId::proportional(14.0),
        theme::text_primary(),
    );

    if response.clicked() {
        on_click();
    }
}

pub fn render_browse_category_playlists(
    ui: &mut egui::Ui,
    state: &SharedState,
    category_id: &str,
    category_name: &str,
    image_cache: &mut ImageCache,
    context_menu: &mut context_menu::ContextMenu,
) -> Action {
    let mut action = Action::None;

    // Back button
    ui.add_space(16.0);
    ui.horizontal(|ui| {
        ui.add_space(24.0);
        let back_rect = ui
            .allocate_exact_size(egui::vec2(80.0, 32.0), egui::Sense::click());
        let bg = if back_rect.1.hovered() {
            theme::bg_hover()
        } else {
            theme::bg_card()
        };
        ui.painter().rect_filled(back_rect.0, 6.0, bg);
        ui.painter().text(
            back_rect.0.center(),
            egui::Align2::CENTER_CENTER,
            "\u{2190} Back",
            egui::FontId::proportional(13.0),
            theme::text_primary(),
        );
        if back_rect.1.clicked() {
            action = Action::BackToBrowse;
        }
    });

    theme::page_title(ui, category_name);

    let data = state.data.read();
    let playlists = data
        .browse
        .category_playlists
        .get(category_id)
        .cloned()
        .unwrap_or_default();
    drop(data);

    if playlists.is_empty() {
        ui.add_space(60.0);
        ui.horizontal(|ui| {
            ui.add_space(ui.available_width() / 2.0 - 30.0);
            ui.spinner();
        });
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            ui.add_space(ui.available_width() / 2.0 - 80.0);
            ui.label(
                egui::RichText::new("Loading playlists...")
                    .size(16.0)
                    .color(theme::text_dim()),
            );
        });
    } else {
        egui::ScrollArea::vertical()
            .id_salt("category_playlists_scroll")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(24.0);
                });

                egui::Grid::new("category_playlists_grid")
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
                                let response = grid_card(
                                    ui,
                                    &playlist.name,
                                    &playlist.owner.0,
                                    cover_path.as_deref(),
                                    image_cache,
                                    || {
                                        action = Action::OpenBrowsePlaylist(playlist.clone());
                                    },
                                );
                                if response.secondary_clicked() {
                                    if let Some(click_pos) = response.interact_pointer_pos() {
                                        context_menu.open(
                                            ContextTarget::Playlist(playlist.clone()),
                                            click_pos,
                                        );
                                    }
                                }
                            });
                            if (i + 1) % 4 == 0 {
                                ui.end_row();
                            }
                        }
                    });

                ui.add_space(24.0);
            });
    }

    action
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
                        .color(theme::text_dim()),
                );
            });
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.add_space(24.0);
                let width = ui.available_width() - 48.0;
                let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 64.0), egui::Sense::hover());

                ui.painter().rect_filled(rect, 8.0, theme::bg_card());

                // Green left accent
                ui.painter().rect_filled(
                    egui::Rect::from_min_size(rect.min, egui::vec2(3.0, rect.height())),
                    1.5,
                    theme::green(),
                );

                match item {
                    rspotify::model::PlayableItem::Track(track) => {
                        ui.painter().text(
                            rect.left_center() + egui::vec2(24.0, -8.0),
                            egui::Align2::LEFT_CENTER,
                            &track.name,
                            egui::FontId::proportional(15.0),
                            theme::green(),
                        );
                        let artists: Vec<_> = track.artists.iter().map(|a| a.name.as_str()).collect();
                        ui.painter().text(
                            rect.left_center() + egui::vec2(24.0, 10.0),
                            egui::Align2::LEFT_CENTER,
                            artists.join(", "),
                            egui::FontId::proportional(12.0),
                            theme::text_dim(),
                        );
                    }
                    rspotify::model::PlayableItem::Episode(ep) => {
                        ui.painter().text(
                            rect.left_center() + egui::vec2(24.0, -8.0),
                            egui::Align2::LEFT_CENTER,
                            &ep.name,
                            egui::FontId::proportional(15.0),
                            theme::green(),
                        );
                        ui.painter().text(
                            rect.left_center() + egui::vec2(24.0, 10.0),
                            egui::Align2::LEFT_CENTER,
                            &ep.show.name,
                            egui::FontId::proportional(12.0),
                            theme::text_dim(),
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
                    .color(theme::text_dim()),
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
                        theme::bg_card()
                    } else {
                        egui::Color32::TRANSPARENT
                    };
                    ui.painter().rect_filled(row_rect, 4.0, bg);

                    ui.painter().text(
                        row_rect.left_center() + egui::vec2(40.0, 0.0),
                        egui::Align2::CENTER_CENTER,
                        format!("{}", i + 1),
                        egui::FontId::monospace(12.0),
                        theme::text_muted(),
                    );

                    match item {
                        rspotify::model::PlayableItem::Track(track) => {
                            ui.painter().text(
                                row_rect.left_center() + egui::vec2(80.0, -7.0),
                                egui::Align2::LEFT_CENTER,
                                &track.name,
                                egui::FontId::proportional(14.0),
                                theme::text_primary(),
                            );
                            let artists: Vec<_> = track.artists.iter().map(|a| a.name.as_str()).collect();
                            ui.painter().text(
                                row_rect.left_center() + egui::vec2(80.0, 10.0),
                                egui::Align2::LEFT_CENTER,
                                artists.join(", "),
                                egui::FontId::proportional(12.0),
                                theme::text_dim(),
                            );
                            let dur = theme::format_duration_secs(track.duration.num_seconds() as u64);
                            ui.painter().text(
                                row_rect.right_center() + egui::vec2(-24.0, 0.0),
                                egui::Align2::RIGHT_CENTER,
                                &dur,
                                egui::FontId::monospace(12.0),
                                theme::text_dim(),
                            );
                        }
                        rspotify::model::PlayableItem::Episode(ep) => {
                            ui.painter().text(
                                row_rect.left_center() + egui::vec2(80.0, -7.0),
                                egui::Align2::LEFT_CENTER,
                                &ep.name,
                                egui::FontId::proportional(14.0),
                                theme::text_primary(),
                            );
                            ui.painter().text(
                                row_rect.left_center() + egui::vec2(80.0, 10.0),
                                egui::Align2::LEFT_CENTER,
                                &ep.show.name,
                                egui::FontId::proportional(12.0),
                                theme::text_dim(),
                            );
                        }
                        _ => {}
                    }

                    let div = egui::Rect::from_min_size(
                        row_rect.left_bottom() + egui::vec2(24.0, 0.0),
                        egui::vec2(row_rect.width() - 48.0, 1.0),
                    );
                    ui.painter().rect_filled(div, 0.0, theme::divider());
                }
            });
    } else {
        ui.add_space(40.0);
        ui.horizontal(|ui| {
            ui.add_space(ui.available_width() / 2.0 - 60.0);
            ui.label(
                egui::RichText::new("Queue is empty")
                    .size(16.0)
                    .color(theme::text_dim()),
            );
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    General,
    Playback,
    Appearance,
    Keybindings,
    About,
}

pub enum SettingsAction {
    None,
    Save,
    Reset,
}

impl SettingsTab {
    fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Playback => "Playback",
            Self::Appearance => "Appearance",
            Self::Keybindings => "Keybindings",
            Self::About => "About",
        }
    }
    fn icon(self) -> &'static str {
        match self {
            Self::General => "⚙",
            Self::Playback => "▶",
            Self::Appearance => "🎨",
            Self::Keybindings => "⌨",
            Self::About => "ℹ",
        }
    }
    fn all() -> &'static [SettingsTab] {
        &[
            Self::General,
            Self::Playback,
            Self::Appearance,
            Self::Keybindings,
            Self::About,
        ]
    }
}

fn settings_toggle(ui: &mut egui::Ui, label: &str, value: &mut bool) -> bool {
    let toggle_size = egui::vec2(36.0, 20.0);
    let (toggle_rect, toggle_resp) = ui.allocate_exact_size(toggle_size, egui::Sense::click());
    let toggle_bg = if *value {
        theme::green()
    } else {
        theme::bg_active()
    };
    ui.painter()
        .rect_filled(toggle_rect, egui::CornerRadius::same(10), toggle_bg);
    let knob_x = if *value {
        toggle_rect.right() - 10.0
    } else {
        toggle_rect.left() + 10.0
    };
    ui.painter().circle_filled(
        egui::pos2(knob_x, toggle_rect.center().y),
        7.0,
        egui::Color32::from_rgb(255, 255, 255),
    );
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new(label)
            .size(13.0)
            .color(theme::text_secondary()),
    );
    toggle_resp.clicked()
}

fn settings_text_field(ui: &mut egui::Ui, label: &str, value: &mut String, hint: &str) {
    ui.label(
        egui::RichText::new(label)
            .size(12.0)
            .color(theme::text_dim()),
    );
    ui.add_space(4.0);
    ui.add(
        egui::TextEdit::singleline(value)
            .desired_width(f32::INFINITY)
            .hint_text(hint)
            .font(egui::FontId::proportional(13.0))
            .margin(egui::Margin::symmetric(10, 8))
            .background_color(theme::bg_input()),
    );
    ui.add_space(12.0);
}

fn settings_number_field_u16(ui: &mut egui::Ui, label: &str, value: &mut u16) {
    ui.label(
        egui::RichText::new(label)
            .size(12.0)
            .color(theme::text_dim()),
    );
    ui.add_space(4.0);
    let mut s = value.to_string();
    let resp = ui.add(
        egui::TextEdit::singleline(&mut s)
            .desired_width(120.0)
            .font(egui::FontId::proportional(13.0))
            .margin(egui::Margin::symmetric(10, 8))
            .background_color(theme::bg_input()),
    );
    if resp.changed() {
        if let Ok(n) = s.parse::<u16>() {
            *value = n;
        }
    }
    ui.add_space(12.0);
}

fn settings_number_field_u8(ui: &mut egui::Ui, label: &str, value: &mut u8, min: u8, max: u8) {
    ui.label(
        egui::RichText::new(label)
            .size(12.0)
            .color(theme::text_dim()),
    );
    ui.add_space(4.0);
    let mut s = value.to_string();
    let resp = ui.add(
        egui::TextEdit::singleline(&mut s)
            .desired_width(120.0)
            .font(egui::FontId::proportional(13.0))
            .margin(egui::Margin::symmetric(10, 8))
            .background_color(theme::bg_input()),
    );
    if resp.changed() {
        if let Ok(n) = s.parse::<u8>() {
            *value = n.clamp(min, max);
        }
    }
    ui.add_space(12.0);
}

fn settings_number_field_usize(ui: &mut egui::Ui, label: &str, value: &mut usize, min: usize, max: usize) {
    ui.label(
        egui::RichText::new(label)
            .size(12.0)
            .color(theme::text_dim()),
    );
    ui.add_space(4.0);
    let mut s = value.to_string();
    let resp = ui.add(
        egui::TextEdit::singleline(&mut s)
            .desired_width(120.0)
            .font(egui::FontId::proportional(13.0))
            .margin(egui::Margin::symmetric(10, 8))
            .background_color(theme::bg_input()),
    );
    if resp.changed() {
        if let Ok(n) = s.parse::<usize>() {
            *value = n.clamp(min, max);
        }
    }
    ui.add_space(12.0);
}

fn settings_slider_u8(ui: &mut egui::Ui, label: &str, value: &mut u8, min: u8, max: u8) {
    ui.label(
        egui::RichText::new(label)
            .size(12.0)
            .color(theme::text_dim()),
    );
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.add(
            egui::Slider::new(value, min..=max)
                .fixed_decimals(0)
                .custom_formatter(|v, _| format!("{}", v as u8)),
        );
        ui.label(
            egui::RichText::new(format!("{}", value))
                .size(13.0)
                .monospace()
                .color(theme::text_primary()),
        );
    });
    ui.add_space(12.0);
}

fn settings_slider_u16(ui: &mut egui::Ui, label: &str, value: &mut u16, min: u16, max: u16) {
    ui.label(
        egui::RichText::new(label)
            .size(12.0)
            .color(theme::text_dim()),
    );
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.add(
            egui::Slider::new(value, min..=max)
                .fixed_decimals(0),
        );
        ui.label(
            egui::RichText::new(format!("{}", value))
                .size(13.0)
                .monospace()
                .color(theme::text_primary()),
        );
    });
    ui.add_space(12.0);
}

pub fn render_settings(
    ui: &mut egui::Ui,
    current_tab: &mut SettingsTab,
    config: &mut crate::config::AppConfig,
    dirty: &mut bool,
    keybinding_search: &mut String,
    editing_keybinding: &mut Option<usize>,
    keybindings: &[crate::key::CommandBinding],
    current_theme_name: &str,
    _client_pub: &flume::Sender<ClientRequest>,
) -> SettingsAction {
    let mut action = SettingsAction::None;

    theme::page_title(ui, "Settings");

    // Tab bar
    ui.horizontal(|ui| {
        ui.add_space(24.0);
        for tab in SettingsTab::all() {
            let is_selected = *current_tab == *tab;
            let label = format!("{} {}", tab.icon(), tab.label());
            let text_color = if is_selected {
                theme::bg_black()
            } else {
                theme::text_secondary()
            };
            let btn_rect = ui
                .allocate_exact_size(egui::vec2(label.len() as f32 * 9.0 + 24.0, 32.0), egui::Sense::click())
                .0;
            let resp = ui.allocate_rect(btn_rect, egui::Sense::click());
            let bg = if is_selected {
                theme::green()
            } else if resp.hovered() {
                theme::bg_hover()
            } else {
                theme::bg_card()
            };
            ui.painter()
                .rect_filled(btn_rect, egui::CornerRadius::same(6), bg);
            ui.painter().text(
                btn_rect.center(),
                egui::Align2::CENTER_CENTER,
                &label,
                egui::FontId::proportional(13.0),
                text_color,
            );
            if resp.clicked() && *current_tab != *tab {
                *current_tab = *tab;
            }
            ui.add_space(4.0);
        }

        // Save / Reset buttons on the right
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(24.0);
            if *dirty {
                // Reset button
                let reset_rect = ui
                    .allocate_exact_size(egui::vec2(100.0, 32.0), egui::Sense::click())
                    .0;
                let reset_resp = ui.allocate_rect(reset_rect, egui::Sense::click());
                let reset_bg = if reset_resp.hovered() {
                    theme::bg_hover()
                } else {
                    theme::bg_card()
                };
                ui.painter()
                    .rect_filled(reset_rect, egui::CornerRadius::same(6), reset_bg);
                ui.painter().rect_stroke(
                    reset_rect,
                    egui::CornerRadius::same(6),
                    egui::Stroke::new(1.0, theme::divider()),
                    egui::StrokeKind::Outside,
                );
                ui.painter().text(
                    reset_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Reset",
                    egui::FontId::proportional(13.0),
                    theme::text_primary(),
                );
                if reset_resp.clicked() {
                    action = SettingsAction::Reset;
                }
                ui.add_space(8.0);

                // Save button
                let save_rect = ui
                    .allocate_exact_size(egui::vec2(100.0, 32.0), egui::Sense::click())
                    .0;
                let save_resp = ui.allocate_rect(save_rect, egui::Sense::click());
                let save_bg = if save_resp.hovered() {
                    theme::green_hover()
                } else {
                    theme::green()
                };
                ui.painter()
                    .rect_filled(save_rect, egui::CornerRadius::same(6), save_bg);
                ui.painter().text(
                    save_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Save",
                    egui::FontId::proportional(13.0),
                    theme::bg_black(),
                );
                if save_resp.clicked() {
                    action = SettingsAction::Save;
                }
            } else {
                ui.label(
                    egui::RichText::new("No changes")
                        .size(12.0)
                        .color(theme::text_dim()),
                );
            }
        });
    });

    ui.add_space(16.0);

    // Divider
    let div_rect = ui.allocate_space(egui::vec2(ui.available_width(), 1.0)).1;
    ui.painter().rect_filled(div_rect, 0.0, theme::divider());

    ui.add_space(8.0);

    match current_tab {
        SettingsTab::General => render_settings_general(ui, config, dirty),
        SettingsTab::Playback => render_settings_playback(ui, config, dirty),
        SettingsTab::Appearance => render_settings_appearance(ui, config, dirty, current_theme_name),
        SettingsTab::Keybindings => {
            render_settings_keybindings(ui, keybinding_search, editing_keybinding, keybindings)
        }
        SettingsTab::About => render_settings_about(ui),
    }

    action
}

fn render_settings_general(
    ui: &mut egui::Ui,
    config: &mut crate::config::AppConfig,
    dirty: &mut bool,
) {
    egui::ScrollArea::vertical()
        .id_salt("settings_general")
        .show(ui, |ui| {
            ui.add_space(8.0);

            // Connection section
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.label(
                    egui::RichText::new("Connection")
                        .size(16.0)
                        .strong()
                        .color(theme::text_primary()),
                );
            });
            ui.add_space(12.0);

            ui.horizontal(|ui| {
                ui.add_space(24.0);
                let card_width = ui.available_width() - 48.0;
                theme::card(ui, |ui| {
                    ui.set_width(card_width - 32.0);

                    let old_client_id = config.client_id.clone().unwrap_or_default();
                    let mut client_id_str = old_client_id.clone();
                    settings_text_field(ui, "Client ID", &mut client_id_str, "Spotify app client ID");
                    if client_id_str != old_client_id {
                        config.client_id = Some(client_id_str);
                    }

                    let old_port = config.client_port;
                    settings_number_field_u16(ui, "Client Port", &mut config.client_port);
                    if config.client_port != old_port {
                        *dirty = true;
                    }

                    let old_device = config.default_device.clone();
                    settings_text_field(
                        ui,
                        "Default Device",
                        &mut config.default_device,
                        "Device name for Spotify connect",
                    );
                    if config.default_device != old_device {
                        *dirty = true;
                    }
                });
            });

            ui.add_space(20.0);

            // Device section
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.label(
                    egui::RichText::new("Device")
                        .size(16.0)
                        .strong()
                        .color(theme::text_primary()),
                );
            });
            ui.add_space(12.0);

            ui.horizontal(|ui| {
                ui.add_space(24.0);
                let card_width = ui.available_width() - 48.0;
                theme::card(ui, |ui| {
                    ui.set_width(card_width - 32.0);

                    let old_name = config.device.name.clone();
                    settings_text_field(ui, "Device Name", &mut config.device.name, "spotify-player");
                    if config.device.name != old_name {
                        *dirty = true;
                    }

                    // Bitrate dropdown
                    ui.label(
                        egui::RichText::new("Bitrate")
                            .size(12.0)
                            .color(theme::text_dim()),
                    );
                    ui.add_space(4.0);
                    let bitrate_options = [96u16, 160, 320];
                    let current_bitrate = config.device.bitrate;
                    let selected_label = format!("{} kbps", current_bitrate);
                    egui::ComboBox::from_id_salt("bitrate_select")
                        .selected_text(&selected_label)
                        .width(120.0)
                        .show_ui(ui, |ui| {
                            for &opt in &bitrate_options {
                                let label = format!("{} kbps", opt);
                                if ui
                                    .selectable_value(&mut config.device.bitrate, opt, &label)
                                    .changed()
                                {
                                    *dirty = true;
                                }
                            }
                        });
                    ui.add_space(12.0);

                    if settings_toggle(ui, "Autoplay", &mut config.device.autoplay) {
                        *dirty = true;
                    }
                    ui.add_space(4.0);

                    if settings_toggle(ui, "Normalization", &mut config.device.normalization) {
                        *dirty = true;
                    }
                    ui.add_space(4.0);
                });
            });

            ui.add_space(20.0);

            // Behavior section
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.label(
                    egui::RichText::new("Behavior")
                        .size(16.0)
                        .strong()
                        .color(theme::text_primary()),
                );
            });
            ui.add_space(12.0);

            ui.horizontal(|ui| {
                ui.add_space(24.0);
                let card_width = ui.available_width() - 48.0;
                theme::card(ui, |ui| {
                    ui.set_width(card_width - 32.0);

                    if settings_toggle(ui, "Enable media control", &mut config.enable_media_control) {
                        *dirty = true;
                    }
                    ui.add_space(4.0);

                    if settings_toggle(ui, "Enable cover image cache", &mut config.enable_cover_image_cache) {
                        *dirty = true;
                    }
                    ui.add_space(4.0);

                    let old_limit = config.tracks_playback_limit;
                    settings_number_field_usize(
                        ui,
                        "Tracks playback limit",
                        &mut config.tracks_playback_limit,
                        1,
                        50,
                    );
                    if config.tracks_playback_limit != old_limit {
                        *dirty = true;
                    }

                    let old_seek = config.seek_duration_secs;
                    settings_number_field_u16(ui, "Seek duration (seconds)", &mut config.seek_duration_secs);
                    if config.seek_duration_secs != old_seek {
                        *dirty = true;
                    }

                    let old_step = config.volume_scroll_step;
                    settings_number_field_u8(ui, "Volume scroll step", &mut config.volume_scroll_step, 1, 20);
                    if config.volume_scroll_step != old_step {
                        *dirty = true;
                    }
                });
            });

            ui.add_space(24.0);
        });
}

fn render_settings_playback(
    ui: &mut egui::Ui,
    config: &mut crate::config::AppConfig,
    dirty: &mut bool,
) {
    egui::ScrollArea::vertical()
        .id_salt("settings_playback")
        .show(ui, |ui| {
            ui.add_space(8.0);

            // Volume & Device
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.label(
                    egui::RichText::new("Volume & Device")
                        .size(16.0)
                        .strong()
                        .color(theme::text_primary()),
                );
            });
            ui.add_space(12.0);

            ui.horizontal(|ui| {
                ui.add_space(24.0);
                let card_width = ui.available_width() - 48.0;
                theme::card(ui, |ui| {
                    ui.set_width(card_width - 32.0);

                    settings_slider_u8(ui, "Default Volume", &mut config.device.volume, 0, 100);

                    if settings_toggle(ui, "Audio cache", &mut config.device.audio_cache) {
                        *dirty = true;
                    }
                    ui.add_space(4.0);
                });
            });

            ui.add_space(20.0);

            // Format templates
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.label(
                    egui::RichText::new("Format Templates")
                        .size(16.0)
                        .strong()
                        .color(theme::text_primary()),
                );
            });
            ui.add_space(12.0);

            ui.horizontal(|ui| {
                ui.add_space(24.0);
                let card_width = ui.available_width() - 48.0;
                theme::card(ui, |ui| {
                    ui.set_width(card_width - 32.0);

                    let old_format = config.playback_format.clone();
                    settings_text_field(
                        ui,
                        "Playback format",
                        &mut config.playback_format,
                        "{status} {track} • {artists}",
                    );
                    if config.playback_format != old_format {
                        *dirty = true;
                    }

                    // Preview
                    ui.label(
                        egui::RichText::new("Preview:")
                            .size(11.0)
                            .color(theme::text_dim()),
                    );
                    let preview = config
                        .playback_format
                        .replace("{status}", "▶")
                        .replace("{track}", "Bohemian Rhapsody")
                        .replace("{artists}", "Queen")
                        .replace("{album}", "A Night at the Opera")
                        .replace("{liked}", "♥")
                        .replace("{genres}", "rock, classic")
                        .replace("{metadata}", "vol:50%");
                    ui.label(
                        egui::RichText::new(&preview)
                            .size(12.0)
                            .color(theme::text_secondary()),
                    );
                    ui.add_space(12.0);
                });
            });

            ui.add_space(24.0);
        });
}

fn render_settings_appearance(
    ui: &mut egui::Ui,
    config: &mut crate::config::AppConfig,
    dirty: &mut bool,
    _current_theme_name: &str,
) {
    egui::ScrollArea::vertical()
        .id_salt("settings_appearance")
        .show(ui, |ui| {
            ui.add_space(8.0);

            // Theme
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.label(
                    egui::RichText::new("Theme")
                        .size(16.0)
                        .strong()
                        .color(theme::text_primary()),
                );
            });
            ui.add_space(12.0);

            ui.horizontal(|ui| {
                ui.add_space(24.0);
                let card_width = ui.available_width() - 48.0;
                theme::card(ui, |ui| {
                    ui.set_width(card_width - 32.0);

                    ui.label(
                        egui::RichText::new("Current theme")
                            .size(12.0)
                            .color(theme::text_dim()),
                    );
                    ui.add_space(4.0);

                    let built_in = theme::built_in_themes();
                    let theme_config = crate::config::get_config();
                    let custom_themes: Vec<_> = theme_config.theme_config.themes.clone();

                    let mut all_theme_names: Vec<String> = built_in
                        .iter()
                        .map(|t| t.name.to_string())
                        .collect();
                    for ct in &custom_themes {
                        all_theme_names.push(ct.name.clone());
                    }

                    let current = config.theme.clone();
                    egui::ComboBox::from_id_salt("theme_select")
                        .selected_text(&current)
                        .width(200.0)
                        .show_ui(ui, |ui| {
                            for name in &all_theme_names {
                                if ui
                                    .selectable_value(&mut config.theme, name.clone(), name)
                                    .changed()
                                {
                                    *dirty = true;
                                }
                            }
                        });

                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("Use T or Ctrl+Shift+P → 'Switch Theme' for live preview")
                            .size(11.0)
                            .color(theme::text_dim()),
                    );
                });
            });

            ui.add_space(20.0);

            // Layout
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.label(
                    egui::RichText::new("Library Layout")
                        .size(16.0)
                        .strong()
                        .color(theme::text_primary()),
                );
            });
            ui.add_space(12.0);

            ui.horizontal(|ui| {
                ui.add_space(24.0);
                let card_width = ui.available_width() - 48.0;
                theme::card(ui, |ui| {
                    ui.set_width(card_width - 32.0);

                    settings_slider_u16(
                        ui,
                        "Playlist section width (%)",
                        &mut config.layout.library.playlist_percent,
                        10,
                        80,
                    );

                    settings_slider_u16(
                        ui,
                        "Album section width (%)",
                        &mut config.layout.library.album_percent,
                        10,
                        80,
                    );
                });
            });

            ui.add_space(24.0);
        });
}

fn render_settings_keybindings(
    ui: &mut egui::Ui,
    keybinding_search: &mut String,
    editing_keybinding: &mut Option<usize>,
    keybindings: &[crate::key::CommandBinding],
) {
    // Search bar
    ui.horizontal(|ui| {
        ui.add_space(24.0);
        let search_width = (ui.available_width() - 48.0).min(400.0);
        let search_rect = ui
            .allocate_exact_size(egui::vec2(search_width, 36.0), egui::Sense::click())
            .0;
        ui.painter()
            .rect_filled(search_rect, 18.0, theme::bg_input());
        let text_rect = egui::Rect::from_min_size(
            search_rect.min + egui::vec2(12.0, 4.0),
            egui::vec2(search_rect.width() - 24.0, search_rect.height() - 8.0),
        );
        ui.put(
            text_rect,
            egui::TextEdit::singleline(keybinding_search)
                .hint_text(
                    egui::RichText::new("Search keybindings...").color(theme::text_muted()),
                )
                .frame(false)
                .font(egui::FontId::proportional(13.0)),
        );
    });
    ui.add_space(12.0);

    let filter = keybinding_search.to_lowercase();

    // Group by category
    let category_order = [
        crate::key::CommandCategory::Navigation,
        crate::key::CommandCategory::Playback,
        crate::key::CommandCategory::Sorting,
        crate::key::CommandCategory::Actions,
        crate::key::CommandCategory::Pages,
        crate::key::CommandCategory::Other,
    ];

    egui::ScrollArea::vertical()
        .id_salt("settings_keybindings")
        .show(ui, |ui| {
            let mut binding_idx = 0usize;

            for cat in &category_order {
                let items: Vec<_> = keybindings
                    .iter()
                    .filter(|b| b.category == *cat)
                    .filter(|b| {
                        filter.is_empty()
                            || b.description.to_lowercase().contains(&filter)
                            || b.command.0.to_lowercase().contains(&filter)
                            || b.keybindings.iter().any(|kb| {
                                kb.display_string().to_lowercase().contains(&filter)
                            })
                    })
                    .collect();

                if items.is_empty() {
                    binding_idx += keybindings.iter().filter(|b| b.category == *cat).count();
                    continue;
                }

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.add_space(24.0);
                    ui.label(
                        egui::RichText::new(cat.display_name())
                            .size(14.0)
                            .strong()
                            .color(theme::green()),
                    );
                });
                ui.add_space(6.0);

                for binding in &items {
                    let is_editing = *editing_keybinding == Some(binding_idx);

                    ui.horizontal(|ui| {
                        ui.add_space(24.0);
                        let row_width = ui.available_width() - 48.0;
                        let row_height = 36.0;
                        let (row_rect, _) =
                            ui.allocate_exact_size(egui::vec2(row_width, row_height), egui::Sense::hover());

                        let bg = if is_editing {
                            theme::bg_active()
                        } else {
                            theme::bg_card()
                        };
                        ui.painter()
                            .rect_filled(row_rect, 4.0, bg);

                        // Command description
                        ui.painter().text(
                            row_rect.left_center() + egui::vec2(12.0, 0.0),
                            egui::Align2::LEFT_CENTER,
                            binding.description,
                            egui::FontId::proportional(13.0),
                            theme::text_primary(),
                        );

                        // Keybindings display
                        // Key badges display
                        let badge_x = row_rect.right() - 200.0;
                        if is_editing {
                            // Show "Press a key..." prompt
                            ui.painter().text(
                                egui::pos2(badge_x, row_rect.center().y),
                                egui::Align2::LEFT_CENTER,
                                "Press a key... (Esc to cancel)",
                                egui::FontId::proportional(12.0),
                                theme::green(),
                            );
                        } else {
                            // Render key badges
                            let mut x = badge_x;
                            for kb in binding.keybindings.iter() {
                                let display = kb.display_string();
                                let badge_w = (display.len() as f32 * 8.0) + 16.0;
                                let badge_rect = egui::Rect::from_min_size(
                                    egui::pos2(x, row_rect.center().y - 10.0),
                                    egui::vec2(badge_w, 20.0),
                                );
                                ui.painter().rect_filled(
                                    badge_rect,
                                    egui::CornerRadius::same(4),
                                    theme::bg_input(),
                                );
                                ui.painter().rect_stroke(
                                    badge_rect,
                                    egui::CornerRadius::same(4),
                                    egui::Stroke::new(1.0, theme::divider()),
                                    egui::StrokeKind::Outside,
                                );
                                ui.painter().text(
                                    badge_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    &display,
                                    egui::FontId::monospace(11.0),
                                    theme::text_secondary(),
                                );
                                x += badge_w + 4.0;
                            }

                            // Edit button
                            let edit_rect = egui::Rect::from_center_size(
                                egui::pos2(row_rect.right() - 16.0, row_rect.center().y),
                                egui::vec2(24.0, 24.0),
                            );
                            let edit_resp = ui.allocate_rect(edit_rect, egui::Sense::click());
                            let edit_bg = if edit_resp.hovered() {
                                theme::bg_hover()
                            } else {
                                egui::Color32::TRANSPARENT
                            };
                            ui.painter().rect_filled(edit_rect, 4.0, edit_bg);
                            ui.painter().text(
                                edit_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "✎",
                                egui::FontId::proportional(12.0),
                                if edit_resp.hovered() {
                                    theme::text_primary()
                                } else {
                                    theme::text_dim()
                                },
                            );
                            if edit_resp.clicked() {
                                *editing_keybinding = Some(binding_idx);
                            }
                        }
                    });
                    ui.add_space(2.0);
                    binding_idx += 1;
                }

                // Skip remaining bindings for this category that weren't filtered
                let total_in_cat = keybindings.iter().filter(|b| b.category == *cat).count();
                binding_idx += total_in_cat - items.len();
            }

            // Handle keybinding editing (capture key presses)
            if editing_keybinding.is_some() {
                // Check for escape to cancel
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    *editing_keybinding = None;
                }
            }
        });
}

fn render_settings_about(ui: &mut egui::Ui) {
    egui::ScrollArea::vertical()
        .id_salt("settings_about")
        .show(ui, |ui| {
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.add_space(24.0);
                let card_width = ui.available_width() - 48.0;
                theme::card(ui, |ui| {
                    ui.set_width(card_width - 32.0);

                    // App name and version
                    ui.label(
                        egui::RichText::new("spotify-player-gui")
                            .size(20.0)
                            .strong()
                            .color(theme::text_primary()),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("Version 0.1.0")
                            .size(13.0)
                            .color(theme::text_dim()),
                    );
                    ui.add_space(16.0);

                    theme::divider_line(ui);

                    ui.label(
                        egui::RichText::new("A native macOS Spotify player with a dark GUI, built in Rust")
                            .size(13.0)
                            .color(theme::text_secondary()),
                    );
                    ui.add_space(16.0);

                    // Links
                    ui.label(
                        egui::RichText::new("Configuration")
                            .size(14.0)
                            .strong()
                            .color(theme::text_primary()),
                    );
                    ui.add_space(8.0);

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
                                    .size(12.0)
                                    .color(theme::green()),
                            );
                            ui.label(
                                egui::RichText::new(format!("— {desc}"))
                                    .size(12.0)
                                    .color(theme::text_dim()),
                            );
                        });
                        ui.add_space(4.0);
                    }

                    ui.add_space(16.0);

                    // Tech stack
                    ui.label(
                        egui::RichText::new("Built with")
                            .size(14.0)
                            .strong()
                            .color(theme::text_primary()),
                    );
                    ui.add_space(8.0);

                    let tech = [
                        ("Rust", "Systems language"),
                        ("egui / eframe", "Immediate mode GUI"),
                        ("librespot", "Spotify client library"),
                        ("rspotify", "Spotify Web API client"),
                    ];

                    for (name, desc) in &tech {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(*name)
                                    .monospace()
                                    .size(12.0)
                                    .color(theme::green()),
                            );
                            ui.label(
                                egui::RichText::new(format!("— {desc}"))
                                    .size(12.0)
                                    .color(theme::text_dim()),
                            );
                        });
                        ui.add_space(4.0);
                    }
                });
            });

            ui.add_space(24.0);
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
    ui.painter().rect_filled(full_rect, 0.0, theme::lyrics_bg());

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
                .color(theme::text_primary()),
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
                    .color(theme::text_primary()),
            );
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add_space(32.0);
            ui.label(
                egui::RichText::new(&artists_str)
                    .size(14.0)
                    .color(theme::text_dim()),
            );
        });
    }
    ui.add_space(20.0);

    // Divider
    let div_rect = ui.allocate_space(egui::vec2(ui.available_width(), 1.0)).1;
    ui.painter().rect_filled(div_rect, 0.0, theme::divider());
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
                            (theme::lyrics_played(), 16.0, false)
                        } else if i == current_idx {
                            (theme::lyrics_current(), 20.0, true)
                        } else {
                            (theme::lyrics_upcoming(), 16.0, false)
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
                            .color(theme::text_dim()),
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
                            .color(theme::text_dim()),
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
                .color(theme::text_muted()),
        );
    });
    ui.add_space(16.0);
    ui.horizontal(|ui| {
        ui.add_space(ui.available_width() / 2.0 - 90.0);
        ui.label(
            egui::RichText::new("No lyrics available")
                .size(16.0)
                .color(theme::text_dim()),
        );
    });
}

pub fn render_artist(
    ui: &mut egui::Ui,
    state: &SharedState,
    client_pub: &flume::Sender<ClientRequest>,
    artist_context: &Option<crate::state::Context>,
    image_cache: &mut ImageCache,
    context_menu: &mut context_menu::ContextMenu,
) -> Action {
    let mut action = Action::None;

    let ctx = match artist_context {
        Some(crate::state::Context::Artist { artist, top_tracks, albums, related_artists }) => {
            (artist, top_tracks, albums, related_artists)
        }
        _ => {
            ui.add_space(80.0);
            ui.horizontal(|ui| {
                ui.add_space(ui.available_width() / 2.0 - 30.0);
                ui.spinner();
            });
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.add_space(ui.available_width() / 2.0 - 80.0);
                ui.label(
                    egui::RichText::new("Loading artist...")
                        .size(14.0)
                        .color(theme::text_dim()),
                );
            });
            return Action::None;
        }
    };

    let (artist, top_tracks, albums, related_artists) = ctx;

    // === Header Section ===
    ui.add_space(24.0);
    ui.horizontal(|ui| {
        ui.add_space(24.0);
        let header_height = 160.0;
        let header_width = ui.available_width() - 48.0;
        let (header_rect, _) = ui.allocate_exact_size(
            egui::vec2(header_width, header_height),
            egui::Sense::hover(),
        );

        // Artist image (circular)
        let img_size = 120.0;
        let img_rect = egui::Rect::from_min_size(
            header_rect.min + egui::vec2(0.0, (header_height - img_size) / 2.0),
            egui::vec2(img_size, img_size),
        );

        let mut img_drawn = false;
        let cover_path = image_cache::artist_cover_path(artist);
        if let (Some(path), Some(url)) = (&cover_path, &artist.image_url) {
            if !path.exists() {
                image_cache.request_download(url, path);
            }
            if let Some(texture) = image_cache.get_texture(ui.ctx(), path) {
                ui.painter().rect_filled(img_rect, img_size / 2.0, theme::bg_active());
                egui::Image::new(texture)
                    .corner_radius(img_size / 2.0)
                    .paint_at(ui, img_rect);
                img_drawn = true;
            }
        }
        if !img_drawn {
            ui.painter().rect_filled(img_rect, img_size / 2.0, theme::bg_active());
            ui.painter().text(
                img_rect.center(),
                egui::Align2::CENTER_CENTER,
                "🎤",
                egui::FontId::proportional(40.0),
                theme::text_muted(),
            );
        }

        // Artist info
        let text_x = img_rect.right() + 24.0;
        ui.painter().text(
            egui::pos2(text_x, header_rect.top() + 20.0),
            egui::Align2::LEFT_TOP,
            &artist.name,
            egui::FontId::proportional(32.0),
            theme::text_primary(),
        );

        let followers_str = format_followers(artist.followers);
        ui.painter().text(
            egui::pos2(text_x, header_rect.top() + 62.0),
            egui::Align2::LEFT_TOP,
            &followers_str,
            egui::FontId::proportional(13.0),
            theme::text_dim(),
        );

        if !artist.genres.is_empty() {
            let genres_str = artist.genres.iter().take(5).cloned().collect::<Vec<_>>().join(", ");
            ui.painter().text(
                egui::pos2(text_x, header_rect.top() + 82.0),
                egui::Align2::LEFT_TOP,
                &genres_str,
                egui::FontId::proportional(12.0),
                theme::text_secondary(),
            );
        }

        // Play button
        let play_btn_rect = egui::Rect::from_min_size(
            egui::pos2(text_x, header_rect.top() + 110.0),
            egui::vec2(120.0, 36.0),
        );
        let play_response = ui.allocate_rect(play_btn_rect, egui::Sense::click());
        let play_bg = if play_response.hovered() { theme::green_hover() } else { theme::green() };
        ui.painter().rect_filled(play_btn_rect, 18.0, play_bg);
        ui.painter().text(
            play_btn_rect.center(),
            egui::Align2::CENTER_CENTER,
            "Play",
            egui::FontId::proportional(14.0),
            theme::bg_black(),
        );
        if play_response.clicked() {
            let track_uris: Vec<PlayableId<'static>> = top_tracks
                .iter()
                .map(|t| PlayableId::Track(t.id.clone()))
                .collect();
            if !track_uris.is_empty() {
                let _ = client_pub.send(ClientRequest::Player(
                    crate::client::PlayerRequest::StartPlayback(
                        crate::state::Playback::URIs(track_uris, None),
                        None,
                    ),
                ));
            }
        }
    });

    ui.add_space(32.0);

    // === Popular Tracks Section ===
    ui.horizontal(|ui| {
        ui.add_space(24.0);
        ui.label(
            egui::RichText::new("Popular")
                .size(18.0)
                .strong()
                .color(theme::text_primary()),
        );
    });
    ui.add_space(12.0);

    let player = state.player.read();
    let current_track_uri: Option<String> = player.playback.as_ref().and_then(|p| {
        p.item.as_ref().map(|item| match item {
            rspotify::model::PlayableItem::Track(t) => {
                t.id.as_ref().map(|id| id.uri()).unwrap_or_default()
            }
            _ => String::new(),
        })
    });
    drop(player);

    egui::ScrollArea::vertical()
        .id_salt("artist_top_tracks")
        .max_height(400.0)
        .show(ui, |ui| {
            for (i, track) in top_tracks.iter().enumerate() {
                let is_playing = current_track_uri
                    .as_ref()
                    .map_or(false, |uri| uri == &track.id.uri());

                let row_height = 48.0;
                let (row_rect, response) = ui
                    .allocate_exact_size(
                        egui::vec2(ui.available_width(), row_height),
                        egui::Sense::click(),
                    );

                let bg = if response.hovered() {
                    egui::Color32::from_rgb(10, 10, 10)
                } else if i % 2 == 0 {
                    egui::Color32::from_rgb(10, 10, 10)
                } else {
                    theme::bg_black()
                };
                ui.painter().rect_filled(row_rect, 4.0, bg);

                // Number / play indicator
                let num_color = if is_playing { theme::green() } else { theme::text_muted() };
                let num_str = if is_playing { "\u{25B6}".to_string() } else { format!("{}", i + 1) };
                ui.painter().text(
                    row_rect.left_center() + egui::vec2(28.0, 0.0),
                    egui::Align2::CENTER_CENTER,
                    &num_str,
                    egui::FontId::monospace(12.0),
                    num_color,
                );

                // Track thumbnail
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
                                theme::bg_active(),
                            );
                            egui::Image::new(texture)
                                .corner_radius(theme::ART_CORNER_RADIUS)
                                .paint_at(ui, thumb_rect);
                            thumb_drawn = true;
                        }
                    }
                }
                if !thumb_drawn {
                    ui.painter().rect_filled(thumb_rect, theme::ART_CORNER_RADIUS, theme::bg_active());
                    ui.painter().text(
                        thumb_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "\u{266B}",
                        egui::FontId::proportional(14.0),
                        theme::text_muted(),
                    );
                }

                // Track name
                let title_color = if is_playing { theme::green() } else { theme::text_primary() };
                ui.painter().text(
                    row_rect.left_center() + egui::vec2(92.0, -7.0),
                    egui::Align2::LEFT_CENTER,
                    &track.name,
                    egui::FontId::proportional(14.0),
                    title_color,
                );

                // Album name
                let album_name = track.album_info();
                if !album_name.is_empty() {
                    ui.painter().text(
                        row_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        &album_name,
                        egui::FontId::proportional(12.0),
                        theme::text_dim(),
                    );
                }

                // Duration
                let dur_str = theme::format_duration_secs(track.duration.as_secs());
                ui.painter().text(
                    row_rect.right_center() + egui::vec2(-52.0, 0.0),
                    egui::Align2::RIGHT_CENTER,
                    &dur_str,
                    egui::FontId::monospace(12.0),
                    theme::text_dim(),
                );

                // "..." button on hover
                let more_btn_rect = egui::Rect::from_center_size(
                    row_rect.right_center() + egui::vec2(-16.0, 0.0),
                    egui::vec2(24.0, 24.0),
                );
                if response.hovered() {
                    let more_resp = ui.allocate_rect(more_btn_rect, egui::Sense::click());
                    let more_bg = if more_resp.hovered() {
                        egui::Color32::from_rgb(40, 40, 40)
                    } else {
                        egui::Color32::TRANSPARENT
                    };
                    ui.painter().rect_filled(more_btn_rect, 12.0, more_bg);
                    ui.painter().text(
                        more_btn_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "\u{22EF}",
                        egui::FontId::proportional(14.0),
                        theme::text_dim(),
                    );
                    if more_resp.clicked() {
                        context_menu.open(
                            ContextTarget::Track {
                                track: track.clone(),
                                index: i,
                                playlist_id: None,
                            },
                            more_btn_rect.left_bottom(),
                        );
                    }
                }

                // Right-click opens context menu
                if response.secondary_clicked() {
                    if let Some(click_pos) = response.interact_pointer_pos() {
                        context_menu.open(
                            ContextTarget::Track {
                                track: track.clone(),
                                index: i,
                                playlist_id: None,
                            },
                            click_pos,
                        );
                    }
                }

                // Play button on hover
                if response.hovered() && !is_playing {
                    let play_btn_rect = egui::Rect::from_center_size(
                        row_rect.left_center() + egui::vec2(28.0, 0.0),
                        egui::vec2(24.0, 24.0),
                    );
                    ui.painter().rect_filled(play_btn_rect, 12.0, theme::green());
                    ui.painter().text(
                        play_btn_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "\u{25B6}",
                        egui::FontId::proportional(10.0),
                        theme::bg_black(),
                    );
                }

                // Click to play track
                if response.clicked() {
                    let track_uris: Vec<PlayableId<'static>> = top_tracks
                        .iter()
                        .map(|t| PlayableId::Track(t.id.clone()))
                        .collect();
                    let _ = client_pub.send(ClientRequest::Player(
                        crate::client::PlayerRequest::StartPlayback(
                            crate::state::Playback::URIs(
                                track_uris,
                                Some(rspotify::model::Offset::Uri(track.id.uri())),
                            ),
                            None,
                        ),
                    ));
                }

                // Row divider
                let div = egui::Rect::from_min_size(
                    row_rect.left_bottom() + egui::vec2(24.0, 0.0),
                    egui::vec2(row_rect.width() - 48.0, 1.0),
                );
                ui.painter().rect_filled(div, 0.0, theme::divider());
            }
        });

    ui.add_space(32.0);

    // === Albums Section ===
    if !albums.is_empty() {
        ui.horizontal(|ui| {
            ui.add_space(24.0);
            ui.label(
                egui::RichText::new("Discography")
                    .size(18.0)
                    .strong()
                    .color(theme::text_primary()),
            );
        });
        ui.add_space(12.0);

        egui::ScrollArea::horizontal()
            .id_salt("artist_albums_h")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(24.0);
                    for album in albums.iter() {
                        let sub = format!(
                            "{} · {}",
                            album.album_type(),
                            album.year()
                        );
                        let cover_path = image_cache::album_cover_path(album);
                        if let (Some(path), Some(url)) = (&cover_path, &album.cover_url) {
                            if !path.exists() {
                                image_cache.request_download(url, path);
                            }
                        }
                        let album_clone = album.clone();
                        let response = artist_album_card(ui, &album.name, &sub, cover_path.as_deref(), image_cache, || {
                            action = Action::OpenSearchResultAlbum(album_clone);
                        });
                        if response.secondary_clicked() {
                            if let Some(click_pos) = response.interact_pointer_pos() {
                                context_menu.open(
                                    ContextTarget::Album(album.clone()),
                                    click_pos,
                                );
                            }
                        }
                        ui.add_space(12.0);
                    }
                });
            });

        ui.add_space(32.0);
    }

    // === Related Artists Section ===
    if !related_artists.is_empty() {
        // Divider
        let div_rect = ui.allocate_space(egui::vec2(ui.available_width() - 48.0, 1.0)).1;
        ui.painter().rect_filled(
            egui::Rect::from_min_size(div_rect.min + egui::vec2(24.0, 0.0), div_rect.size()),
            0.0,
            theme::divider(),
        );
        ui.add_space(24.0);

        ui.horizontal(|ui| {
            ui.add_space(24.0);
            ui.label(
                egui::RichText::new("Fans also like")
                    .size(18.0)
                    .strong()
                    .color(theme::text_primary()),
            );
        });
        ui.add_space(12.0);

        egui::ScrollArea::horizontal()
            .id_salt("artist_related_h")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(24.0);
                    for related in related_artists.iter() {
                        let related_clone = related.clone();
                        let cover_path = image_cache::artist_cover_path(related);
                        if let (Some(path), Some(url)) = (&cover_path, &related.image_url) {
                            if !path.exists() {
                                image_cache.request_download(url, path);
                            }
                        }
                        let response = artist_card(ui, &related.name, cover_path.as_deref(), image_cache, || {
                            action = Action::OpenArtist(related_clone);
                        });
                        if response.secondary_clicked() {
                            if let Some(click_pos) = response.interact_pointer_pos() {
                                context_menu.open(
                                    ContextTarget::Artist(related.clone()),
                                    click_pos,
                                );
                            }
                        }
                        ui.add_space(12.0);
                    }
                });
            });

        ui.add_space(24.0);
    }

    action
}

fn format_followers(count: u64) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M listeners", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}K listeners", count as f64 / 1_000.0)
    } else {
        format!("{} listeners", count)
    }
}

fn artist_album_card(
    ui: &mut egui::Ui,
    title: &str,
    subtitle: &str,
    cover_path: Option<&std::path::Path>,
    image_cache: &mut ImageCache,
    on_click: impl FnOnce(),
) -> egui::Response {
    let width = 160.0;
    let height = 210.0;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());

    let bg = if response.hovered() {
        theme::bg_hover()
    } else {
        egui::Color32::from_rgb(17, 17, 17)
    };
    ui.painter().rect_filled(rect, 4.0, bg);

    let art_size = width - 24.0;
    let art_rect = egui::Rect::from_min_size(
        rect.min + egui::vec2(12.0, 12.0),
        egui::vec2(art_size, art_size),
    );

    let mut art_drawn = false;
    if let Some(path) = cover_path {
        if let Some(texture) = image_cache.get_texture(ui.ctx(), path) {
            ui.painter().rect_filled(art_rect, theme::ART_CORNER_RADIUS, theme::bg_active());
            egui::Image::new(texture)
                .corner_radius(theme::ART_CORNER_RADIUS)
                .paint_at(ui, art_rect);
            art_drawn = true;
        }
    }

    if !art_drawn {
        ui.painter().rect_filled(art_rect, theme::ART_CORNER_RADIUS, theme::bg_active());
        ui.painter().text(
            art_rect.center(),
            egui::Align2::CENTER_CENTER,
            "\u{266B}",
            egui::FontId::proportional(28.0),
            theme::text_muted(),
        );
    }

    // Hover overlay with play button
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
        ui.painter().rect_filled(play_rect, 20.0, theme::green());
        ui.painter().text(
            play_rect.center(),
            egui::Align2::CENTER_CENTER,
            "\u{25B6}",
            egui::FontId::proportional(16.0),
            theme::bg_black(),
        );
    }

    // Title
    ui.painter().text(
        rect.left_top() + egui::vec2(12.0, art_size + 26.0),
        egui::Align2::LEFT_TOP,
        title,
        egui::FontId::proportional(13.0),
        theme::text_primary(),
    );

    // Subtitle
    ui.painter().text(
        rect.left_top() + egui::vec2(12.0, art_size + 46.0),
        egui::Align2::LEFT_TOP,
        subtitle,
        egui::FontId::proportional(11.0),
        theme::text_dim(),
    );

    if response.clicked() {
        on_click();
    }

    response
}

fn artist_card(
    ui: &mut egui::Ui,
    name: &str,
    cover_path: Option<&std::path::Path>,
    image_cache: &mut ImageCache,
    on_click: impl FnOnce(),
) -> egui::Response {
    let width = 160.0;
    let height = 200.0;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());

    let bg = if response.hovered() {
        theme::bg_hover()
    } else {
        theme::bg_card()
    };
    ui.painter().rect_filled(rect, 8.0, bg);

    // Artist circle
    let circle_size = 100.0;
    let circle_rect = egui::Rect::from_center_size(
        rect.center() + egui::vec2(0.0, -30.0),
        egui::vec2(circle_size, circle_size),
    );

    let mut img_drawn = false;
    if let Some(path) = cover_path {
        if let Some(texture) = image_cache.get_texture(ui.ctx(), path) {
            ui.painter().rect_filled(circle_rect, circle_size / 2.0, theme::bg_active());
            egui::Image::new(texture)
                .corner_radius(circle_size / 2.0)
                .paint_at(ui, circle_rect);
            img_drawn = true;
        }
    }
    if !img_drawn {
        ui.painter().rect_filled(circle_rect, circle_size / 2.0, theme::bg_active());
        ui.painter().text(
            circle_rect.center(),
            egui::Align2::CENTER_CENTER,
            "🎤",
            egui::FontId::proportional(28.0),
            theme::text_muted(),
        );
    }

    // Name
    ui.painter().text(
        rect.center() + egui::vec2(0.0, 40.0),
        egui::Align2::CENTER_CENTER,
        name,
        egui::FontId::proportional(13.0),
        theme::text_primary(),
    );
    ui.painter().text(
        rect.center() + egui::vec2(0.0, 58.0),
        egui::Align2::CENTER_CENTER,
        "Artist",
        egui::FontId::proportional(11.0),
        theme::text_dim(),
    );

    if response.clicked() {
        on_click();
    }

    response
}

pub fn render_help(
    ui: &mut egui::Ui,
    keybindings: &[crate::key::CommandBinding],
    search_query: &mut String,
) {
    theme::page_title(ui, "Keyboard Shortcuts");

    // Search bar
    ui.horizontal(|ui| {
        ui.add_space(24.0);
        let search_width = (ui.available_width() - 48.0).min(400.0);
        let search_rect = ui
            .allocate_exact_size(egui::vec2(search_width, 36.0), egui::Sense::click())
            .0;
        ui.painter().rect_filled(search_rect, 18.0, theme::bg_input());
        let text_rect = egui::Rect::from_min_size(
            search_rect.min + egui::vec2(12.0, 4.0),
            egui::vec2(search_rect.width() - 24.0, search_rect.height() - 8.0),
        );
        ui.put(
            text_rect,
            egui::TextEdit::singleline(search_query)
                .hint_text(
                    egui::RichText::new("Search shortcuts...")
                        .color(theme::text_muted()),
                )
                .frame(false)
                .font(egui::FontId::proportional(13.0)),
        );
    });
    ui.add_space(16.0);

    let filter = search_query.to_lowercase();

    let mut categories: Vec<(&str, Vec<&crate::key::CommandBinding>)> = Vec::new();
    let category_order = [
        crate::key::CommandCategory::Navigation,
        crate::key::CommandCategory::Playback,
        crate::key::CommandCategory::Sorting,
        crate::key::CommandCategory::Actions,
        crate::key::CommandCategory::Pages,
        crate::key::CommandCategory::Other,
    ];

    for cat in &category_order {
        let items: Vec<_> = keybindings
            .iter()
            .filter(|b| b.category == *cat)
            .filter(|b| {
                filter.is_empty()
                    || b.description.to_lowercase().contains(&filter)
                    || b.command.0.to_lowercase().contains(&filter)
                    || b.keybindings.iter().any(|kb| {
                        kb.display_string().to_lowercase().contains(&filter)
                    })
            })
            .collect();
        if !items.is_empty() {
            categories.push((cat.display_name(), items));
        }
    }

    egui::ScrollArea::vertical()
        .id_salt("help_scroll")
        .show(ui, |ui| {
            for (cat_name, bindings) in &categories {
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    ui.add_space(24.0);
                    ui.label(
                        egui::RichText::new(*cat_name)
                            .size(16.0)
                            .strong()
                            .color(theme::green()),
                    );
                });
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.add_space(24.0);
                    let header_rect = ui
                        .allocate_exact_size(
                            egui::vec2(ui.available_width() - 48.0, 24.0),
                            egui::Sense::hover(),
                        )
                        .0;
                    ui.painter().text(
                        header_rect.left_center() + egui::vec2(8.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        "KEYS",
                        egui::FontId::monospace(11.0),
                        theme::text_dim(),
                    );
                    ui.painter().text(
                        header_rect.left_center() + egui::vec2(200.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        "DESCRIPTION",
                        egui::FontId::monospace(11.0),
                        theme::text_dim(),
                    );
                });

                let div_rect = ui
                    .allocate_space(egui::vec2(ui.available_width() - 48.0, 1.0))
                    .1;
                ui.painter().rect_filled(
                    egui::Rect::from_min_size(
                        div_rect.min + egui::vec2(24.0, 0.0),
                        div_rect.size(),
                    ),
                    0.0,
                    theme::divider(),
                );

                for binding in bindings {
                    let row_height = 32.0;
                    let row_rect = ui
                        .allocate_exact_size(
                            egui::vec2(ui.available_width(), row_height),
                            egui::Sense::hover(),
                        )
                        .0;

                    if ui.allocate_rect(row_rect, egui::Sense::hover()).hovered() {
                        ui.painter()
                            .rect_filled(row_rect, 4.0, theme::bg_card());
                    }

                    let key_strs: Vec<_> = binding
                        .keybindings
                        .iter()
                        .map(|kb| kb.display_string())
                        .collect();

                    let mut x_offset = 32.0_f32;
                    for (i, key_str) in key_strs.iter().enumerate() {
                        if i > 0 {
                            ui.painter().text(
                                row_rect.left_center() + egui::vec2(x_offset, 0.0),
                                egui::Align2::LEFT_CENTER,
                                "|",
                                egui::FontId::proportional(11.0),
                                theme::text_muted(),
                            );
                            x_offset += 16.0;
                        }

                        let badge_padding = 6.0;
                        let badge_text_width = key_str.len() as f32 * 7.5 + badge_padding * 2.0;
                        let badge_rect = egui::Rect::from_min_size(
                            row_rect.left_top() + egui::vec2(x_offset, 6.0),
                            egui::vec2(badge_text_width, 20.0),
                        );
                        ui.painter().rect_filled(
                            badge_rect,
                            4.0,
                            egui::Color32::from_rgb(30, 30, 30),
                        );
                        ui.painter().rect_stroke(
                            badge_rect,
                            4.0,
                            egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 50, 50)),
                            egui::StrokeKind::Outside,
                        );
                        ui.painter().text(
                            badge_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            key_str,
                            egui::FontId::monospace(11.0),
                            theme::text_primary(),
                        );
                        x_offset += badge_text_width + 4.0;
                    }

                    ui.painter().text(
                        row_rect.left_center() + egui::vec2(200.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        binding.description,
                        egui::FontId::proportional(13.0),
                        theme::text_secondary(),
                    );

                    let div = egui::Rect::from_min_size(
                        row_rect.left_bottom() + egui::vec2(24.0, 0.0),
                        egui::vec2(row_rect.width() - 48.0, 1.0),
                    );
                    ui.painter().rect_filled(div, 0.0, theme::divider());
                }
            }

            ui.add_space(32.0);
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.label(
                    egui::RichText::new("Tip: Use vim-style count prefixes (e.g. 5j, 10k, 3gg)")
                        .size(12.0)
                        .color(theme::text_hint())
                        .italics(),
                );
            });
            ui.add_space(24.0);
        });
}
