//! Theme configuration module
//!
//! This module defines the theme system for the application.
//! Themes can be customized via `theme.toml`.
//!
//! # Color Fields
//!
//! ## Background Colors
//! - `background` - Main background
//! - `bg_dark` - Darker background
//! - `bg_card` - Card background
//! - `bg_hover` - Hover state
//! - `bg_active` - Active state
//! - `bg_elevated` - Elevated surfaces
//! - `bg_input` - Input fields
//! - `bg_selected` - Selected items
//!
//! ## Text Colors
//! - `text_primary` - Primary text
//! - `text_secondary` - Secondary text
//! - `text_dim` - Dimmed text
//! - `text_muted` - Muted text
//! - `text_hint` - Hint text
//!
//! ## Accent Colors
//! - `accent` - Primary accent (Spotify green)
//! - `accent_hover` - Accent hover state
//! - `accent_dark` - Darker accent
//!
//! ## Semantic Colors
//! - `success` - Success states
//! - `error` - Error states
//! - `warning` - Warning states
//!
//! ## Lyrics Colors
//! - `lyrics_current` - Currently playing line
//! - `lyrics_played` - Already played lines
//! - `lyrics_upcoming` - Upcoming lines
//! - `lyrics_bg` - Lyrics background
//!
//! # Example Theme
//!
//! ```toml
//! [[themes]]
//! name = "MyTheme"
//! [palette]
//! background = "#121212"
//! foreground = "#ffffff"
//! accent = "#1ed760"
//! ```

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Theme {
    pub name: String,
    #[serde(default)]
    pub palette: Palette,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Palette {
    #[serde(default = "default_background")]
    pub background: String,
    #[serde(default = "default_foreground")]
    pub foreground: String,
    #[serde(default = "default_accent")]
    pub accent: String,
    #[serde(default = "default_accent_hover")]
    pub accent_hover: String,
    #[serde(default = "default_accent_dark")]
    pub accent_dark: String,

    #[serde(default = "default_bg_dark")]
    pub bg_dark: String,
    #[serde(default = "default_bg_card")]
    pub bg_card: String,
    #[serde(default = "default_bg_hover")]
    pub bg_hover: String,
    #[serde(default = "default_bg_active")]
    pub bg_active: String,
    #[serde(default = "default_bg_elevated")]
    pub bg_elevated: String,
    #[serde(default = "default_bg_input")]
    pub bg_input: String,
    #[serde(default = "default_bg_selected")]
    pub bg_selected: String,

    #[serde(default = "default_text_primary")]
    pub text_primary: String,
    #[serde(default = "default_text_secondary")]
    pub text_secondary: String,
    #[serde(default = "default_text_dim")]
    pub text_dim: String,
    #[serde(default = "default_text_muted")]
    pub text_muted: String,
    #[serde(default = "default_text_hint")]
    pub text_hint: String,

    #[serde(default = "default_border")]
    pub border: String,
    #[serde(default = "default_divider")]
    pub divider: String,

    #[serde(default = "default_success")]
    pub success: String,
    #[serde(default = "default_error")]
    pub error: String,
    #[serde(default = "default_warning")]
    pub warning: String,

    #[serde(default = "default_lyrics_current")]
    pub lyrics_current: String,
    #[serde(default = "default_lyrics_played")]
    pub lyrics_played: String,
    #[serde(default = "default_lyrics_upcoming")]
    pub lyrics_upcoming: String,
    #[serde(default = "default_lyrics_bg")]
    pub lyrics_bg: String,
}

fn default_background() -> String { "#000000".to_string() }
fn default_foreground() -> String { "#ffffff".to_string() }
fn default_accent() -> String { "#1ed760".to_string() }
fn default_accent_hover() -> String { "#2de16e".to_string() }
fn default_accent_dark() -> String { "#19b450".to_string() }
fn default_bg_dark() -> String { "#000000".to_string() }
fn default_bg_card() -> String { "#000000".to_string() }
fn default_bg_hover() -> String { "#0a0a0a".to_string() }
fn default_bg_active() -> String { "#0c0c0c".to_string() }
fn default_bg_elevated() -> String { "#000000".to_string() }
fn default_bg_input() -> String { "#050505".to_string() }
fn default_bg_selected() -> String { "#080808".to_string() }
fn default_text_primary() -> String { "#ffffff".to_string() }
fn default_text_secondary() -> String { "#b3b3b3".to_string() }
fn default_text_dim() -> String { "#737373".to_string() }
fn default_text_muted() -> String { "#4d4d4d".to_string() }
fn default_text_hint() -> String { "#666666".to_string() }
fn default_border() -> String { "#0a0a0a".to_string() }
fn default_divider() -> String { "#0c0c0c".to_string() }
fn default_success() -> String { "#1ed760".to_string() }
fn default_error() -> String { "#ff5050".to_string() }
fn default_warning() -> String { "#ffc832".to_string() }
fn default_lyrics_current() -> String { "#1db954".to_string() }
fn default_lyrics_played() -> String { "#555555".to_string() }
fn default_lyrics_upcoming() -> String { "#cccccc".to_string() }
fn default_lyrics_bg() -> String { "#000000".to_string() }

impl Palette {
    /// Validate all color fields and return a list of invalid field names.
    /// Each color must match `^#[0-9a-fA-F]{6}$` or `^#[0-9a-fA-F]{8}$`.
    pub fn validate(&self) -> Vec<&'static str> {
        fn is_valid_color(s: &str) -> bool {
            (s.len() == 7 || s.len() == 9)
                && s.starts_with('#')
                && s[1..].chars().all(|c| c.is_ascii_hexdigit())
        }

        let mut invalid = Vec::new();
        if !is_valid_color(&self.background) { invalid.push("background"); }
        if !is_valid_color(&self.foreground) { invalid.push("foreground"); }
        if !is_valid_color(&self.accent) { invalid.push("accent"); }
        if !is_valid_color(&self.accent_hover) { invalid.push("accent_hover"); }
        if !is_valid_color(&self.accent_dark) { invalid.push("accent_dark"); }
        if !is_valid_color(&self.bg_dark) { invalid.push("bg_dark"); }
        if !is_valid_color(&self.bg_card) { invalid.push("bg_card"); }
        if !is_valid_color(&self.bg_hover) { invalid.push("bg_hover"); }
        if !is_valid_color(&self.bg_active) { invalid.push("bg_active"); }
        if !is_valid_color(&self.bg_elevated) { invalid.push("bg_elevated"); }
        if !is_valid_color(&self.bg_input) { invalid.push("bg_input"); }
        if !is_valid_color(&self.bg_selected) { invalid.push("bg_selected"); }
        if !is_valid_color(&self.text_primary) { invalid.push("text_primary"); }
        if !is_valid_color(&self.text_secondary) { invalid.push("text_secondary"); }
        if !is_valid_color(&self.text_dim) { invalid.push("text_dim"); }
        if !is_valid_color(&self.text_muted) { invalid.push("text_muted"); }
        if !is_valid_color(&self.text_hint) { invalid.push("text_hint"); }
        if !is_valid_color(&self.border) { invalid.push("border"); }
        if !is_valid_color(&self.divider) { invalid.push("divider"); }
        if !is_valid_color(&self.success) { invalid.push("success"); }
        if !is_valid_color(&self.error) { invalid.push("error"); }
        if !is_valid_color(&self.warning) { invalid.push("warning"); }
        if !is_valid_color(&self.lyrics_current) { invalid.push("lyrics_current"); }
        if !is_valid_color(&self.lyrics_played) { invalid.push("lyrics_played"); }
        if !is_valid_color(&self.lyrics_upcoming) { invalid.push("lyrics_upcoming"); }
        if !is_valid_color(&self.lyrics_bg) { invalid.push("lyrics_bg"); }
        invalid
    }
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            background: default_background(),
            foreground: default_foreground(),
            accent: default_accent(),
            accent_hover: default_accent_hover(),
            accent_dark: default_accent_dark(),
            bg_dark: default_bg_dark(),
            bg_card: default_bg_card(),
            bg_hover: default_bg_hover(),
            bg_active: default_bg_active(),
            bg_elevated: default_bg_elevated(),
            bg_input: default_bg_input(),
            bg_selected: default_bg_selected(),
            text_primary: default_text_primary(),
            text_secondary: default_text_secondary(),
            text_dim: default_text_dim(),
            text_muted: default_text_muted(),
            text_hint: default_text_hint(),
            border: default_border(),
            divider: default_divider(),
            success: default_success(),
            error: default_error(),
            warning: default_warning(),
            lyrics_current: default_lyrics_current(),
            lyrics_played: default_lyrics_played(),
            lyrics_upcoming: default_lyrics_upcoming(),
            lyrics_bg: default_lyrics_bg(),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            name: "Dracula".to_string(),
            palette: Palette::default(),
        }
    }
}

