use eframe::egui;
use parking_lot::RwLock;
use std::sync::OnceLock;

use crate::config::theme::Palette;

static PALETTE: OnceLock<RwLock<GuiPalette>> = OnceLock::new();

#[derive(Clone, Debug)]
pub struct GuiPalette {
    pub background: egui::Color32,
    pub foreground: egui::Color32,
    pub accent: egui::Color32,
    pub accent_hover: egui::Color32,
    pub accent_dark: egui::Color32,

    pub bg_dark: egui::Color32,
    pub bg_card: egui::Color32,
    pub bg_hover: egui::Color32,
    pub bg_active: egui::Color32,
    pub bg_elevated: egui::Color32,
    pub bg_input: egui::Color32,
    pub bg_selected: egui::Color32,

    pub text_primary: egui::Color32,
    pub text_secondary: egui::Color32,
    pub text_dim: egui::Color32,
    pub text_muted: egui::Color32,
    pub text_hint: egui::Color32,

    pub border: egui::Color32,
    pub divider: egui::Color32,

    pub success: egui::Color32,
    pub error: egui::Color32,
    pub warning: egui::Color32,

    pub lyrics_current: egui::Color32,
    pub lyrics_played: egui::Color32,
    pub lyrics_upcoming: egui::Color32,
    pub lyrics_bg: egui::Color32,
}

pub struct BuiltInTheme {
    pub name: &'static str,
    pub palette: Palette,
}

