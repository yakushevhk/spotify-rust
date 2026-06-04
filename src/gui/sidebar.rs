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
                .color(theme::GREEN),
        );
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new("Spotify Player")
                .size(18.0)
                .strong()
                .color(theme::TEXT_PRIMARY),
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
    ];

    for (view, icon, label) in &nav {
        if theme::nav_item(ui, icon, label, *current_view == *view).clicked() {
            action = Action::Navigate(view.clone());
        }
    }

    ui.allocate_space(egui::vec2(ui.available_width(), 12.0));
    theme::divider(ui);

    // Playlists
    theme::section_header(ui, "PLAYLISTS");

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
                            theme::TEXT_DIM,
                        );
                    }
                }
            }
        });

    ui.allocate_space(egui::vec2(ui.available_width(), 8.0));
    theme::divider(ui);

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
    theme::divider(ui);

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

    action
}
