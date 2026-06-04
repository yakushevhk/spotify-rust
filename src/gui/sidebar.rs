use eframe::egui;

use crate::gui::{theme, Action, View};
use crate::state::{self, SharedState};

pub fn render(
    ui: &mut egui::Ui,
    current_view: &View,
    state: &SharedState,
) -> Action {
    let mut action = Action::None;

    // Logo area
    ui.allocate_space(egui::vec2(ui.available_width(), 16.0));
    ui.horizontal(|ui| {
        ui.add_space(20.0);
        ui.label(
            egui::RichText::new("♫")
                .size(22.0)
                .color(theme::green()),
        );
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new("Spotify Player")
                .size(18.0)
                .strong()
                .color(theme::text_primary()),
        );
    });
    ui.allocate_space(egui::vec2(ui.available_width(), 20.0));

    // Navigation
    let nav = [
        (View::Library, "🏠", "Library"),
        (View::Search, "🔍", "Search"),
        (View::Browse, "📂", "Browse"),
        (View::Queue, "📋", "Queue"),
        (View::Lyrics, "🎤", "Lyrics"),
        (View::Settings, "⚙", "Settings"),
        (View::Help, "❓", "Help"),
    ];

    for (view, icon, label) in &nav {
        if theme::nav_item(ui, icon, label, *current_view == *view).clicked() {
            action = Action::Navigate(view.clone());
        }
    }

    ui.allocate_space(egui::vec2(ui.available_width(), 12.0));
    theme::divider_line(ui);

    // Playlists
    theme::section_header(ui, "PLAYLISTS");

    // New Playlist button
    ui.add_space(4.0);
    let btn_rect = ui
        .allocate_exact_size(egui::vec2(ui.available_width(), 36.0), egui::Sense::click())
        .0;
    let btn_resp = ui.allocate_rect(btn_rect, egui::Sense::click());
    let btn_bg = if btn_resp.hovered() {
        theme::bg_hover()
    } else {
        theme::bg_dark()
    };
    ui.painter()
        .rect_filled(btn_rect, egui::CornerRadius::same(6), btn_bg);
    ui.painter().rect_stroke(
        btn_rect,
        egui::CornerRadius::same(6),
        egui::Stroke::new(1.0, theme::text_muted()),
        egui::StrokeKind::Outside,
    );
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
    if btn_resp.clicked() {
        action = Action::OpenCreatePlaylist;
    }
    ui.add_space(4.0);

    egui::ScrollArea::vertical()
        .id_salt("sidebar_playlists")
        .max_height(ui.available_height() * 0.45)
        .show(ui, |ui| {
            let data = state.data.read();
            for (i, item) in data.user_data.playlists.iter().enumerate() {
                match item {
                    state::PlaylistFolderItem::Playlist(playlist) => {
                        if theme::list_item(ui, &playlist.name, &playlist.owner.0, false).clicked() {
                            action = Action::OpenPlaylist(i);
                        }
                    }
                    state::PlaylistFolderItem::Folder(folder) => {
                        let rect = ui.allocate_space(egui::vec2(ui.available_width(), 28.0)).1;
                        ui.painter().text(
                            rect.left_center() + egui::vec2(16.0, 0.0),
                            egui::Align2::LEFT_CENTER,
                            format!("📁 {}", folder.name),
                            egui::FontId::proportional(13.0),
                            theme::text_dim(),
                        );
                    }
                }
            }
        });

    ui.allocate_space(egui::vec2(ui.available_width(), 8.0));
    theme::divider_line(ui);

    // Albums
    theme::section_header(ui, "ALBUMS");

    egui::ScrollArea::vertical()
        .id_salt("sidebar_albums")
        .max_height(ui.available_height() * 0.3)
        .show(ui, |ui| {
            let data = state.data.read();
            for (i, album) in data.user_data.saved_albums.iter().enumerate() {
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
        });

    ui.allocate_space(egui::vec2(ui.available_width(), 8.0));
    theme::divider_line(ui);

    // Artists
    theme::section_header(ui, "ARTISTS");

    egui::ScrollArea::vertical()
        .id_salt("sidebar_artists")
        .show(ui, |ui| {
            let data = state.data.read();
            for artist in data.user_data.followed_artists.iter() {
                if theme::list_item(ui, &artist.name, "", false).clicked() {
                    action = Action::OpenArtist(artist.clone());
                }
            }
        });

    ui.allocate_space(egui::vec2(ui.available_width(), 8.0));
    theme::divider_line(ui);

    // Keyboard hints
    ui.add_space(4.0);
    let hints: &[(&str, &str)] = &[
        ("Space", "Play/Pause"),
        ("j / k", "Navigate"),
        ("gg / G", "First / Last"),
        ("Enter", "Play selected"),
        ("?  ", "Help"),
        ("Esc", "Back"),
        ("Ctrl+↑↓", "Volume"),
        ("l  ", "Lyrics"),
        ("z  ", "Queue"),
        ("/  ", "Search"),
    ];
    for (key, desc) in hints {
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            ui.label(
                egui::RichText::new(*key)
                    .size(10.0)
                    .color(theme::green())
                    .monospace(),
            );
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(*desc)
                    .size(10.0)
                    .color(theme::text_hint()),
            );
        });
    }

    action
}
