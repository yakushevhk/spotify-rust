use serde::Deserialize;

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Theme {
    pub name: String,
    #[serde(default)]
    palette: Palette,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct Palette {}

#[derive(Clone, Debug, Deserialize)]
pub struct ThemeConfig {
    #[serde(default)]
    pub themes: Vec<Theme>,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            themes: vec![Theme {
                name: "default".to_string(),
                palette: Palette {},
            }],
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
        self.themes.iter().find(|t| t.name == name).cloned()
    }
}
