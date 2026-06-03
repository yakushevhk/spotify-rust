use eframe::egui;

pub const BG_BLACK: egui::Color32 = egui::Color32::from_rgb(0, 0, 0);
pub const BG_DARK: egui::Color32 = egui::Color32::from_rgb(12, 12, 12);
pub const BG_CARD: egui::Color32 = egui::Color32::from_rgb(18, 18, 18);
pub const BG_HOVER: egui::Color32 = egui::Color32::from_rgb(28, 28, 28);
pub const BG_ACTIVE: egui::Color32 = egui::Color32::from_rgb(38, 38, 38);
pub const BG_ELEVATED: egui::Color32 = egui::Color32::from_rgb(24, 24, 24);
pub const BG_INPUT: egui::Color32 = egui::Color32::from_rgb(35, 35, 35);

pub const GREEN: egui::Color32 = egui::Color32::from_rgb(30, 215, 96);
pub const GREEN_HOVER: egui::Color32 = egui::Color32::from_rgb(45, 225, 110);
pub const GREEN_DARK: egui::Color32 = egui::Color32::from_rgb(25, 180, 80);

pub const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(255, 255, 255);
pub const TEXT_SECONDARY: egui::Color32 = egui::Color32::from_rgb(179, 179, 179);
pub const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(115, 115, 115);
pub const TEXT_MUTED: egui::Color32 = egui::Color32::from_rgb(77, 77, 77);

pub const BORDER: egui::Color32 = egui::Color32::from_rgb(33, 33, 33);
pub const DIVIDER: egui::Color32 = egui::Color32::from_rgb(40, 40, 40);

pub const SIDEBAR_WIDTH: f32 = 280.0;
pub const PLAYBACK_BAR_HEIGHT: f32 = 100.0;
pub const ICON_SIZE: f32 = 24.0;

pub const PLAYBACK_ART_SIZE: f32 = 80.0;
pub const TRACK_THUMB_SIZE: f32 = 36.0;
pub const ART_CORNER_RADIUS: f32 = 4.0;

fn wv(bg: egui::Color32, fg: egui::Color32) -> egui::style::WidgetVisuals {
    egui::style::WidgetVisuals {
        bg_fill: bg,
        weak_bg_fill: bg,
        bg_stroke: egui::Stroke::NONE,
        corner_radius: egui::CornerRadius::same(6),
        fg_stroke: egui::Stroke::new(1.0, fg),
        expansion: 0.0,
    }
}

pub fn setup_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    style.visuals = egui::Visuals {
        dark_mode: true,
        override_text_color: Some(TEXT_PRIMARY),
        widgets: egui::style::Widgets {
            noninteractive: wv(BG_DARK, TEXT_PRIMARY),
            inactive: wv(BG_CARD, TEXT_SECONDARY),
            hovered: wv(BG_HOVER, TEXT_PRIMARY),
            active: wv(BG_ACTIVE, TEXT_PRIMARY),
            open: wv(BG_ACTIVE, TEXT_PRIMARY),
        },
        selection: egui::style::Selection {
            bg_fill: GREEN,
            stroke: egui::Stroke::new(1.0, TEXT_PRIMARY),
        },
        hyperlink_color: GREEN,
        faint_bg_color: BG_CARD,
        extreme_bg_color: BG_BLACK,
        code_bg_color: BG_CARD,
        warn_fg_color: egui::Color32::from_rgb(255, 200, 50),
        error_fg_color: egui::Color32::from_rgb(255, 80, 80),
        window_fill: BG_DARK,
        panel_fill: BG_DARK,
        window_stroke: egui::Stroke::new(1.0, BORDER),
        ..style.visuals
    };

    style.spacing.item_spacing = egui::vec2(0.0, 0.0);
    style.spacing.window_margin = egui::Margin::same(0);

    ctx.set_style(style);
}

pub fn primary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let desired_size = egui::vec2(ui.available_width().min(160.0), 36.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    let bg = if response.hovered() { GREEN_HOVER } else { GREEN };
    ui.painter().rect_filled(rect, egui::CornerRadius::same(18), bg);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::proportional(14.0),
        BG_BLACK,
    );
    response
}

pub fn secondary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let desired_size = egui::vec2(ui.available_width().min(160.0), 36.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    let (bg, bc) = if response.hovered() {
        (BG_HOVER, TEXT_SECONDARY)
    } else {
        (BG_CARD, DIVIDER)
    };
    ui.painter().rect_filled(rect, egui::CornerRadius::same(18), bg);
    ui.painter().rect_stroke(rect, egui::CornerRadius::same(18), egui::Stroke::new(1.0, bc), egui::StrokeKind::Outside);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::proportional(14.0),
        TEXT_PRIMARY,
    );
    response
}