pub fn built_in_themes() -> Vec<BuiltInTheme> {
    vec![
        BuiltInTheme {
            name: "Dracula",
            palette: Palette {
                background: "#000000".into(),
                foreground: "#f8f8f2".into(),
                accent: "#bd93f9".into(),
                accent_hover: "#caa8fa".into(),
                accent_dark: "#9b7fd4".into(),
                bg_dark: "#050505".into(),
                bg_card: "#080808".into(),
                bg_hover: "#0a0a0a".into(),
                bg_active: "#0c0c0c".into(),
                bg_elevated: "#060606".into(),
                bg_input: "#0a0a0a".into(),
                bg_selected: "#0a0a0a".into(),
                text_primary: "#f8f8f2".into(),
                text_secondary: "#bfbfbf".into(),
                text_dim: "#737686".into(),
                text_muted: "#4d5062".into(),
                text_hint: "#6272a4".into(),
                border: "#0c0c0c".into(),
                divider: "#0f0f0f".into(),
                success: "#50fa7b".into(),
                error: "#ff5555".into(),
                warning: "#f1fa8c".into(),
                lyrics_current: "#bd93f9".into(),
                lyrics_played: "#6272a4".into(),
                lyrics_upcoming: "#f8f8f2".into(),
                lyrics_bg: "#000000".into(),
            },
        },
        BuiltInTheme {
            name: "Spotify",
            palette: Palette {
                background: "#000000".into(),
                foreground: "#ffffff".into(),
                accent: "#1ed760".into(),
                accent_hover: "#2de16e".into(),
                accent_dark: "#19b450".into(),
                bg_dark: "#000000".into(),
                bg_card: "#000000".into(),
                bg_hover: "#0a0a0a".into(),
                bg_active: "#0c0c0c".into(),
                bg_elevated: "#000000".into(),
                bg_input: "#050505".into(),
                bg_selected: "#080808".into(),
                text_primary: "#ffffff".into(),
                text_secondary: "#b3b3b3".into(),
                text_dim: "#737373".into(),
                text_muted: "#4d4d4d".into(),
                text_hint: "#666666".into(),
                border: "#0a0a0a".into(),
                divider: "#0c0c0c".into(),
                success: "#1ed760".into(),
                error: "#e22134".into(),
                warning: "#ffc832".into(),
                lyrics_current: "#1db954".into(),
                lyrics_played: "#555555".into(),
                lyrics_upcoming: "#cccccc".into(),
                lyrics_bg: "#000000".into(),
            },
        },
        BuiltInTheme {
            name: "Catppuccin Mocha",
            palette: Palette {
                background: "#000000".into(),
                foreground: "#cdd6f4".into(),
                accent: "#cba6f7".into(),
                accent_hover: "#d4b8f8".into(),
                accent_dark: "#a88de0".into(),
                bg_dark: "#050505".into(),
                bg_card: "#080808".into(),
                bg_hover: "#0a0a0a".into(),
                bg_active: "#0c0c0c".into(),
                bg_elevated: "#060606".into(),
                bg_input: "#0a0a0a".into(),
                bg_selected: "#0a0a0a".into(),
                text_primary: "#cdd6f4".into(),
                text_secondary: "#a6adc8".into(),
                text_dim: "#6c7086".into(),
                text_muted: "#45475a".into(),
                text_hint: "#585b70".into(),
                border: "#0c0c0c".into(),
                divider: "#0f0f0f".into(),
                success: "#a6e3a1".into(),
                error: "#f38ba8".into(),
                warning: "#f9e2af".into(),
                lyrics_current: "#cba6f7".into(),
                lyrics_played: "#585b70".into(),
                lyrics_upcoming: "#bac2de".into(),
                lyrics_bg: "#000000".into(),
            },
        },
        BuiltInTheme {
            name: "Gruvbox Dark",
            palette: Palette {
                background: "#000000".into(),
                foreground: "#ebdbb2".into(),
                accent: "#fe8019".into(),
                accent_hover: "#fabd2f".into(),
                accent_dark: "#d65d0e".into(),
                bg_dark: "#050505".into(),
                bg_card: "#080808".into(),
                bg_hover: "#0a0a0a".into(),
                bg_active: "#0c0c0c".into(),
                bg_elevated: "#060606".into(),
                bg_input: "#0c0c0c".into(),
                bg_selected: "#0c0c0c".into(),
                text_primary: "#ebdbb2".into(),
                text_secondary: "#d5c4a1".into(),
                text_dim: "#7c6f64".into(),
                text_muted: "#504945".into(),
                text_hint: "#665c54".into(),
                border: "#0c0c0c".into(),
                divider: "#0f0f0f".into(),
                success: "#b8bb26".into(),
                error: "#fb4934".into(),
                warning: "#fabd2f".into(),
                lyrics_current: "#fe8019".into(),
                lyrics_played: "#665c54".into(),
                lyrics_upcoming: "#d5c4a1".into(),
                lyrics_bg: "#000000".into(),
            },
        },
        BuiltInTheme {
            name: "Nord",
            palette: Palette {
                background: "#000000".into(),
                foreground: "#eceff4".into(),
                accent: "#88c0d0".into(),
                accent_hover: "#8fbcbb".into(),
                accent_dark: "#5e81ac".into(),
                bg_dark: "#050505".into(),
                bg_card: "#080808".into(),
                bg_hover: "#0a0a0a".into(),
                bg_active: "#0c0c0c".into(),
                bg_elevated: "#060606".into(),
                bg_input: "#0a0a0a".into(),
                bg_selected: "#0a0a0a".into(),
                text_primary: "#eceff4".into(),
                text_secondary: "#d8dee9".into(),
                text_dim: "#616e88".into(),
                text_muted: "#4c566a".into(),
                text_hint: "#4c566a".into(),
                border: "#0c0c0c".into(),
                divider: "#0f0f0f".into(),
                success: "#a3be8c".into(),
                error: "#bf616a".into(),
                warning: "#ebcb8b".into(),
                lyrics_current: "#88c0d0".into(),
                lyrics_played: "#4c566a".into(),
                lyrics_upcoming: "#d8dee9".into(),
                lyrics_bg: "#000000".into(),
            },
        },
        BuiltInTheme {
            name: "Solarized Dark",
            palette: Palette {
                background: "#000000".into(),
                foreground: "#839496".into(),
                accent: "#268bd2".into(),
                accent_hover: "#2aa1e3".into(),
                accent_dark: "#1a6da0".into(),
                bg_dark: "#050505".into(),
                bg_card: "#080808".into(),
                bg_hover: "#0a0a0a".into(),
                bg_active: "#0c0c0c".into(),
                bg_elevated: "#060606".into(),
                bg_input: "#0a0a0a".into(),
                bg_selected: "#0a0a0a".into(),
                text_primary: "#93a1a1".into(),
                text_secondary: "#839496".into(),
                text_dim: "#586e75".into(),
                text_muted: "#2c4a52".into(),
                text_hint: "#657b83".into(),
                border: "#0c0c0c".into(),
                divider: "#0f0f0f".into(),
                success: "#859900".into(),
                error: "#dc322f".into(),
                warning: "#b58900".into(),
                lyrics_current: "#268bd2".into(),
                lyrics_played: "#586e75".into(),
                lyrics_upcoming: "#93a1a1".into(),
                lyrics_bg: "#000000".into(),
            },
        },
        BuiltInTheme {
            name: "Tokyo Night",
            palette: Palette {
                background: "#000000".into(),
                foreground: "#c0caf5".into(),
                accent: "#7aa2f7".into(),
                accent_hover: "#89b4fa".into(),
                accent_dark: "#5a84d4".into(),
                bg_dark: "#050505".into(),
                bg_card: "#080808".into(),
                bg_hover: "#0a0a0a".into(),
                bg_active: "#0c0c0c".into(),
                bg_elevated: "#060606".into(),
                bg_input: "#0a0a0a".into(),
                bg_selected: "#0a0a0a".into(),
                text_primary: "#c0caf5".into(),
                text_secondary: "#a9b1d6".into(),
                text_dim: "#565f89".into(),
                text_muted: "#3b3f57".into(),
                text_hint: "#444b6a".into(),
                border: "#0c0c0c".into(),
                divider: "#0f0f0f".into(),
                success: "#9ece6a".into(),
                error: "#f7768e".into(),
                warning: "#e0af68".into(),
                lyrics_current: "#7aa2f7".into(),
                lyrics_played: "#444b6a".into(),
                lyrics_upcoming: "#a9b1d6".into(),
                lyrics_bg: "#000000".into(),
            },
        },
        BuiltInTheme {
            name: "One Dark",
            palette: Palette {
                background: "#000000".into(),
                foreground: "#abb2bf".into(),
                accent: "#61afef".into(),
                accent_hover: "#74b9f0".into(),
                accent_dark: "#4d99d4".into(),
                bg_dark: "#050505".into(),
                bg_card: "#080808".into(),
                bg_hover: "#0a0a0a".into(),
                bg_active: "#0c0c0c".into(),
                bg_elevated: "#060606".into(),
                bg_input: "#0a0a0a".into(),
                bg_selected: "#0a0a0a".into(),
                text_primary: "#abb2bf".into(),
                text_secondary: "#9da5b4".into(),
                text_dim: "#5c6370".into(),
                text_muted: "#3e4452".into(),
                text_hint: "#4b5263".into(),
                border: "#0c0c0c".into(),
                divider: "#0f0f0f".into(),
                success: "#98c379".into(),
                error: "#e06c75".into(),
                warning: "#e5c07b".into(),
                lyrics_current: "#61afef".into(),
                lyrics_played: "#4b5263".into(),
                lyrics_upcoming: "#abb2bf".into(),
                lyrics_bg: "#000000".into(),
            },
        },
    ]
}

