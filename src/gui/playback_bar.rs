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

fn generate_waveform_bars(width: usize, seed: u64) -> Vec<f32> {
    let mut bars = Vec::with_capacity(width);
    for i in 0..width {
        let x = i as f32 / width as f32;
        let base = 0.3
            + 0.25 * (x * 6.28 + seed as f32 * 0.7).sin()
            + 0.15 * (x * 12.56 + seed as f32 * 1.3).sin()
            + 0.1 * (x * 18.84 + seed as f32 * 2.1).sin();
        bars.push(base.clamp(0.1, 0.9));
    }
    bars
}

pub fn render(
    ui: &mut egui::Ui,
    state: &SharedState,
    client_pub: &flume::Sender<ClientRequest>,
    image_cache: &mut ImageCache,
    waveform_cache: &mut Option<(String, usize, Vec<f32>)>,
) -> PlaybackBarResponse {
    let mut result = PlaybackBarResponse {
        navigate: None,
        device_button_clicked: false,
    };
    let player = state.player.read();
    let playback = player.current_playback();

    // Solid black background (AMOLED)
    let full_rect = ui.max_rect();
    ui.painter().rect_filled(full_rect, 0.0, theme::bg_black());

    // Subtle top glow border
    let glow_rect = egui::Rect::from_min_size(
        full_rect.min,
        egui::vec2(full_rect.width(), 2.0),
    );
    ui.painter().rect_filled(
        glow_rect,
        0.0,
        theme::with_alpha(theme::accent(), 30),
    );

    // Top border
    let rect = ui.allocate_space(egui::vec2(ui.available_width(), 1.0)).1;
    ui.painter().rect_filled(rect, 0.0, theme::with_alpha(theme::divider(), 60));

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

                // Track name with truncation to prevent overflow
                let track_name_rect = egui::Rect::from_min_size(
                    track_rect.min + egui::vec2(theme::PLAYBACK_ART_SIZE + 12.0, 12.0),
                    egui::vec2(track_info_width - theme::PLAYBACK_ART_SIZE - 28.0, 20.0),
                );
                let track_name_galley = ui.painter().layout_no_wrap(
                    name.clone(),
                    egui::FontId::proportional(14.0),
                    theme::text_primary(),
                );
                let truncated_name = if track_name_galley.size().x > track_name_rect.width() {
                    // Truncate with ellipsis
                    let mut s = name.clone();
                    while s.len() > 1 {
                        s.pop();
                        let test = format!("{s}...");
                        let galley = ui.painter().layout_no_wrap(
                            test.clone(),
                            egui::FontId::proportional(14.0),
                            theme::text_primary(),
                        );
                        if galley.size().x <= track_name_rect.width() {
                            s = test;
                            break;
                        }
                    }
                    s
                } else {
                    name
                };
                ui.painter().text(
                    track_name_rect.left_center(),
                    egui::Align2::LEFT_CENTER,
                    &truncated_name,
                    egui::FontId::proportional(14.0),
                    theme::text_primary(),
                );

                // Artists with truncation
                let artist_rect = egui::Rect::from_min_size(
                    track_rect.min + egui::vec2(theme::PLAYBACK_ART_SIZE + 12.0, 36.0),
                    egui::vec2(track_info_width - theme::PLAYBACK_ART_SIZE - 28.0, 18.0),
                );
                let artist_galley = ui.painter().layout_no_wrap(
                    artists_str.clone(),
                    egui::FontId::proportional(12.0),
                    theme::text_dim(),
                );
                let truncated_artist = if artist_galley.size().x > artist_rect.width() {
                    let mut s = artists_str;
                    while s.len() > 1 {
                        s.pop();
                        let test = format!("{s}...");
                        let galley = ui.painter().layout_no_wrap(
                            test.clone(),
                            egui::FontId::proportional(12.0),
                            theme::text_dim(),
                        );
                        if galley.size().x <= artist_rect.width() {
                            s = test;
                            break;
                        }
                    }
                    s
                } else {
                    artists_str
                };
                ui.painter().text(
                    artist_rect.left_center(),
                    egui::Align2::LEFT_CENTER,
                    &truncated_artist,
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
        let available = ui.available_width();
        let left_pad = ((available - center_width) / 2.0).max(0.0);
        ui.allocate_space(egui::vec2(left_pad, 0.0));

        ui.vertical(|ui| {
            // Playback controls row
            ui.horizontal(|ui| {
                let controls_width = 280.0;
                let pad = ((ui.available_width() - controls_width) / 2.0).max(0.0);
                ui.add_space(pad);

                let is_playing = playback.as_ref().is_some_and(|p| p.is_playing);

                // Shuffle
                let shuffle_on = playback.as_ref().is_some_and(|p| p.shuffle_state);
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
                    rspotify::model::RepeatState::Track => "⟳¹",
                    _ => "⟳",
                };
                if theme::icon_button(ui, repeat_icon, 30.0, repeat_active).clicked() {
                    let _ = client_pub.send(ClientRequest::Player(PlayerRequest::Repeat));
                }
            });

            ui.add_space(6.0);

            // Waveform seekbar row
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

                    // Waveform seekbar
                    let bar_width = (ui.available_width() - 100.0).max(100.0);
                    let bar_height = 28.0;
                    let (bar_rect, bar_response) =
                        ui.allocate_exact_size(egui::vec2(bar_width, bar_height), egui::Sense::click());

                    // Get track URI for cache key
                    let track_uri = playback.item.as_ref().map(|item| match item {
                        rspotify::model::PlayableItem::Track(t) => t.id.as_ref().map(|id| id.uri()).unwrap_or_default(),
                        rspotify::model::PlayableItem::Episode(e) => e.id.uri(),
                        _ => String::new(),
                    }).unwrap_or_default();

                    // Generate waveform bars (cached by track URI and bar count)
                    let num_bars = (bar_width / 3.0) as usize;
                    let cache_key_uri = track_uri.clone();
                    if waveform_cache.as_ref().map_or(true, |(uri, n, _)| *uri != cache_key_uri || *n != num_bars) {
                        let bars = generate_waveform_bars(num_bars, d_secs);
                        *waveform_cache = Some((cache_key_uri, num_bars, bars));
                    }
                    let waveform = &waveform_cache.as_ref().unwrap().2;

                    let bar_gap = 1.0;
                    let bar_w = (bar_width / num_bars as f32) - bar_gap;
                    let hover_pos = bar_response.hover_pos();

                    for (i, &amplitude) in waveform.iter().enumerate() {
                        let x = bar_rect.left() + i as f32 * (bar_w + bar_gap);
                        let h = amplitude * bar_height * 0.8;
                        let y_center = bar_rect.center().y;
                        let bar = egui::Rect::from_center_size(
                            egui::pos2(x + bar_w / 2.0, y_center),
                            egui::vec2(bar_w, h),
                        );

                        let played = (i as f32 / num_bars as f32) < ratio;

                        let is_hovered = hover_pos
                            .map(|pos| (x..=x + bar_w).contains(&pos.x))
                            .unwrap_or(false);

                        let color = if played {
                            if is_hovered {
                                theme::accent_hover()
                            } else {
                                theme::accent()
                            }
                        } else if is_hovered {
                            theme::lerp_color(theme::bg_active(), theme::text_dim(), 0.3)
                        } else {
                            theme::bg_active()
                        };

                        ui.painter().rect_filled(bar, 1.0, color);
                    }

                    // Hover time tooltip
                    if let Some(pos) = hover_pos {
                        let hover_ratio =
                            ((pos.x - bar_rect.left()) / bar_rect.width()).clamp(0.0, 1.0);
                        let hover_secs = (d_secs as f32 * hover_ratio) as u64;
                        let tooltip_text = theme::format_duration_secs(hover_secs);

                        let tooltip_pos = egui::pos2(pos.x, bar_rect.top() - 20.0);
                        let galley = ui.painter().layout_no_wrap(
                            tooltip_text,
                            egui::FontId::monospace(10.0),
                            theme::text_primary(),
                        );
                        let tooltip_rect = egui::Rect::from_center_size(
                            tooltip_pos,
                            galley.size() + egui::vec2(8.0, 4.0),
                        );
                        ui.painter().rect_filled(
                            tooltip_rect,
                            3.0,
                            theme::with_alpha(theme::bg_dark(), 220),
                        );
                        ui.painter().galley(
                            tooltip_rect.center() - galley.size() / 2.0,
                            galley,
                            theme::text_primary(),
                        );
                    }

                    // Seek on click
                    if bar_response.clicked() {
                        if let Some(click_pos) = bar_response.interact_pointer_pos() {
                            let click_ratio =
                                ((click_pos.x - bar_rect.left()) / bar_rect.width()).clamp(0.0, 1.0);
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
                    // Empty waveform
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("0:00")
                            .size(11.0)
                            .color(theme::text_muted())
                            .monospace(),
                    );
                    let bar_width = (ui.available_width() - 100.0).max(100.0);
                    let bar_height = 28.0;
                    let (bar_rect, _) =
                        ui.allocate_exact_size(egui::vec2(bar_width, bar_height), egui::Sense::hover());

                    // Static waveform placeholder
                    let num_bars = (bar_width / 3.0) as usize;
                    let bar_gap = 1.0;
                    let bar_w = (bar_width / num_bars as f32) - bar_gap;
                    for i in 0..num_bars {
                        let x = bar_rect.left() + i as f32 * (bar_w + bar_gap);
                        let h = (0.15 + 0.1 * (i as f32 * 0.3).sin()) * bar_height * 0.8;
                        let bar = egui::Rect::from_center_size(
                            egui::pos2(x + bar_w / 2.0, bar_rect.center().y),
                            egui::vec2(bar_w, h),
                        );
                        ui.painter().rect_filled(bar, 1.0, theme::bg_active());
                    }

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

            // Device button (always visible)
            if theme::icon_button(ui, "🖥", 28.0, false).clicked() {
                result.device_button_clicked = true;
            }

            ui.add_space(8.0);

            // Lyrics button
            if theme::icon_button(ui, "🎤", 28.0, false).clicked() {
                result.navigate = Some(View::Lyrics);
            }

            if let Some(ref playback) = playback {
                // Device name
                ui.add_space(16.0);
                ui.label(
                    egui::RichText::new(format!("{}%", playback.device.volume_percent.unwrap_or(50) as u32))
                        .size(11.0)
                        .color(theme::text_dim())
                        .monospace(),
                );

                // Volume
                let volume = playback
                    .device
                    .volume_percent
                    .unwrap_or(50) as f32;

                // Volume icon
                let vol_icon = if volume == 0.0 { "🔇" } else if volume < 30.0 { "🔈" } else if volume < 70.0 { "🔉" } else { "🔊" };
                ui.label(
                    egui::RichText::new(vol_icon).size(14.0).color(theme::text_dim()),
                );

                let mut vol = volume;
                let vol_slider = egui::Slider::new(&mut vol, 0.0..=100.0)
                    .show_value(false);
                let vol_resp = ui.add(vol_slider);
                if vol_resp.drag_stopped() {
                    let _ = client_pub.send(ClientRequest::Player(PlayerRequest::Volume(vol as u8)));
                }

                ui.add_space(12.0);

                // Mute button
                let mute_text = if volume == 0.0 { "🔇" } else { "🔊" };
                if ui.button(egui::RichText::new(mute_text).size(14.0)).clicked() {
                    let _ = client_pub.send(ClientRequest::Player(PlayerRequest::ToggleMute));
                }

                ui.add_space(8.0);

                // Device name label
                ui.label(
                    egui::RichText::new(&playback.device.name)
                        .size(11.0)
                        .color(theme::text_dim()),
                );
            }
        });
    });

    ui.allocate_space(egui::vec2(ui.available_width(), 8.0));
    result
}
