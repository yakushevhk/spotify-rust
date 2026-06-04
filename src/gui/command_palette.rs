use eframe::egui;

use crate::config::keymap::default_keybindings;
use crate::gui::theme;
use crate::key::{CommandCategory, CommandId};

const MAX_RECENT: usize = 10;

#[derive(Clone)]
pub struct PaletteEntry {
    pub command_id: CommandId,
    pub description: String,
    pub category: CommandCategory,
    pub key_display: String,
}

pub struct CommandPalette {
    query: String,
    selected: usize,
    entries: Vec<PaletteEntry>,
    recent: Vec<CommandId>,
}

impl CommandPalette {
    pub fn new() -> Self {
        let entries = Self::build_entries();
        Self {
            query: String::new(),
            selected: 0,
            entries,
            recent: Vec::new(),
        }
    }

    fn build_entries() -> Vec<PaletteEntry> {
        default_keybindings()
            .iter()
            .map(|b| PaletteEntry {
                command_id: b.command.clone(),
                description: b.description.to_string(),
                category: b.category.clone(),
                key_display: Self::format_keybindings(&b.keybindings),
            })
            .collect()
    }

    fn format_keybindings(keybindings: &[crate::key::KeyBinding]) -> String {
        keybindings
            .iter()
            .map(|kb| kb.display_string())
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn filtered_entries(&self) -> Vec<PaletteEntry> {
        if self.query.is_empty() {
            return self.entries.clone();
        }
        let q = self.query.to_lowercase();
        let mut scored: Vec<(PaletteEntry, i32)> = self
            .entries
            .iter()
            .filter_map(|e| {
                let desc = e.description.to_lowercase();
                let cat = e.category.display_name().to_lowercase();
                let key = e.key_display.to_lowercase();
                if desc.contains(&q) || cat.contains(&q) || key.contains(&q) {
                    let score = if desc.starts_with(&q) {
                        100
                    } else if desc.contains(&q) {
                        50
                    } else {
                        10
                    };
                    Some((e.clone(), score))
                } else {
                    None
                }
            })
            .collect();
        scored.sort_by(|a, b| b.1.cmp(&a.1));
        scored.into_iter().map(|(e, _)| e).collect()
    }

    pub fn record_usage(&mut self, cmd: &CommandId) {
        self.recent.retain(|c| c.0 != cmd.0);
        self.recent.insert(0, cmd.clone());
        if self.recent.len() > MAX_RECENT {
            self.recent.truncate(MAX_RECENT);
        }
    }

    pub fn open(&mut self) {
        self.query.clear();
        self.selected = 0;
    }

    pub fn render(
        &mut self,
        ctx: &egui::Context,
    ) -> Option<CommandId> {
        let mut result: Option<CommandId> = None;

        // Dark overlay
        let screen = ctx.screen_rect();
        let overlay_id = egui::Id::new("cmd_palette_overlay");
        egui::Area::new(overlay_id)
            .order(egui::Order::Foreground)
            .fixed_pos(screen.min)
            .interactable(true)
            .show(ctx, |ui| {
                let (overlay_rect, _) = ui.allocate_exact_size(screen.size(), egui::Sense::click());
                ui.painter().rect_filled(
                    overlay_rect,
                    0,
                    egui::Color32::from_black_alpha(120),
                );
            });

        // Palette dimensions
        let palette_width = 520.0_f32.min(screen.width() * 0.85);
        let palette_max_height = screen.height() * 0.6;
        let palette_x = screen.center().x - palette_width / 2.0;
        let palette_y = screen.center().y - palette_max_height / 2.0;

        let mut close = false;

        egui::Area::new(egui::Id::new("command_palette"))
            .order(egui::Order::Foreground)
            .fixed_pos(egui::pos2(palette_x, palette_y))
            .interactable(true)
            .show(ctx, |ui| {
                let frame = theme::glass_frame();

                frame.show(ui, |ui| {
                    ui.set_min_width(palette_width);
                    ui.set_max_height(palette_max_height);

                    // Search input
                    let input_margin = 12.0;
                    ui.add_space(input_margin);
                    ui.horizontal(|ui| {
                        ui.add_space(input_margin);
                        ui.label(
                            egui::RichText::new("🔍")
                                .size(16.0)
                                .color(theme::text_dim()),
                        );
                        ui.add_space(6.0);
                        let input = egui::TextEdit::singleline(&mut self.query)
                            .desired_width(f32::INFINITY)
                            .hint_text("Type a command...")
                            .font(egui::FontId::proportional(15.0))
                            .margin(egui::Margin::symmetric(8, 8))
                            .background_color(egui::Color32::TRANSPARENT);
                        let response = ui.add(input);
                        response.request_focus();
                    });
                    ui.add_space(8.0);

                    // Divider
                    let div_rect = ui.allocate_space(egui::vec2(ui.available_width(), 1.0)).1;
                    ui.painter()
                        .rect_filled(div_rect, 0, theme::divider());

                    // Filtered entries
                    let filtered = self.filtered_entries();

                    // Clamp selection
                    if filtered.is_empty() {
                        self.selected = 0;
                    } else if self.selected >= filtered.len() {
                        self.selected = filtered.len() - 1;
                    }

                    let item_height = 40.0;
                    let list_max = palette_max_height - 80.0;

                    egui::ScrollArea::vertical()
                        .id_salt("cmd_palette_list")
                        .max_height(list_max)
                        .show(ui, |ui| {
                            for (i, entry) in filtered.iter().enumerate() {
                                let is_selected = i == self.selected;
                                let item_rect = ui
                                    .allocate_exact_size(
                                        egui::vec2(ui.available_width() - 8.0, item_height),
                                        egui::Sense::click(),
                                    )
                                    .0;
                                let item_resp =
                                    ui.allocate_rect(item_rect, egui::Sense::click());

                                // Background
                                let bg = if is_selected {
                                    theme::accent()
                                } else if item_resp.hovered() {
                                    theme::bg_hover()
                                } else {
                                    egui::Color32::TRANSPARENT
                                };
                                ui.painter().rect_filled(
                                    item_rect,
                                    egui::CornerRadius::same(4),
                                    bg,
                                );

                                // Text color
                                let text_color = if is_selected {
                                    theme::bg_black()
                                } else if item_resp.hovered() {
                                    theme::text_primary()
                                } else {
                                    theme::text_secondary()
                                };
                                let dim_color = if is_selected {
                                    theme::with_alpha(theme::bg_black(), 160)
                                } else {
                                    theme::text_dim()
                                };

                                // Category badge
                                let cat_icon = match entry.category {
                                    CommandCategory::Navigation => "↕",
                                    CommandCategory::Sorting => "⇅",
                                    CommandCategory::Playback => "♫",
                                    CommandCategory::Actions => "⚡",
                                    CommandCategory::Pages => "📄",
                                    CommandCategory::Other => "⚙",
                                };
                                ui.painter().text(
                                    item_rect.left_center() + egui::vec2(10.0, 0.0),
                                    egui::Align2::LEFT_CENTER,
                                    cat_icon,
                                    egui::FontId::proportional(13.0),
                                    dim_color,
                                );

                                // Description
                                ui.painter().text(
                                    item_rect.left_center() + egui::vec2(30.0, 0.0),
                                    egui::Align2::LEFT_CENTER,
                                    &entry.description,
                                    egui::FontId::proportional(13.0),
                                    text_color,
                                );

                                // Key shortcut on the right
                                if !entry.key_display.is_empty() {
                                    let key_text = format!("[{}]", entry.key_display);
                                    let galley = ui.painter().layout_no_wrap(
                                        key_text.clone(),
                                        egui::FontId::proportional(11.0),
                                        dim_color,
                                    );
                                    let key_width = galley.size().x;
                                    ui.painter().galley(
                                        item_rect.right_center() + egui::vec2(-key_width - 10.0, -galley.size().y / 2.0),
                                        galley,
                                        dim_color,
                                    );
                                }

                                if item_resp.clicked() {
                                    result = Some(entry.command_id.clone());
                                    close = true;
                                }

                                ui.add_space(2.0);
                            }
                        });
                });

                // Keyboard handling
                ctx.input(|i| {
                    // Escape
                    if i.key_pressed(egui::Key::Escape) {
                        close = true;
                    }

                    // Arrow down / Ctrl+N
                    if i.key_pressed(egui::Key::ArrowDown)
                        || (i.modifiers.ctrl && i.key_pressed(egui::Key::N))
                    {
                        let filtered = self.filtered_entries();
                        if !filtered.is_empty() {
                            self.selected = (self.selected + 1) % filtered.len();
                        }
                    }

                    // Arrow up / Ctrl+P
                    if i.key_pressed(egui::Key::ArrowUp)
                        || (i.modifiers.ctrl && i.key_pressed(egui::Key::P))
                    {
                        let filtered = self.filtered_entries();
                        if !filtered.is_empty() {
                            self.selected = if self.selected == 0 {
                                filtered.len() - 1
                            } else {
                                self.selected - 1
                            };
                        }
                    }

                    // Enter
                    if i.key_pressed(egui::Key::Enter) {
                        let filtered = self.filtered_entries();
                        if let Some(entry) = filtered.get(self.selected) {
                            result = Some(entry.command_id.clone());
                            close = true;
                        }
                    }

                    // Home
                    if i.key_pressed(egui::Key::Home) {
                        self.selected = 0;
                    }

                    // End
                    if i.key_pressed(egui::Key::End) {
                        let filtered = self.filtered_entries();
                        if !filtered.is_empty() {
                            self.selected = filtered.len() - 1;
                        }
                    }
                });
            });

        if close {
            return result;
        }

        result
    }
}