fn parse_hex_color(hex: &str) -> egui::Color32 {
    let hex = hex.trim().trim_start_matches('#');
    if hex.len() == 6 {
        if let Ok(val) = u32::from_str_radix(hex, 16) {
            let r = ((val >> 16) & 0xFF) as u8;
            let g = ((val >> 8) & 0xFF) as u8;
            let b = (val & 0xFF) as u8;
            return egui::Color32::from_rgb(r, g, b);
        }
    }
    tracing::warn!("Failed to parse hex color: '{hex}', using fallback magenta");
    egui::Color32::from_rgb(255, 0, 255)
}

impl GuiPalette {
    pub fn from_config_palette(palette: &Palette) -> Self {
        Self {
            background: parse_hex_color(&palette.background),
            foreground: parse_hex_color(&palette.foreground),
            accent: parse_hex_color(&palette.accent),
            accent_hover: parse_hex_color(&palette.accent_hover),
            accent_dark: parse_hex_color(&palette.accent_dark),
            bg_dark: parse_hex_color(&palette.bg_dark),
            bg_card: parse_hex_color(&palette.bg_card),
            bg_hover: parse_hex_color(&palette.bg_hover),
            bg_active: parse_hex_color(&palette.bg_active),
            bg_elevated: parse_hex_color(&palette.bg_elevated),
            bg_input: parse_hex_color(&palette.bg_input),
            bg_selected: parse_hex_color(&palette.bg_selected),
            text_primary: parse_hex_color(&palette.text_primary),
            text_secondary: parse_hex_color(&palette.text_secondary),
            text_dim: parse_hex_color(&palette.text_dim),
            text_muted: parse_hex_color(&palette.text_muted),
            text_hint: parse_hex_color(&palette.text_hint),
            border: parse_hex_color(&palette.border),
            divider: parse_hex_color(&palette.divider),
            success: parse_hex_color(&palette.success),
            error: parse_hex_color(&palette.error),
            warning: parse_hex_color(&palette.warning),
            lyrics_current: parse_hex_color(&palette.lyrics_current),
            lyrics_played: parse_hex_color(&palette.lyrics_played),
            lyrics_upcoming: parse_hex_color(&palette.lyrics_upcoming),
            lyrics_bg: parse_hex_color(&palette.lyrics_bg),
        }
    }
}