impl ThemeConfig {
    pub fn new(path: &std::path::Path) -> anyhow::Result<Self> {
        let file_path = path.join("theme.toml");
        match std::fs::read_to_string(file_path) {
            Ok(content) => match toml::from_str::<ThemeConfig>(&content) {
                Ok(mut config) => {
                    for theme in &mut config.themes {
                        let invalid = theme.palette.validate();
                        if !invalid.is_empty() {
                            tracing::warn!(
                                "theme '{}' has invalid color values: {:?}, using defaults for those",
                                theme.name,
                                invalid
                            );
                            let defaults = Palette::default();
                            let palette = &mut theme.palette;
                            for field in &invalid {
                                match *field {
                                    "background" => palette.background = defaults.background.clone(),
                                    "foreground" => palette.foreground = defaults.foreground.clone(),
                                    "accent" => palette.accent = defaults.accent.clone(),
                                    "accent_hover" => palette.accent_hover = defaults.accent_hover.clone(),
                                    "accent_dark" => palette.accent_dark = defaults.accent_dark.clone(),
                                    "bg_dark" => palette.bg_dark = defaults.bg_dark.clone(),
                                    "bg_card" => palette.bg_card = defaults.bg_card.clone(),
                                    "bg_hover" => palette.bg_hover = defaults.bg_hover.clone(),
                                    "bg_active" => palette.bg_active = defaults.bg_active.clone(),
                                    "bg_elevated" => palette.bg_elevated = defaults.bg_elevated.clone(),
                                    "bg_input" => palette.bg_input = defaults.bg_input.clone(),
                                    "bg_selected" => palette.bg_selected = defaults.bg_selected.clone(),
                                    "text_primary" => palette.text_primary = defaults.text_primary.clone(),
                                    "text_secondary" => palette.text_secondary = defaults.text_secondary.clone(),
                                    "text_dim" => palette.text_dim = defaults.text_dim.clone(),
                                    "text_muted" => palette.text_muted = defaults.text_muted.clone(),
                                    "text_hint" => palette.text_hint = defaults.text_hint.clone(),
                                    "border" => palette.border = defaults.border.clone(),
                                    "divider" => palette.divider = defaults.divider.clone(),
                                    "success" => palette.success = defaults.success.clone(),
                                    "error" => palette.error = defaults.error.clone(),
                                    "warning" => palette.warning = defaults.warning.clone(),
                                    "lyrics_current" => palette.lyrics_current = defaults.lyrics_current.clone(),
                                    "lyrics_played" => palette.lyrics_played = defaults.lyrics_played.clone(),
                                    "lyrics_upcoming" => palette.lyrics_upcoming = defaults.lyrics_upcoming.clone(),
                                    "lyrics_bg" => palette.lyrics_bg = defaults.lyrics_bg.clone(),
                                    _ => {}
                                }
                            }
                        }
                    }
                    Ok(config)
                }
                Err(e) => {
                    tracing::warn!("failed to parse theme config: {:#}, using defaults", e);
                    Ok(Self::default())
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error.into()),
        }
    }

    pub fn find_theme(&self, name: &str) -> Option<Theme> {
        self.themes.iter().find(|t| t.name.eq_ignore_ascii_case(name)).cloned()
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct ThemeConfig {
    #[serde(default)]
    pub themes: Vec<Theme>,
}
