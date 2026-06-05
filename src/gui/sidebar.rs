use eframe::egui;

use crate::gui::{theme, Action, View};
use crate::state::{self, SharedState};

fn parent_view(view: &View) -> &View {
    match view {
        View::Tracks => &View::Library,
        View::ShowDetail => &View::Shows,
        View::BrowseCategory { .. } => &View::Browse,
        View::Artist => &View::Library,
        other => other,
    }
}

pub fn render(
    ui: &mut egui::Ui,
    current_view: &View,
    state: &SharedState,
    is_authenticated: bool,
    window_width: f32,
) -> Action {
    let mut action = Action::None;
    
    // Calculate responsive sidebar width
    let (_sidebar_width, collapsed) = theme::responsive_sidebar_width(window_width);

    // Solid black background (AMOLED)
    let full_rect = ui.max_rect();
    ui.painter().rect_filled(full_rect, 0.0, theme::bg_black());

    // Subtle right glow border
    let glow_rect = egui::Rect::from_min_size(
        egui::pos2(full_rect.right() - 1.0, full_rect.top()),
        egui::vec2(1.0, full_rect.height()),
    );
    ui.painter().rect_filled(
        glow_rect,
        0.0,
        theme::with_alpha(theme::divider(), 60),
    );

    // Logo area
    egui::ScrollArea::vertical()
        .id_salt("sidebar_scroll")
        .show(ui, |ui| {
    ui.allocate_space(egui::vec2(ui.available_width(), 16.0));
    ui.horizontal(|ui| {
        ui.add_space(20.0);
        ui.label(
            egui::RichText::new(theme::ICON_LIBRARY)
                .size(22.0)
                .color(theme::green()),
        );
        if !collapsed {
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("Spotify Player")
                    .size(18.0)
                    .strong()
                    .color(theme::text_primary()),
            );
        }
    });
    ui.allocate_space(egui::vec2(ui.available_width(), 16.0));

    // Sign In button (when not authenticated)
    if !is_authenticated {
        let (signin_rect, signin_resp) = ui
            .allocate_exact_size(egui::vec2(ui.available_width() - 32.0, 40.0), egui::Sense::click());
        let signin_bg = if signin_resp.hovered() {
            theme::green_hover()
        } else {
            theme::green()
        };
        ui.painter().rect_filled(
            signin_rect,
            egui::CornerRadius::same(theme::RADIUS_MEDIUM),
            signin_bg,
        );
        let signin_text = if collapsed { "\u{1F511}" } else { "\u{1F511} Sign In" };
        ui.painter().text(
            signin_rect.center(),
            egui::Align2::CENTER_CENTER,
            signin_text,
            egui::FontId::proportional(if collapsed { 16.0 } else { 14.0 }),
            theme::bg_black(),
        );
        if signin_resp.clicked() {
            action = Action::OpenAuthModal;
        }
        ui.add_space(8.0);
        theme::divider_line(ui);
        ui.add_space(8.0);
    }

    ui.allocate_space(egui::vec2(ui.available_width(), 4.0));

    // Navigation
    let nav = [
        (View::Library, theme::ICON_HOME, "Library"),
        (View::Search, theme::ICON_SEARCH, "Search"),
        (View::Browse, theme::ICON_BROWSE, "Browse"),
        (View::Shows, theme::ICON_SHOWS, "Shows"),
        (View::Queue, theme::ICON_QUEUE, "Queue"),
        (View::Lyrics, theme::ICON_LYRICS, "Lyrics"),
        (View::Settings, theme::ICON_SETTINGS, "Settings"),
        (View::Help, theme::ICON_HELP, "Help"),
    ];

    for (view, icon, label) in &nav {
        if theme::nav_item(ui, icon, label, *parent_view(current_view) == *view, collapsed).clicked() {
            action = Action::Navigate(view.clone());
        }
    }

    ui.allocate_space(egui::vec2(ui.available_width(), 12.0));
    theme::divider_line(ui);

    // Playlists
    theme::section_header(ui, "PLAYLISTS");

    // New Playlist button
    ui.add_space(4.0);
    let (btn_rect, btn_resp) = ui
        .allocate_exact_size(egui::vec2(ui.available_width(), 36.0), egui::Sense::click());
    let btn_bg = if btn_resp.hovered() {
        theme::bg_hover()
    } else {
        theme::bg_dark()
    };
    ui.painter()
        .rect_filled(btn_rect, egui::CornerRadius::same(theme::RADIUS_MEDIUM), btn_bg);
    ui.painter().rect_stroke(
        btn_rect,
        egui::CornerRadius::same(theme::RADIUS_MEDIUM),
        egui::Stroke::new(1.0, theme::text_muted()),
        egui::StrokeKind::Outside,
    );
    if collapsed {
        // Center the icon when collapsed
        ui.painter().text(
            btn_rect.center(),
            egui::Align2::CENTER_CENTER,
            "\u{2795}",
            egui::FontId::proportional(14.0),
            theme::green(),
        );
    } else {
        ui.painter().text(
            btn_rect.left_center() + egui::vec2(12.0, 0.0),
            egui::Align2::LEFT_CENTER,
            "\u{2795}",
            egui::FontId::proportional(14.0),
            theme::green(),
        );
        ui.painter().text(
            btn_rect.left_center() + egui::vec2(34.0, 0.0),
            egui::Align2::LEFT_CENTER,
            "New Playlist",
            egui::FontId::proportional(13.0),
            theme::text_secondary(),
        );
    }
    if btn_resp.clicked() {
        action = Action::OpenCreatePlaylist;
    }
    ui.add_space(4.0);

    // Read all needed data once to avoid multiple lock acquisitions per frame (L121-196)
    let (playlists_snapshot, albums_snapshot, shows_snapshot, artists_snapshot) = {
        let data = state.data.read();
        (
            data.user_data.playlists.clone(),
            data.user_data.saved_albums.clone(),
            data.user_data.saved_shows.clone(),
            data.user_data.followed_artists.clone(),
        )
    };

    {
        for (i, item) in playlists_snapshot.iter().enumerate() {
            match item {
                state::PlaylistFolderItem::Playlist(playlist) => {
                    if collapsed {
                        // Just show first letter or icon when collapsed
                        let (rect, resp) = ui
                            .allocate_exact_size(egui::vec2(ui.available_width(), 36.0), egui::Sense::click());
                        let bg = if resp.hovered() { theme::bg_card() } else { egui::Color32::TRANSPARENT };
                        ui.painter().rect_filled(rect, theme::RADIUS_SMALL, bg);
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            &playlist.name.chars().next().unwrap_or('\u{266B}').to_string(),
                            egui::FontId::proportional(14.0),
                            if resp.hovered() { theme::text_primary() } else { theme::text_secondary() },
                        );
                        if resp.clicked() {
                            action = Action::OpenPlaylist(i);
                        }
                    } else if theme::list_item(ui, &playlist.name, &playlist.owner.0, false).clicked() {
                        action = Action::OpenPlaylist(i);
                    }
                }
                state::PlaylistFolderItem::Folder(folder) => {
                    if !collapsed {
                        let (rect, resp) = ui
                            .allocate_exact_size(egui::vec2(ui.available_width(), 28.0), egui::Sense::click());
                        let bg = if resp.hovered() { theme::bg_card() } else { egui::Color32::TRANSPARENT };
                        ui.painter().rect_filled(rect, theme::RADIUS_SMALL, bg);
                        ui.painter().text(
                            rect.left_center() + egui::vec2(16.0, 0.0),
                            egui::Align2::LEFT_CENTER,
                            format!("\u{1F4C1} {}", folder.name),
                            egui::FontId::proportional(13.0),
                            if resp.hovered() { theme::text_secondary() } else { theme::text_dim() },
                        );
                    }
                }
            }
        }
    }

    ui.allocate_space(egui::vec2(ui.available_width(), 8.0));
    theme::divider_line(ui);

    // Albums (skip when collapsed)
    if !collapsed {
        theme::section_header(ui, "ALBUMS");

        {
            for (i, album) in albums_snapshot.iter().enumerate() {
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
                if theme::list_item(ui, &album.name, &sub, false).clicked() {
                    action = Action::OpenAlbum(i);
                }
            }
        }

        ui.allocate_space(egui::vec2(ui.available_width(), 8.0));
        theme::divider_line(ui);

        // Shows
        theme::section_header(ui, "SHOWS");

        {
            for show in shows_snapshot.iter() {
                if theme::list_item(ui, &show.name, &show.publisher, false).clicked() {
                    action = Action::OpenShowDetail(show.clone());
                }
            }
        }

        ui.allocate_space(egui::vec2(ui.available_width(), 8.0));
        theme::divider_line(ui);

        // Artists
        theme::section_header(ui, "ARTISTS");

        {
            for artist in artists_snapshot.iter() {
                if theme::list_item(ui, &artist.name, "", false).clicked() {
                    action = Action::OpenArtist(artist.clone());
                }
            }
        }

        ui.allocate_space(egui::vec2(ui.available_width(), 8.0));
        theme::divider_line(ui);

        // Keyboard hints — two-column layout with fixed widths
        ui.add_space(4.0);
        let hints: &[(&str, &str)] = &[
            ("Space", "Play/Pause"),
            ("j / k", "Navigate"),
            ("gg / G", "First / Last"),
            ("Enter", "Play selected"),
            ("?", "Help"),
            ("Esc", "Back"),
            ("Ctrl+i/d", "Volume"),
            ("l", "Lyrics"),
            ("z", "Queue"),
            ("gs", "Search"),
        ];
        for (key, desc) in hints {
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(70.0, 16.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.label(
                            egui::RichText::new(*key)
                                .size(11.0)
                                .color(theme::green())
                                .monospace(),
                        );
                    },
                );
                ui.label(
                    egui::RichText::new(*desc)
                        .size(11.0)
                        .color(theme::text_secondary()),
                );
            });
        }
    }
        }); // end ScrollArea

    action
}