pub fn get_palette() -> parking_lot::RwLockReadGuard<'static, GuiPalette> {
    PALETTE.get_or_init(|| {
        let default = built_in_themes();
        let p = &default[0].palette;
        RwLock::new(GuiPalette::from_config_palette(p))
    }).read()
}

pub fn set_palette_from_config(palette: &Palette) {
    let gui = GuiPalette::from_config_palette(palette);
    let lock = PALETTE.get_or_init(|| RwLock::new(gui.clone()));
    *lock.write() = gui;
}

pub fn set_palette(name: &str) {
    let themes = built_in_themes();
    if let Some(builtin) = themes.iter().find(|t| t.name.eq_ignore_ascii_case(name)) {
        set_palette_from_config(&builtin.palette);
    }
}

// === Layout constants ===
pub const SIDEBAR_WIDTH: f32 = 280.0;
pub const PLAYBACK_BAR_HEIGHT: f32 = 100.0;
pub const ICON_SIZE: f32 = 24.0;
pub const PLAYBACK_ART_SIZE: f32 = 80.0;
pub const TRACK_THUMB_SIZE: f32 = 36.0;
pub const ART_CORNER_RADIUS: f32 = 4.0;

pub const RADIUS_SMALL: u8 = 4;
pub const RADIUS_MEDIUM: u8 = 8;
pub const RADIUS_LARGE: u8 = 12;

// === Color accessor functions ===
// These replace the old constants. Every call reads from the global palette.

#[inline] pub fn background() -> egui::Color32 { get_palette().background }
#[inline] pub fn foreground() -> egui::Color32 { get_palette().foreground }
#[inline] pub fn accent() -> egui::Color32 { get_palette().accent }
#[inline] pub fn accent_hover() -> egui::Color32 { get_palette().accent_hover }
#[inline] pub fn accent_dark() -> egui::Color32 { get_palette().accent_dark }

// Short aliases matching old constant names for minimal diff
#[inline] pub fn bg_black() -> egui::Color32 { get_palette().background }
#[inline] pub fn bg_dark() -> egui::Color32 { get_palette().bg_dark }
#[inline] pub fn bg_card() -> egui::Color32 { get_palette().bg_card }
#[inline] pub fn bg_hover() -> egui::Color32 { get_palette().bg_hover }
#[inline] pub fn bg_active() -> egui::Color32 { get_palette().bg_active }
#[inline] pub fn bg_elevated() -> egui::Color32 { get_palette().bg_elevated }
#[inline] pub fn bg_input() -> egui::Color32 { get_palette().bg_input }
#[inline] pub fn bg_selected() -> egui::Color32 { get_palette().bg_selected }

#[inline] pub fn green() -> egui::Color32 { get_palette().accent }
#[inline] pub fn green_hover() -> egui::Color32 { get_palette().accent_hover }
#[inline] pub fn green_dark() -> egui::Color32 { get_palette().accent_dark }

#[inline] pub fn text_primary() -> egui::Color32 { get_palette().text_primary }
#[inline] pub fn text_secondary() -> egui::Color32 { get_palette().text_secondary }
#[inline] pub fn text_dim() -> egui::Color32 { get_palette().text_dim }
#[inline] pub fn text_muted() -> egui::Color32 { get_palette().text_muted }
#[inline] pub fn text_hint() -> egui::Color32 { get_palette().text_hint }

#[inline] pub fn border() -> egui::Color32 { get_palette().border }
#[inline] pub fn divider() -> egui::Color32 { get_palette().divider }

#[inline] pub fn success() -> egui::Color32 { get_palette().success }
#[inline] pub fn error_color() -> egui::Color32 { get_palette().error }
#[inline] pub fn warning() -> egui::Color32 { get_palette().warning }

#[inline] pub fn lyrics_current() -> egui::Color32 { get_palette().lyrics_current }
#[inline] pub fn lyrics_played() -> egui::Color32 { get_palette().lyrics_played }
#[inline] pub fn lyrics_upcoming() -> egui::Color32 { get_palette().lyrics_upcoming }
#[inline] pub fn lyrics_bg() -> egui::Color32 { get_palette().lyrics_bg }

