use std::time::{Duration, Instant};

use eframe::egui;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeySequenceResult {
    /// Need more keys to complete the sequence
    Pending(String),
    /// A complete command was resolved
    Complete(CommandId),
    /// No matching sequence found
    None,
    /// Count prefix parsed (e.g. "5" in "5j")
    CountPending(usize, String),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CommandId(pub &'static str);

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
pub enum KeyBinding {
    /// Single key like "j", "k", "r"
    Key(char),
    /// Key with modifiers like "C-f", "C-b"
    Modified { key: char, ctrl: bool, shift: bool },
    /// Special key like "Space", "Enter", "Escape", "Home", "End", etc.
    Special(String),
    /// Multi-key sequence like "gg", "gt", "st"
    Sequence(Vec<String>),
}

impl KeyBinding {
    pub fn display_string(&self) -> String {
        match self {
            KeyBinding::Key(c) => c.to_string(),
            KeyBinding::Modified { key, ctrl, shift } => {
                let mut s = String::new();
                if *ctrl {
                    s.push_str("C-");
                }
                if *shift {
                    s.push_str("S-");
                }
                s.push(*key);
                s
            }
            KeyBinding::Special(name) => name.clone(),
            KeyBinding::Sequence(parts) => parts.join(" "),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CommandBinding {
    pub command: CommandId,
    pub keybindings: Vec<KeyBinding>,
    pub description: &'static str,
    pub category: CommandCategory,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum CommandCategory {
    Navigation,
    Sorting,
    Playback,
    Actions,
    Pages,
    Other,
}

impl CommandCategory {
    pub fn display_name(&self) -> &'static str {
        match self {
            CommandCategory::Navigation => "Navigation",
            CommandCategory::Sorting => "Sorting",
            CommandCategory::Playback => "Playback",
            CommandCategory::Actions => "Actions",
            CommandCategory::Pages => "Pages",
            CommandCategory::Other => "Other",
        }
    }
}

const KEY_SEQUENCE_TIMEOUT: Duration = Duration::from_millis(500);

pub struct KeySequenceState {
    pending_keys: String,
    last_key_time: Instant,
    count_prefix: Option<usize>,
}

impl KeySequenceState {
    pub fn new() -> Self {
        Self {
            pending_keys: String::new(),
            last_key_time: Instant::now(),
            count_prefix: None,
        }
    }

    pub fn pending_display(&self) -> String {
        let mut s = String::new();
        if let Some(count) = self.count_prefix {
            s.push_str(&count.to_string());
        }
        s.push_str(&self.pending_keys);
        s
    }

    pub fn is_pending(&self) -> bool {
        !self.pending_keys.is_empty() || self.count_prefix.is_some()
    }

    pub fn reset(&mut self) {
        self.pending_keys.clear();
        self.count_prefix = None;
    }

    pub fn process_key(
        &mut self,
        key: egui::Key,
        modifiers: egui::Modifiers,
        keybindings: &[CommandBinding],
    ) -> (KeySequenceResult, Option<usize>) {
        // Check for timeout
        if !self.pending_keys.is_empty() || self.count_prefix.is_some() {
            if self.last_key_time.elapsed() > KEY_SEQUENCE_TIMEOUT {
                self.reset();
            }
        }

        self.last_key_time = Instant::now();

        // Try to read a char for vim-style keys
        let ch = key_to_char(key, modifiers);

        // Handle count prefix: digits 1-9 (vim-style, 0 is a motion, not a count)
        if let Some(c) = ch {
            if c.is_ascii_digit() && c != '0' && self.pending_keys.is_empty() {
                let digit = c.to_digit(10).unwrap() as usize;
                let current = self.count_prefix.unwrap_or(0);
                self.count_prefix = Some(current * 10 + digit);
                return (
                    KeySequenceResult::CountPending(
                        self.count_prefix.unwrap(),
                        self.pending_display(),
                    ),
                    None,
                );
            }
        }

        // Build the key string for matching.
        // For Ctrl/Alt/Cmd-modified keys, format with prefix (e.g. "C-f", "C-Space").
        // For plain keys and Shift-only, use the character directly.
        // For special non-character keys (Home, Enter, etc.), use their name.
        let has_modifier = modifiers.ctrl || modifiers.alt || modifiers.mac_cmd;
        let key_str = if key == egui::Key::Space && !has_modifier {
            "Space".to_string()
        } else {
            match ch {
                Some(c) if !has_modifier => {
                    c.to_string()
                }
                Some(c) => {
                    format_modified_key_with_alt(c, modifiers.ctrl, modifiers.shift, modifiers.alt, modifiers.mac_cmd)
                }
                None => {
                    format_special_key(key, modifiers)
                }
            }
        };

        if key_str.is_empty() {
            return (KeySequenceResult::None, None);
        }

        // Append to pending
        if !self.pending_keys.is_empty() {
            self.pending_keys.push(' ');
        }
        self.pending_keys.push_str(&key_str);

        // Try to match against bindings
        let count = self.count_prefix.unwrap_or(1);
        let result = self.match_sequence(keybindings);
        match result {
            KeySequenceResult::Complete(cmd) => {
                self.reset();
                (KeySequenceResult::Complete(cmd), Some(count))
            }
            KeySequenceResult::None => {
                // Check if any binding starts with our current sequence
                let starts_with = keybindings.iter().any(|b| {
                    match &b.keybindings[..] {
                        [KeyBinding::Sequence(parts)] => {
                            let seq_str = parts.join(" ");
                            seq_str.starts_with(&self.pending_keys)
                                && seq_str != self.pending_keys
                        }
                        _ => false,
                    }
                });
                if starts_with {
                    (
                        KeySequenceResult::Pending(self.pending_display()),
                        None,
                    )
                } else {
                    self.reset();
                    (KeySequenceResult::None, None)
                }
            }
            other => (other, None),
        }
    }

    fn match_sequence(&self, keybindings: &[CommandBinding]) -> KeySequenceResult {
        // First, try exact sequence match
        for binding in keybindings {
            for kb in &binding.keybindings {
                match kb {
                    KeyBinding::Sequence(parts) => {
                        let seq_str = parts.join(" ");
                        if seq_str == self.pending_keys {
                            return KeySequenceResult::Complete(binding.command.clone());
                        }
                    }
                    _ => {}
                }
            }
        }

        // Try single key match.
        // A pending string without spaces is a single key (might be multi-char
        // like "Home", "C-f", "Enter"). A string with spaces is a sequence.
        if !self.pending_keys.contains(' ') {
            // Only single-char keys can be start of a multi-key sequence.
            // For multi-char keys (like "Home", "C-f"), match directly.
            let starts_sequence = self.pending_keys.len() == 1
                && keybindings.iter().any(|b| {
                    b.keybindings.iter().any(|kb| match kb {
                        KeyBinding::Sequence(parts) => {
                            parts.first().map_or(false, |p| *p == self.pending_keys)
                        }
                        _ => false,
                    })
                });

            if !starts_sequence {
                for binding in keybindings {
                    for kb in &binding.keybindings {
                        match kb {
                            KeyBinding::Key(c) => {
                                if self.pending_keys.len() == 1
                                    && self.pending_keys.chars().next() == Some(*c)
                                {
                                    return KeySequenceResult::Complete(binding.command.clone());
                                }
                            }
                            KeyBinding::Modified { key, ctrl, shift } => {
                                let expected = format_modified_key(*key, *ctrl, *shift);
                                if self.pending_keys == expected {
                                    return KeySequenceResult::Complete(binding.command.clone());
                                }
                            }
                            KeyBinding::Special(name) => {
                                if self.pending_keys == *name {
                                    return KeySequenceResult::Complete(binding.command.clone());
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        KeySequenceResult::None
    }
}

pub fn key_to_char(key: egui::Key, modifiers: egui::Modifiers) -> Option<char> {
    match key {
        egui::Key::A => Some(if modifiers.shift { 'A' } else { 'a' }),
        egui::Key::B => Some(if modifiers.shift { 'B' } else { 'b' }),
        egui::Key::C => Some(if modifiers.shift { 'C' } else { 'c' }),
        egui::Key::D => Some(if modifiers.shift { 'D' } else { 'd' }),
        egui::Key::E => Some(if modifiers.shift { 'E' } else { 'e' }),
        egui::Key::F => Some(if modifiers.shift { 'F' } else { 'f' }),
        egui::Key::G => Some(if modifiers.shift { 'G' } else { 'g' }),
        egui::Key::H => Some(if modifiers.shift { 'H' } else { 'h' }),
        egui::Key::I => Some(if modifiers.shift { 'I' } else { 'i' }),
        egui::Key::J => Some(if modifiers.shift { 'J' } else { 'j' }),
        egui::Key::K => Some(if modifiers.shift { 'K' } else { 'k' }),
        egui::Key::L => Some(if modifiers.shift { 'L' } else { 'l' }),
        egui::Key::M => Some(if modifiers.shift { 'M' } else { 'm' }),
        egui::Key::N => Some(if modifiers.shift { 'N' } else { 'n' }),
        egui::Key::O => Some(if modifiers.shift { 'O' } else { 'o' }),
        egui::Key::P => Some(if modifiers.shift { 'P' } else { 'p' }),
        egui::Key::Q => Some(if modifiers.shift { 'Q' } else { 'q' }),
        egui::Key::R => Some(if modifiers.shift { 'R' } else { 'r' }),
        egui::Key::S => Some(if modifiers.shift { 'S' } else { 's' }),
        egui::Key::T => Some(if modifiers.shift { 'T' } else { 't' }),
        egui::Key::U => Some(if modifiers.shift { 'U' } else { 'u' }),
        egui::Key::V => Some(if modifiers.shift { 'V' } else { 'v' }),
        egui::Key::W => Some(if modifiers.shift { 'W' } else { 'w' }),
        egui::Key::X => Some(if modifiers.shift { 'X' } else { 'x' }),
        egui::Key::Y => Some(if modifiers.shift { 'Y' } else { 'y' }),
        egui::Key::Z => Some(if modifiers.shift { 'Z' } else { 'z' }),
        egui::Key::Num0 => Some(if modifiers.shift { ')' } else { '0' }),
        egui::Key::Num1 => Some(if modifiers.shift { '!' } else { '1' }),
        egui::Key::Num2 => Some(if modifiers.shift { '@' } else { '2' }),
        egui::Key::Num3 => Some(if modifiers.shift { '#' } else { '3' }),
        egui::Key::Num4 => Some(if modifiers.shift { '$' } else { '4' }),
        egui::Key::Num5 => Some(if modifiers.shift { '%' } else { '5' }),
        egui::Key::Num6 => Some(if modifiers.shift { '^' } else { '6' }),
        egui::Key::Num7 => Some(if modifiers.shift { '&' } else { '7' }),
        egui::Key::Num8 => Some(if modifiers.shift { '*' } else { '8' }),
        egui::Key::Num9 => Some(if modifiers.shift { '(' } else { '9' }),
        // Symbol keys with shift variants
        egui::Key::Slash => Some(if modifiers.shift { '?' } else { '/' }),
        egui::Key::Backslash => Some(if modifiers.shift { '|' } else { '\\' }),
        egui::Key::Pipe => Some('|'),
        egui::Key::Period => Some(if modifiers.shift { '>' } else { '.' }),
        egui::Key::Comma => Some(if modifiers.shift { '<' } else { ',' }),
        egui::Key::Semicolon => Some(if modifiers.shift { ':' } else { ';' }),
        egui::Key::Colon => Some(':'),
        egui::Key::Questionmark => Some('?'),
        egui::Key::Exclamationmark => Some('!'),
        egui::Key::Equals => Some(if modifiers.shift { '+' } else { '=' }),
        egui::Key::Plus => Some('+'),
        egui::Key::Minus => Some(if modifiers.shift { '_' } else { '-' }),
        egui::Key::OpenBracket => Some(if modifiers.shift { '{' } else { '[' }),
        egui::Key::CloseBracket => Some(if modifiers.shift { '}' } else { ']'}),
        // Space is special but can be a char for "g space"
        egui::Key::Space => Some(' '),
        // Underscore (Shift+Minus)
        _ => None,
    }
}

fn format_special_key(key: egui::Key, modifiers: egui::Modifiers) -> String {
    let ctrl = modifiers.ctrl;
    match key {
        egui::Key::Home => "Home".to_string(),
        egui::Key::End => "End".to_string(),
        egui::Key::PageUp => "PageUp".to_string(),
        egui::Key::PageDown => "PageDown".to_string(),
        egui::Key::Tab => {
            if modifiers.shift {
                "BackTab".to_string()
            } else {
                "Tab".to_string()
            }
        }
        egui::Key::Backspace => "Backspace".to_string(),
        egui::Key::Enter => "Enter".to_string(),
        egui::Key::Escape => "Escape".to_string(),
        egui::Key::ArrowUp => "ArrowUp".to_string(),
        egui::Key::ArrowDown => "ArrowDown".to_string(),
        egui::Key::ArrowLeft => "ArrowLeft".to_string(),
        egui::Key::ArrowRight => "ArrowRight".to_string(),
        egui::Key::F1 => "F1".to_string(),
        egui::Key::F2 => "F2".to_string(),
        egui::Key::F3 => "F3".to_string(),
        egui::Key::F4 => "F4".to_string(),
        egui::Key::F5 => "F5".to_string(),
        egui::Key::F6 => "F6".to_string(),
        egui::Key::F7 => "F7".to_string(),
        egui::Key::F8 => "F8".to_string(),
        egui::Key::F9 => "F9".to_string(),
        egui::Key::F10 => "F10".to_string(),
        egui::Key::F11 => "F11".to_string(),
        egui::Key::F12 => "F12".to_string(),
        _ => {
            // Handle Control+key combinations that aren't regular chars
            if ctrl {
                if let Some(c) = key_to_char(key, egui::Modifiers::NONE) {
                    format!("C-{}", c)
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        }
    }
}

fn format_modified_key(key: char, ctrl: bool, shift: bool) -> String {
    let mut s = String::new();
    if ctrl {
        s.push_str("C-");
    }
    if shift {
        s.push_str("S-");
    }
    if key == ' ' {
        s.push_str("Space");
    } else {
        s.push(key);
    }
    s
}

fn format_modified_key_with_alt(
    key: char,
    ctrl: bool,
    shift: bool,
    alt: bool,
    mac_cmd: bool,
) -> String {
    let mut s = String::new();
    if ctrl {
        s.push_str("C-");
    }
    if alt {
        s.push_str("A-");
    }
    if mac_cmd {
        s.push_str("M-");
    }
    if shift {
        s.push_str("S-");
    }
    if key == ' ' {
        s.push_str("Space");
    } else {
        s.push(key);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vim_count_prefix() {
        let mut state = KeySequenceState::new();
        let bindings = get_default_bindings();

        // Press '5'
        let (result, _) = state.process_key(egui::Key::Num5, egui::Modifiers::NONE, &bindings);
        assert!(matches!(result, KeySequenceResult::CountPending(5, _)));

        // Press 'j'
        let (result, count) = state.process_key(egui::Key::J, egui::Modifiers::NONE, &bindings);
        assert!(matches!(result, KeySequenceResult::Complete(_)));
        assert_eq!(count, Some(5));
    }

    #[test]
    fn test_gg_sequence() {
        let mut state = KeySequenceState::new();
        let bindings = get_default_bindings();

        // Press 'g'
        let (result, _) = state.process_key(egui::Key::G, egui::Modifiers::NONE, &bindings);
        assert!(matches!(result, KeySequenceResult::Pending(_)));

        // Press 'g' again
        let (result, count) = state.process_key(egui::Key::G, egui::Modifiers::NONE, &bindings);
        assert!(matches!(result, KeySequenceResult::Complete(_)));
        assert_eq!(count, Some(1));
    }

    #[test]
    fn test_gt_sequence() {
        let mut state = KeySequenceState::new();
        let bindings = get_default_bindings();

        // Press 'g'
        let (result, _) = state.process_key(egui::Key::G, egui::Modifiers::NONE, &bindings);
        assert!(matches!(result, KeySequenceResult::Pending(_)));

        // Press 't'
        let (result, _) = state.process_key(egui::Key::T, egui::Modifiers::NONE, &bindings);
        assert!(matches!(result, KeySequenceResult::Complete(_)));
    }

    #[test]
    fn test_st_sequence() {
        let mut state = KeySequenceState::new();
        let bindings = get_default_bindings();

        // Press 's'
        let (result, _) = state.process_key(egui::Key::S, egui::Modifiers::NONE, &bindings);
        assert!(matches!(result, KeySequenceResult::Pending(_)));

        // Press 't'
        let (result, _) = state.process_key(egui::Key::T, egui::Modifiers::NONE, &bindings);
        assert!(matches!(result, KeySequenceResult::Complete(_)));
    }

    #[test]
    fn test_count_with_sequence() {
        let mut state = KeySequenceState::new();
        let bindings = get_default_bindings();

        // Press '3'
        let (result, _) = state.process_key(egui::Key::Num3, egui::Modifiers::NONE, &bindings);
        assert!(matches!(result, KeySequenceResult::CountPending(3, _)));

        // Press 'g'
        let (result, _) = state.process_key(egui::Key::G, egui::Modifiers::NONE, &bindings);
        assert!(matches!(result, KeySequenceResult::Pending(_)));

        // Press 'g'
        let (result, count) = state.process_key(egui::Key::G, egui::Modifiers::NONE, &bindings);
        assert!(matches!(result, KeySequenceResult::Complete(_)));
        assert_eq!(count, Some(3));
    }

    #[test]
    fn test_single_key_match() {
        let mut state = KeySequenceState::new();
        let bindings = get_default_bindings();

        // Press 'r'
        let (result, count) = state.process_key(egui::Key::R, egui::Modifiers::NONE, &bindings);
        assert!(matches!(result, KeySequenceResult::Complete(_)));
        assert_eq!(count, Some(1));
    }

    fn get_default_bindings() -> Vec<CommandBinding> {
        crate::config::keymap::default_keybindings()
    }
}
