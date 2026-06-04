use eframe::egui;
use rspotify::prelude::Id;

use crate::client::{ClientRequest, PlayerRequest};
use crate::gui::image_cache::ImageCache;
use crate::gui::theme;
use crate::gui::View;
use crate::state::SharedState;

pub struct PlaybackBarResponse {
    pub navigate: Option<View>,
    pub device_button_clicked: bool,
}

pub fn render(
    ui: &mut egui::Ui,
    state: &SharedState,
    client_pub: &flume::Sender<ClientRequest>,
    image_cache: &mut ImageCache,
) -> PlaybackBarResponse {
    let mut result = PlaybackBarResponse {
        navigate: None,
        device_button_clicked: false,
    };
    let player = state.player.read();
    let playback = player.current_playback();

    // Top border
    let rect = ui.allocate_space(egui::vec2(ui.available_width(), 1.0)).1;
    ui.painter().rect_filled(rect, 0.0, theme::divider());

    ui.allocate_space(egui::vec2(ui.available_width(), 9.0));

    ui.horizontal(|ui| {
        ui.add_space(16.0);

        // === LEFT: Track info ===
        let track_info_width = 320.0;
        ui.allocate_space(egui::vec2(track_info_width, 0.0));

        let track_rect = ui
            .allocate_space(egui::vec2(track_info_width - 16.0, theme::PLAYBACK_ART_SIZE))
            .1;

        // Album art
        let art_rect = egui::Rect::from_min_size(
            track_rect.min,
            egui::vec2(theme::PLAYBACK_ART_SIZE, theme::PLAYBACK_ART_SIZE),
        );

        let mut art_drawn = false;

        if let Some(ref playback) = playback {
            if let Some(ref item) = playback.item {
                // Try to load cover image
                let cover_path = match item {
                    rspotify::model::PlayableItem::Track(track) => {
                        let album = &track.album;
                        let artist_name = album.artists.first().map(|a| a.name.as_str()).unwrap_or("");
                        let id_str = album.id.as_ref().map(|id| id.id().to_string()).unwrap_or_default();
                        let id_prefix = &id_str[..id_str.len().min(6)];
                        let filename = format!("{}-{}-cover-{}.jpg", album.name, artist_name, id_prefix)
                            .replace('/', "");
                        Some(crate::config::get_config().cache_folder.join("image").join(filename))
                    }
                    _ => None,
                };

                if let Some(path) = cover_path {
                    if let Some(texture) = image_cache.get_texture(ui.ctx(), &path) {
                        ui.painter().rect_filled(
                            art_rect,
                            theme::ART_CORNER_RADIUS,
                            theme::bg_active(),
                        );
                        egui::Image::new(texture)
                            .corner_radius(theme::ART_CORNER_RADIUS)
                            .paint_at(ui, art_rect);
                        art_drawn = true;
                    }
                }

                let (name, artists_str) = match item {
                    rspotify::model::PlayableItem::Track(track) => {
                        let a: Vec<_> = track.artists.iter().map(|a| a.name.as_str()).collect();
                        (track.name.clone(), a.join(", "))
                    }
                    rspotify::model::PlayableItem::Episode(ep) => {
                        (ep.name.clone(), ep.show.name.clone())
                    }
                    _ => (String::new(), String::new()),
                };

                if !art_drawn {
                    ui.painter().rect_filled(art_rect, theme::ART_CORNER_RADIUS, theme::bg_active());
                    ui.painter().text(
                        art_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "\u{266B}",
                        egui::FontId::proportional(24.0),
                        theme::text_dim(),
                    );
                }

                // Track name
                ui.painter().text(
                    track_rect.min + egui::vec2(theme::PLAYBACK_ART_SIZE + 12.0, 20.0),
                    egui::Align2::LEFT_CENTER,
                    &name,
                    egui::FontId::proportional(14.0),
                    theme::text_primary(),
                );

                // Artists
                ui.painter().text(
                    track_rect.min + egui::vec2(theme::PLAYBACK_ART_SIZE + 12.0, 44.0),
                    egui::Align2::LEFT_CENTER,
                    &artists_str,
                    egui::FontId::proportional(12.0),
                    theme::text_dim(),
                );
            } else {
                ui.painter().rect_filled(art_rect, theme::ART_CORNER_RADIUS, theme::bg_active());
                ui.painter().text(
                    art_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "\u{266B}",
                    egui::FontId::proportional(24.0),
                    theme::text_muted(),
                );
                ui.painter().text(
                    track_rect.min + egui::vec2(theme::PLAYBACK_ART_SIZE + 12.0, 32.0),
                    egui::Align2::LEFT_CENTER,
                    "No track playing",
                    egui::FontId::proportional(13.0),
                    theme::text_dim(),
                );
            }
        } else {
            ui.painter().rect_filled(art_rect, theme::ART_CORNER_RADIUS, theme::bg_active());
            ui.painter().text(
                art_rect.center(),
                egui::Align2::CENTER_CENTER,
                "\u{266B}",
                egui::FontId::proportional(24.0),
                theme::text_muted(),
            );
            ui.painter().text(
                track_rect.min + egui::vec2(theme::PLAYBACK_ART_SIZE + 12.0, 32.0),
                egui::Align2::LEFT_CENTER,
                "Connect to Spotify",
                egui::FontId::proportional(13.0),
                theme::text_dim(),
            );
        }

        // === CENTER: Controls + Progress ===
        let center_width = 480.0;
        let _center_x = (ui.available_width() - center_width) / 2.0 + ui.next_widget_position().x;
        let _ = ui.allocate_space(egui::vec2(
            (ui.available_width() - center_width - 200.0).max(40.0),
            0.0,
        ));

        ui.vertical(|ui| {
            // Playback controls row
            ui.horizontal(|ui| {
                let controls_width = 280.0;
                let pad = ((ui.available_width() - controls_width) / 2.0).max(0.0);
                ui.add_space(pad);

                let is_playing = playback.as_ref().map_or(false, |p| p.is_playing);

                // Shuffle
                let shuffle_on = playback.as_ref().map_or(false, |p| p.shuffle_state);
                if theme::icon_button(ui, "⇄", 30.0, shuffle_on).clicked() {
                    let _ = client_pub.send(ClientRequest::Player(PlayerRequest::Shuffle));
                }

                ui.add_space(12.0);

                // Previous
                if theme::icon_button(ui, "⏮", 30.0, false).clicked() {
                    let _ = client_pub.send(ClientRequest::Player(PlayerRequest::PreviousTrack));
                }

                ui.add_space(8.0);

                // Play/Pause
                if theme::play_pause_button(ui, is_playing).clicked() {
                    let _ = client_pub.send(ClientRequest::Player(PlayerRequest::ResumePause));
                }

                ui.add_space(8.0);

                // Next
                if theme::icon_button(ui, "⏭", 30.0, false).clicked() {
                    let _ = client_pub.send(ClientRequest::Player(PlayerRequest::NextTrack));
                }

                ui.add_space(12.0);

                // Repeat
                let repeat_state = playback
                    .as_ref()
                    .map_or(rspotify::model::RepeatState::Off, |p| p.repeat_state);
                let repeat_active = repeat_state != rspotify::model::RepeatState::Off;
                let repeat_icon = match repeat_state {
                    rspotify::model::RepeatState::Track => "⟳",
                    _ => "⟳",
                };
                if theme::icon_button(ui, repeat_icon, 30.0, repeat_active).clicked() {
                    let _ = client_pub.send(ClientRequest::Player(PlayerRequest::Repeat));
                }
            });

            ui.add_space(6.0);

            // Progress bar row
            ui.horizontal(|ui| {
                if let Some(ref playback) = playback {
                    let progress = playback.progress.unwrap_or(chrono::Duration::zero());
                    let duration = playback
                        .item
                        .as_ref()
                        .map(|item| match item {
                            rspotify::model::PlayableItem::Track(t) => t.duration,
                            rspotify::model::PlayableItem::Episode(e) => e.duration,
                            _ => chrono::Duration::zero(),
                        })
                        .unwrap_or(chrono::Duration::zero());

                    let p_secs = progress.num_seconds().max(0) as u64;
                    let d_secs = duration.num_seconds().max(0) as u64;
                    let ratio = if d_secs > 0 {
                        (p_secs as f32 / d_secs as f32).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };

                    // Current time
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(theme::format_duration_secs(p_secs))
                            .size(11.0)
                            .color(theme::text_dim())
                            .monospace(),
                    );

                    ui.add_space(8.0);

                    // Progress bar
                    let bar_width = (ui.available_width() - 100.0).max(100.0);
                    let bar_height = 4.0;
                    let (bar_rect, bar_response) =
                        ui.allocate_exact_size(egui::vec2(bar_width, bar_height + 8.0), egui::Sense::click());

                    let bar_y = bar_rect.center().y - bar_height / 2.0;
                    let full_bar = egui::Rect::from_min_size(
                        egui::pos2(bar_rect.left(), bar_y),
                        egui::vec2(bar_rect.width(), bar_height),
                    );

                    // Track background
                    ui.painter().rect_filled(full_bar, 2.0, theme::bg_active());

                    // Progress fill
                    let progress_rect = egui::Rect::from_min_size(
                        full_bar.min,
                        egui::vec2(full_bar.width() * ratio, full_bar.height()),
                    );
                    ui.painter().rect_filled(progress_rect, 2.0, theme::green());

                    // Hover dot
                    if bar_response.hovered() {
                        let dot_x = full_bar.left() + full_bar.width() * ratio;
                        ui.painter().circle_filled(
                            egui::pos2(dot_x, bar_rect.center().y),
                            4.0,
                            theme::green(),
                        );
                    }

                    // Seek on click
                    if bar_response.clicked() {
                        if let Some(click_pos) = bar_response.interact_pointer_pos() {
                            let click_ratio =
                                ((click_pos.x - full_bar.left()) / full_bar.width()).clamp(0.0, 1.0);
                            let seek_ms = (d_secs as f64 * click_ratio as f64 * 1000.0) as i64;
                            let _ = client_pub.send(ClientRequest::Player(PlayerRequest::SeekTrack(
                                chrono::Duration::milliseconds(seek_ms),
                            )));
                        }
                    }

                    ui.add_space(8.0);

                    // Duration
                    ui.label(
                        egui::RichText::new(theme::format_duration_secs(d_secs))
                            .size(11.0)
                            .color(theme::text_dim())
                            .monospace(),
                    );
                } else {
                    // Empty progress bar
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("0:00")
                            .size(11.0)
                            .color(theme::text_muted())
                            .monospace(),
                    );
                    let bar_width = (ui.available_width() - 100.0).max(100.0);
                    let (bar_rect, _) =
                        ui.allocate_exact_size(egui::vec2(bar_width, 12.0), egui::Sense::hover());
                    let bar_y = bar_rect.center().y - 2.0;
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(
                            egui::pos2(bar_rect.left(), bar_y),
                            egui::vec2(bar_rect.width(), 4.0),
                        ),
                        2.0,
                        theme::bg_active(),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("0:00")
                            .size(11.0)
                            .color(theme::text_muted())
                            .monospace(),
                    );
                }
            });
        });

        // === RIGHT: Volume + Device ===
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(16.0);

            // Device name
            if let Some(ref playback) = playback {
                ui.label(
                    egui::RichText::new(&playback.device.name)
                        .size(11.0)
                        .color(theme::text_dim()),
                );
                ui.add_space(12.0);

                // Volume
                let volume = playback
                    .device
                    .volume_percent
                    .unwrap_or(50) as f32;

                let mut vol = volume;
                let _vol_width = 100.0;
                let vol_slider = egui::Slider::new(&mut vol, 0.0..=100.0)
                    .show_value(false)
                    ;
                if ui.add(vol_slider).changed() {
                    let _ = client_pub.send(ClientRequest::Player(PlayerRequest::Volume(vol as u8)));
                }

                // Volume icon
                let vol_icon = if volume == 0.0 { "🔇" } else if volume < 30.0 { "🔈" } else if volume < 70.0 { "🔉" } else { "🔊" };
                ui.label(
                    egui::RichText::new(vol_icon).size(14.0).color(theme::text_dim()),
                );

                ui.label(
                    egui::RichText::new(format!("{}%", volume as u32))
                        .size(11.0)
                        .color(theme::text_dim())
                        .monospace(),
                );

                ui.add_space(16.0);

                // Lyrics button
                if theme::icon_button(ui, "🎤", 28.0, false).clicked() {
                    result.navigate = Some(View::Lyrics);
                }

                ui.add_space(8.0);

                // Device button
                if theme::icon_button(ui, "🖥", 28.0, false).clicked() {
                    result.device_button_clicked = true;
                }
            }
        });
    });

    ui.allocate_space(egui::vec2(ui.available_width(), 8.0));
    result
}