fn wv(bg: egui::Color32, fg: egui::Color32) -> egui::style::WidgetVisuals {
    egui::style::WidgetVisuals {
        bg_fill: bg,
        weak_bg_fill: bg,
        bg_stroke: egui::Stroke::NONE,
        corner_radius: egui::CornerRadius::same(RADIUS_MEDIUM),
        fg_stroke: egui::Stroke::new(1.0, fg),
        expansion: 0.0,
    }
}

pub fn setup_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    let p = get_palette();

    style.visuals = egui::Visuals {
        dark_mode: true,
        override_text_color: Some(p.text_primary),
        widgets: egui::style::Widgets {
            noninteractive: wv(p.background, p.text_primary),
            inactive: wv(p.background, p.text_secondary),
            hovered: wv(p.bg_hover, p.text_primary),
            active: wv(p.bg_active, p.text_primary),
            open: wv(p.bg_active, p.text_primary),
        },
        selection: egui::style::Selection {
            bg_fill: p.accent,
            stroke: egui::Stroke::new(1.0, p.text_primary),
        },
        hyperlink_color: p.accent,
        faint_bg_color: p.background,
        extreme_bg_color: p.background,
        code_bg_color: p.background,
        warn_fg_color: p.warning,
        error_fg_color: p.error,
        window_fill: p.background,
        panel_fill: p.background,
        window_stroke: egui::Stroke::new(1.0, p.border),
        ..style.visuals
    };

    drop(p);

    style.spacing.item_spacing = egui::vec2(4.0, 4.0);
    style.spacing.window_margin = egui::Margin::same(0);

    ctx.set_style(style);
}

pub fn primary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let desired_size = egui::vec2(ui.available_width().min(160.0), 36.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    let bg = if response.hovered() { green_hover() } else { green() };
    ui.painter().rect_filled(rect, egui::CornerRadius::same(18), bg);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::proportional(14.0),
        bg_black(),
    );
    response
}

pub fn secondary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let desired_size = egui::vec2(ui.available_width().min(160.0), 36.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    let (bg, bc) = if response.hovered() {
        (bg_hover(), text_secondary())
    } else {
        (bg_card(), divider())
    };
    ui.painter().rect_filled(rect, egui::CornerRadius::same(18), bg);
    ui.painter().rect_stroke(rect, egui::CornerRadius::same(18), egui::Stroke::new(1.0, bc), egui::StrokeKind::Outside);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::proportional(14.0),
        text_primary(),
    );
    response
}

pub fn icon_button(ui: &mut egui::Ui, text: &str, size: f32, active: bool) -> egui::Response {
    let desired_size = egui::vec2(size, size);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    let color = if active {
        green()
    } else if response.hovered() {
        text_primary()
    } else {
        text_secondary()
    };
    let bg = if response.hovered() && !active {
        bg_hover()
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
    let bg = if response.hovered() { green_hover() } else { green() };
    ui.painter().rect_filled(rect, egui::CornerRadius::same(21), bg);
    let symbol = if is_playing { "⏸" } else { "▶" };
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        symbol,
        egui::FontId::proportional(18.0),
        bg_black(),
    );
    response
}

