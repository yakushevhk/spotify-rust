use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct KeymapConfig {
    #[serde(default)]
    pub keymaps: Vec<Keymap>,
    #[serde(default)]
    pub actions: Vec<ActionMap>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Keymap {
    pub key_sequence: String,
    pub command: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ActionMap {
    pub key_sequence: String,
    pub action: String,
}

impl KeymapConfig {
    pub fn new(_path: &std::path::Path) -> anyhow::Result<Self> {
        Ok(Self::default())
    }
}