pub fn icon_button(ui: &mut egui::Ui, text: &str, size: f32, active: bool) -> egui::Response {
    let desired_size = egui::vec2(size, size);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    let color = if active {
        GREEN
    } else if response.hovered() {
        TEXT_PRIMARY
    } else {
        TEXT_SECONDARY
    };
    let bg = if response.hovered() && !active {
        BG_HOVER
    } else {
        egui::Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, egui::CornerRadius::same((size / 2.0) as u8), bg);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::proportional(size * 0.55),
        color,
    );
    response
}

pub fn play_pause_button(ui: &mut egui::Ui, is_playing: bool) -> egui::Response {
    let size = 42.0;
    let desired_size = egui::vec2(size, size);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    let bg = if response.hovered() { GREEN_HOVER } else { GREEN };
    ui.painter().rect_filled(rect, egui::CornerRadius::same(21), bg);
    let symbol = if is_playing { "⏸" } else { "▶" };
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        symbol,
        egui::FontId::proportional(18.0),
        BG_BLACK,
    );
    response
}

pub fn nav_item(ui: &mut egui::Ui, icon: &str, label: &str, is_selected: bool) -> egui::Response {
    let height = 40.0;
    let desired_size = egui::vec2(ui.available_width(), height);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    let bg = if is_selected {
        BG_HOVER
    } else if response.hovered() {
        BG_CARD
    } else {
        egui::Color32::TRANSPARENT
    };
    let text_color = if is_selected || response.hovered() {
        TEXT_PRIMARY
    } else {
        TEXT_SECONDARY
    };
    let icon_color = if is_selected { GREEN } else { text_color };
    ui.painter().rect_filled(rect, egui::CornerRadius::same(6), bg);
    if is_selected {
        ui.painter().rect_filled(
            egui::Rect::from_min_size(rect.min, egui::vec2(3.0, rect.height())),
            egui::CornerRadius::same(1),
            GREEN,
        );
    }
    ui.painter().text(
        rect.left_center() + egui::vec2(16.0, 0.0),
        egui::Align2::LEFT_CENTER,
        icon,
        egui::FontId::proportional(18.0),
        icon_color,
    );
    ui.painter().text(
        rect.left_center() + egui::vec2(48.0, 0.0),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(14.0),
        text_color,
    );
    response
}

pub fn list_item(ui: &mut egui::Ui, label: &str, sublabel: &str, is_selected: bool) -> egui::Response {
    let height = 44.0;
    let desired_size = egui::vec2(ui.available_width(), height);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    let bg = if is_selected {
        BG_HOVER
    } else if response.hovered() {
        BG_CARD
    } else {
        egui::Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, egui::CornerRadius::same(4), bg);
    let text_color = if is_selected || response.hovered() {
        TEXT_PRIMARY
    } else {
        TEXT_SECONDARY
    };
    ui.painter().text(
        rect.left_center() + egui::vec2(16.0, -6.0),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(14.0),
        text_color,
    );
    if !sublabel.is_empty() {
        ui.painter().text(
            rect.left_center() + egui::vec2(16.0, 10.0),
            egui::Align2::LEFT_CENTER,
            sublabel,
            egui::FontId::proportional(11.0),
            TEXT_DIM,
        );
    }
    response
}

pub fn section_header(ui: &mut egui::Ui, label: &str) {
    let height = 32.0;
    ui.allocate_space(egui::vec2(ui.available_width(), 4.0));
    let rect = ui.allocate_space(egui::vec2(ui.available_width(), height)).1;
    ui.painter().text(
        rect.left_center() + egui::vec2(16.0, 0.0),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(12.0),
        TEXT_DIM,
    );
}

pub fn divider(ui: &mut egui::Ui) {
    ui.allocate_space(egui::vec2(ui.available_width(), 8.0));
    let rect = ui.allocate_space(egui::vec2(ui.available_width(), 1.0)).1;
    ui.painter().rect_filled(rect, egui::CornerRadius::ZERO, DIVIDER);
    ui.allocate_space(egui::vec2(ui.available_width(), 8.0));
}

pub fn card<R>(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> egui::InnerResponse<R> {
    egui::Frame::new()
        .fill(BG_CARD)
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(egui::Margin::same(16))
        .show(ui, add_contents)
}

pub fn page_title(ui: &mut egui::Ui, title: &str) {
    ui.add_space(24.0);
    ui.horizontal(|ui| {
        ui.add_space(24.0);
        ui.label(egui::RichText::new(title).size(28.0).strong().color(TEXT_PRIMARY));
    });
    ui.add_space(20.0);
}

pub const LYRICS_CURRENT: egui::Color32 = egui::Color32::from_rgb(29, 185, 84);
pub const LYRICS_PLAYED: egui::Color32 = egui::Color32::from_rgb(85, 85, 85);
pub const LYRICS_UPCOMING: egui::Color32 = egui::Color32::from_rgb(204, 204, 204);
pub const LYRICS_BG: egui::Color32 = egui::Color32::from_rgb(10, 10, 10);

pub fn format_duration_secs(secs: u64) -> String {
    let mins = secs / 60;
    let s = secs % 60;
    format!("{mins}:{s:02}")
}