pub fn nav_item(ui: &mut egui::Ui, icon: &str, label: &str, is_selected: bool) -> egui::Response {
    let height = 40.0;
    let desired_size = egui::vec2(ui.available_width(), height);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    let bg = if is_selected {
        bg_hover()
    } else if response.hovered() {
        bg_card()
    } else {
        egui::Color32::TRANSPARENT
    };
    let text_color = if is_selected || response.hovered() {
        text_primary()
    } else {
        text_secondary()
    };
    let icon_color = if is_selected { green() } else { text_color };
    ui.painter().rect_filled(rect, egui::CornerRadius::same(RADIUS_MEDIUM), bg);
    if is_selected {
        ui.painter().rect_filled(
            egui::Rect::from_min_size(rect.min, egui::vec2(3.0, rect.height())),
            egui::CornerRadius::same(1),
            green(),
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
        bg_hover()
    } else if response.hovered() {
        bg_card()
    } else {
        egui::Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, egui::CornerRadius::same(RADIUS_SMALL), bg);
    let text_color = if is_selected || response.hovered() {
        text_primary()
    } else {
        text_secondary()
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
            text_dim(),
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
        text_dim(),
    );
}

pub fn divider_line(ui: &mut egui::Ui) {
    ui.allocate_space(egui::vec2(ui.available_width(), 8.0));
    let rect = ui.allocate_space(egui::vec2(ui.available_width(), 1.0)).1;
    ui.painter().rect_filled(rect, egui::CornerRadius::ZERO, divider());
    ui.allocate_space(egui::vec2(ui.available_width(), 8.0));
}

pub fn card<R>(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> egui::InnerResponse<R> {
    egui::Frame::new()
        .fill(bg_dark())
        .corner_radius(egui::CornerRadius::same(RADIUS_MEDIUM))
        .inner_margin(egui::Margin::same(16))
        .show(ui, add_contents)
}

pub fn page_title(ui: &mut egui::Ui, title: &str) {
    ui.add_space(24.0);
    ui.horizontal(|ui| {
        ui.add_space(24.0);
        ui.label(egui::RichText::new(title).size(28.0).strong().color(text_primary()));
    });
    ui.add_space(20.0);
}

pub fn format_duration_secs(secs: u64) -> String {
    let hours = secs / 3600;
    let mins = (secs % 3600) / 60;
    let s = secs % 60;
    if hours > 0 {
        format!("{hours}:{mins:02}:{s:02}")
    } else {
        format!("{mins}:{s:02}")
    }
}

// === UI Polish utilities ===

pub fn lerp_color(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    egui::Color32::from_rgba_premultiplied(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
        (a.a() as f32 + (b.a() as f32 - a.a() as f32) * t) as u8,
    )
}

pub fn with_alpha(c: egui::Color32, alpha: u8) -> egui::Color32 {
    egui::Color32::from_rgba_premultiplied(c.r(), c.g(), c.b(), alpha)
}

pub fn draw_shimmer_rect(
    painter: &egui::Painter,
    rect: egui::Rect,
    corner_radius: impl Into<egui::CornerRadius>,
    time: f32,
) {
    let base = bg_active();
    let r: egui::CornerRadius = corner_radius.into();
    let segments = 5;
    let seg_w = rect.width() / segments as f32;
    for i in 0..segments {
        let seg_rect = egui::Rect::from_min_size(
            egui::pos2(rect.left() + i as f32 * seg_w, rect.top()),
            egui::vec2(seg_w + 1.0, rect.height()),
        );
        let offset = i as f32 / segments as f32;
        let phase = (time * 1.5 + offset) % 1.0;
        let brightness = if phase < 0.5 {
            phase * 2.0
        } else {
            2.0 - phase * 2.0
        };
        let color = lerp_color(base, lerp_color(base, text_primary(), 0.12), brightness);
        painter.rect_filled(seg_rect, r, color);
    }
}

pub fn glass_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(bg_black())
        .stroke(egui::Stroke::new(1.0, with_alpha(border(), 80)))
        .corner_radius(egui::CornerRadius::same(RADIUS_MEDIUM))
        .inner_margin(egui::Margin::same(8))
}

pub fn draw_glow_border(
    painter: &egui::Painter,
    rect: egui::Rect,
    corner_radius: impl Into<egui::CornerRadius> + Copy,
    color: egui::Color32,
) {
    let glow = with_alpha(color, 20);
    for i in 0..3 {
        let expanded = rect.expand(i as f32 + 1.0);
        painter.rect_stroke(
            expanded,
            corner_radius,
            egui::Stroke::new(1.0, glow),
            egui::StrokeKind::Outside,
        );
    }
}

pub fn breadcrumb(ui: &mut egui::Ui, segments: &[(&str, bool)]) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.add_space(24.0);
        for (i, (label, clickable)) in segments.iter().enumerate() {
            if i > 0 {
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("›")
                        .size(12.0)
                        .color(text_muted()),
                );
                ui.add_space(4.0);
            }
            let color = if *clickable {
                accent()
            } else {
                text_secondary()
            };
            let resp = ui.label(
                egui::RichText::new(*label)
                    .size(12.0)
                    .color(color),
            );
            if *clickable && resp.hovered() {
                ui.painter().line_segment(
                    [
                        resp.rect.left_bottom() + egui::vec2(0.0, 1.0),
                        resp.rect.right_bottom() + egui::vec2(0.0, 1.0),
                    ],
                    egui::Stroke::new(1.0, accent()),
                );
            }
        }
    });
    ui.add_space(4.0);
}
