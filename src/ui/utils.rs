pub fn to_bidi_string(s: &str) -> String {
    s.to_string()
}

pub enum Orientation {
    Horizontal,
    Vertical,
}

impl Default for Orientation {
    fn default() -> Self {
        Self::Horizontal
    }
}

impl Orientation {
    pub fn from_size(_columns: u16, _rows: u16) -> Self {
        Self::Horizontal
    }
}
