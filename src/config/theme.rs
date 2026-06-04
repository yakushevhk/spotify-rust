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
            Ok(content) => Ok(toml::from_str(&content).unwrap_or_default()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error.into()),
        }
    }

    pub fn find_theme(&self, name: &str) -> Option<Theme> {
        self.themes.iter().find(|t| t.name.eq_ignore_ascii_case(name)).cloned()
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct ThemeConfig {
    #[serde(default)]
    pub themes: Vec<Theme>,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            themes: Vec::new(),
        }
    }
}
