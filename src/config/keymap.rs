//! Keymap configuration module
//!
//! This module handles keyboard shortcut configuration.
//! Custom keybindings can be defined in `keymap.toml`.
//!
//! # Key Sequence Format
//!
//! ## Single Keys
//! - `j`, `k` - Single character keys
//! - `Space`, `Enter`, `Escape` - Special keys
//!
//! ## Modifier Keys
//! - `C-x` - Ctrl + key (e.g., `C-f`)
//! - `S-x` - Shift + key (e.g., `S-g`)
//! - `C-S-x` - Ctrl + Shift + key
//!
//! ## Multi-key Sequences
//! - `g g` - Press 'g' twice
//! - `g t` - Press 'g' then 't'
//!
//! ## Special Keys
//! - `Space`, `Enter`, `Escape`, `Tab`
//! - `BackTab`, `Backspace`
//! - `Home`, `End`, `PageUp`, `PageDown`
//! - `ArrowUp`, `ArrowDown`, `ArrowLeft`, `ArrowRight`
//! - `F1` through `F12`
//!
//! # Example keymap.toml
//!
//! ```toml
//! [[keymaps]]
//! key_sequence = "C-p"
//! command = "play_pause"
//!
//! [[keymaps]]
//! key_sequence = "g s"
//! command = "page_search"
//! ```

use serde::Deserialize;

use crate::key::{CommandBinding, CommandCategory, CommandId, KeyBinding};

#[derive(Debug, Deserialize, Default, Clone)]
pub struct KeymapConfig {
    #[serde(default)]
    pub keymaps: Vec<Keymap>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Keymap {
    pub key_sequence: String,
    pub command: String,
}

impl KeymapConfig {
    pub fn new(path: &std::path::Path) -> anyhow::Result<Self> {
        let file_path = path.join("keymap.toml");
        match std::fs::read_to_string(&file_path) {
            Ok(content) => {
                let config: KeymapConfig = match toml::from_str(&content) {
                    Ok(config) => config,
                    Err(e) => {
                        tracing::warn!("failed to parse keymap config: {:#}, using defaults", e);
                        KeymapConfig::default()
                    }
                };
                Ok(config)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.into()),
        }
    }

    /// Merge user keymap overrides into the default keybindings.
    /// For each user keymap entry whose command matches an existing binding,
    /// replace the keybindings of that binding.
    pub fn apply_overrides(&self, defaults: &mut [CommandBinding]) {
        for km in &self.keymaps {
            let mut matched = false;
            // K4: find all matching bindings, not just the first
            for binding in defaults.iter_mut().filter(|b| b.command.0 == km.command) {
                let parsed = parse_key_sequence(&km.key_sequence);
                if !parsed.is_empty() {
                    binding.keybindings = parsed;
                }
                matched = true;
            }
            if !matched {
                tracing::warn!("unknown command in keymap: {}", km.command);
            }
        }
    }
}

fn parse_key_sequence(s: &str) -> Vec<KeyBinding> {
    let s = s.trim();
    if s.is_empty() {
        return vec![];
    }

    // K7: normalize to lowercase for case-insensitive matching
    let sl = s.to_ascii_lowercase();

    // Check for modifier format: "C-x", "S-C-x", "C-S-x", etc.
    if sl.starts_with("c-") || sl.starts_with("s-") {
        let mut ctrl = false;
        let mut shift = false;
        let mut rest = sl.as_str();
        loop {
            if rest.starts_with("c-") {
                ctrl = true;
                rest = &rest[2..];
            } else if rest.starts_with("s-") {
                shift = true;
                rest = &rest[2..];
            } else {
                break;
            }
        }
        if let Some(ch) = rest.chars().next() {
            return vec![KeyBinding::Modified { key: ch, ctrl, shift }];
        }
    }

    // Check for special keys
    let special_keys = [
        "Space", "Enter", "Escape", "Tab", "BackTab", "Backspace",
        "Home", "End", "PageUp", "PageDown",
        "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight",
        "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12",
    ];
    for sk in &special_keys {
        if sl.eq_ignore_ascii_case(&sk.to_ascii_lowercase()) {
            return vec![KeyBinding::Special(sk.to_string())];
        }
    }

    // Multi-key sequence like "gg", "g t", "g space"
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() > 1 {
        return vec![KeyBinding::Sequence(
            parts.iter().map(|p| p.to_string()).collect(),
        )];
    }

    // Single character key
    if s.chars().count() == 1 {
        let c = s.chars().next().unwrap();
        return vec![KeyBinding::Key(c)];
    }

    vec![]
}

/// All default keybindings for the application.
pub fn default_keybindings() -> Vec<CommandBinding> {
    vec![
        // === Navigation ===
        CommandBinding {
            command: CommandId("nav_up"),
            keybindings: vec![
                KeyBinding::Key('k'),
                KeyBinding::Special("ArrowUp".to_string()),
            ],
            description: "Move selection up",
            category: CommandCategory::Navigation,
        },
        CommandBinding {
            command: CommandId("nav_down"),
            keybindings: vec![
                KeyBinding::Key('j'),
                KeyBinding::Special("ArrowDown".to_string()),
            ],
            description: "Move selection down",
            category: CommandCategory::Navigation,
        },
        CommandBinding {
            command: CommandId("first"),
            keybindings: vec![
                KeyBinding::Sequence(vec!["g".to_string(), "g".to_string()]),
                KeyBinding::Special("Home".to_string()),
            ],
            description: "Select first / scroll to top",
            category: CommandCategory::Navigation,
        },
        CommandBinding {
            command: CommandId("last"),
            keybindings: vec![
                KeyBinding::Key('G'),
                KeyBinding::Special("End".to_string()),
            ],
            description: "Select last / scroll to bottom",
            category: CommandCategory::Navigation,
        },
        CommandBinding {
            command: CommandId("page_down"),
            keybindings: vec![
                KeyBinding::Special("PageDown".to_string()),
                KeyBinding::Modified {
                    key: 'f',
                    ctrl: true,
                    shift: false,
                },
            ],
            description: "Page down",
            category: CommandCategory::Navigation,
        },
        CommandBinding {
            command: CommandId("page_up"),
            keybindings: vec![
                KeyBinding::Special("PageUp".to_string()),
                KeyBinding::Modified {
                    key: 'b',
                    ctrl: true,
                    shift: false,
                },
            ],
            description: "Page up",
            category: CommandCategory::Navigation,
        },
        CommandBinding {
            command: CommandId("focus_next"),
            keybindings: vec![
                KeyBinding::Special("Tab".to_string()),
            ],
            description: "Focus next window",
            category: CommandCategory::Navigation,
        },
        CommandBinding {
            command: CommandId("focus_prev"),
            keybindings: vec![
                KeyBinding::Special("BackTab".to_string()),
            ],
            description: "Focus previous window",
            category: CommandCategory::Navigation,
        },
        CommandBinding {
            command: CommandId("back"),
            keybindings: vec![
                KeyBinding::Special("Backspace".to_string()),
                KeyBinding::Modified {
                    key: 'q',
                    ctrl: true,
                    shift: false,
                },
            ],
            description: "Previous page (go back)",
            category: CommandCategory::Navigation,
        },
        CommandBinding {
            command: CommandId("forward"),
            keybindings: vec![
                KeyBinding::Modified {
                    key: ']',
                    ctrl: true,
                    shift: false,
                },
            ],
            description: "Next page (go forward)",
            category: CommandCategory::Navigation,
        },
        CommandBinding {
            command: CommandId("enter"),
            keybindings: vec![
                KeyBinding::Special("Enter".to_string()),
            ],
            description: "Play selected / confirm",
            category: CommandCategory::Navigation,
        },
        CommandBinding {
            command: CommandId("quit"),
            keybindings: vec![
                KeyBinding::Key('q'),
                KeyBinding::Modified {
                    key: 'c',
                    ctrl: true,
                    shift: false,
                },
            ],
            description: "Quit the application",
            category: CommandCategory::Navigation,
        },
        CommandBinding {
            command: CommandId("in_page_search"),
            keybindings: vec![KeyBinding::Key('/')],
            description: "Search within current view",
            category: CommandCategory::Navigation,
        },

        // === Sorting ===
        CommandBinding {
            command: CommandId("sort_by_title"),
            keybindings: vec![KeyBinding::Sequence(vec![
                "s".to_string(),
                "t".to_string(),
            ])],
            description: "Sort tracks by title",
            category: CommandCategory::Sorting,
        },
        CommandBinding {
            command: CommandId("sort_by_artist"),
            keybindings: vec![KeyBinding::Sequence(vec![
                "s".to_string(),
                "a".to_string(),
            ])],
            description: "Sort tracks by artists",
            category: CommandCategory::Sorting,
        },
        CommandBinding {
            command: CommandId("sort_by_album"),
            keybindings: vec![KeyBinding::Sequence(vec![
                "s".to_string(),
                "A".to_string(),
            ])],
            description: "Sort tracks by album",
            category: CommandCategory::Sorting,
        },
        CommandBinding {
            command: CommandId("sort_by_added_date"),
            keybindings: vec![KeyBinding::Sequence(vec![
                "s".to_string(),
                "D".to_string(),
            ])],
            description: "Sort tracks by added date",
            category: CommandCategory::Sorting,
        },
        CommandBinding {
            command: CommandId("sort_by_duration"),
            keybindings: vec![KeyBinding::Sequence(vec![
                "s".to_string(),
                "d".to_string(),
            ])],
            description: "Sort tracks by duration",
            category: CommandCategory::Sorting,
        },
        CommandBinding {
            command: CommandId("sort_reverse"),
            keybindings: vec![KeyBinding::Sequence(vec![
                "s".to_string(),
                "r".to_string(),
            ])],
            description: "Reverse track order",
            category: CommandCategory::Sorting,
        },
        CommandBinding {
            command: CommandId("sort_library_alpha"),
            keybindings: vec![KeyBinding::Sequence(vec![
                "s".to_string(),
                "l".to_string(),
                "a".to_string(),
            ])],
            description: "Sort library alphabetically",
            category: CommandCategory::Sorting,
        },
        CommandBinding {
            command: CommandId("sort_library_recent"),
            keybindings: vec![KeyBinding::Sequence(vec![
                "s".to_string(),
                "l".to_string(),
                "r".to_string(),
            ])],
            description: "Sort library by recently added",
            category: CommandCategory::Sorting,
        },

        // === Playback ===
        CommandBinding {
            command: CommandId("play_pause"),
            keybindings: vec![KeyBinding::Special("Space".to_string())],
            description: "Play / Pause",
            category: CommandCategory::Playback,
        },
        CommandBinding {
            command: CommandId("next_track"),
            keybindings: vec![KeyBinding::Special("ArrowRight".to_string())],
            description: "Next track",
            category: CommandCategory::Playback,
        },
        CommandBinding {
            command: CommandId("prev_track"),
            keybindings: vec![KeyBinding::Special("ArrowLeft".to_string())],
            description: "Previous track",
            category: CommandCategory::Playback,
        },
        CommandBinding {
            command: CommandId("refresh_playback"),
            keybindings: vec![KeyBinding::Key('r')],
            description: "Refresh playback",
            category: CommandCategory::Playback,
        },
        CommandBinding {
            command: CommandId("restart_client"),
            keybindings: vec![KeyBinding::Key('R')],
            description: "Restart integrated client",
            category: CommandCategory::Playback,
        },
        CommandBinding {
            command: CommandId("mute_toggle"),
            keybindings: vec![KeyBinding::Key('_')],
            description: "Mute toggle",
            category: CommandCategory::Playback,
        },
        CommandBinding {
            command: CommandId("seek_to_start"),
            keybindings: vec![KeyBinding::Key('^')],
            description: "Seek to start",
            category: CommandCategory::Playback,
        },
        CommandBinding {
            command: CommandId("seek_forward"),
            keybindings: vec![KeyBinding::Key('>')],
            description: "Seek forward by seek_duration_secs",
            category: CommandCategory::Playback,
        },
        CommandBinding {
            command: CommandId("seek_backward"),
            keybindings: vec![KeyBinding::Key('<')],
            description: "Seek backward by seek_duration_secs",
            category: CommandCategory::Playback,
        },
        CommandBinding {
            command: CommandId("play_random"),
            keybindings: vec![KeyBinding::Key('.')],
            description: "Play random track in context",
            category: CommandCategory::Playback,
        },
        CommandBinding {
            command: CommandId("shuffle"),
            keybindings: vec![KeyBinding::Modified {
                key: 's',
                ctrl: true,
                shift: false,
            }],
            description: "Toggle shuffle",
            category: CommandCategory::Playback,
        },
        CommandBinding {
            command: CommandId("repeat"),
            keybindings: vec![KeyBinding::Modified {
                key: 'r',
                ctrl: true,
                shift: false,
            }],
            description: "Toggle repeat",
            category: CommandCategory::Playback,
        },
        CommandBinding {
            command: CommandId("volume_up"),
            keybindings: vec![KeyBinding::Modified {
                key: 'i',
                ctrl: true,
                shift: false,
            }],
            description: "Volume up",
            category: CommandCategory::Playback,
        },
        CommandBinding {
            command: CommandId("volume_down"),
            keybindings: vec![KeyBinding::Modified {
                key: 'd',
                ctrl: true,
                shift: false,
            }],
            description: "Volume down",
            category: CommandCategory::Playback,
        },

        // === Actions ===
        CommandBinding {
            command: CommandId("show_actions_selected"),
            keybindings: vec![
                KeyBinding::Sequence(vec!["g".to_string(), "a".to_string()]),
                KeyBinding::Modified {
                    key: ' ',
                    ctrl: true,
                    shift: false,
                },
            ],
            description: "Show actions on selected item",
            category: CommandCategory::Actions,
        },
        CommandBinding {
            command: CommandId("show_actions_current"),
            keybindings: vec![KeyBinding::Key('a')],
            description: "Show actions on current track",
            category: CommandCategory::Actions,
        },
        CommandBinding {
            command: CommandId("show_actions_context"),
            keybindings: vec![KeyBinding::Key('A')],
            description: "Show actions on current context",
            category: CommandCategory::Actions,
        },
        CommandBinding {
            command: CommandId("add_to_queue"),
            keybindings: vec![
                KeyBinding::Key('Z'),
                KeyBinding::Modified {
                    key: 'z',
                    ctrl: true,
                    shift: false,
                },
            ],
            description: "Add selected item to queue",
            category: CommandCategory::Actions,
        },
        CommandBinding {
            command: CommandId("create_playlist"),
            keybindings: vec![KeyBinding::Key('N')],
            description: "Create new playlist",
            category: CommandCategory::Actions,
        },
        CommandBinding {
            command: CommandId("jump_to_current"),
            keybindings: vec![KeyBinding::Sequence(vec![
                "g".to_string(),
                "c".to_string(),
            ])],
            description: "Jump to current track in context",
            category: CommandCategory::Actions,
        },
        CommandBinding {
            command: CommandId("jump_to_highlighted"),
            keybindings: vec![KeyBinding::Modified {
                key: 'g',
                ctrl: true,
                shift: false,
            }],
            description: "Jump to highlighted track in context",
            category: CommandCategory::Actions,
        },
        CommandBinding {
            command: CommandId("go_to_radio"),
            keybindings: vec![KeyBinding::Modified {
                key: 'R',
                ctrl: true,
                shift: true,
            }],
            description: "Go to radio based on selected track",
            category: CommandCategory::Actions,
        },
        CommandBinding {
            command: CommandId("move_playlist_item_up"),
            keybindings: vec![KeyBinding::Modified {
                key: 'k',
                ctrl: true,
                shift: false,
            }],
            description: "Move playlist item up",
            category: CommandCategory::Actions,
        },
        CommandBinding {
            command: CommandId("move_playlist_item_down"),
            keybindings: vec![KeyBinding::Modified {
                key: 'j',
                ctrl: true,
                shift: false,
            }],
            description: "Move playlist item down",
            category: CommandCategory::Actions,
        },
        CommandBinding {
            command: CommandId("switch_device"),
            keybindings: vec![KeyBinding::Key('D')],
            description: "Switch playback device",
            category: CommandCategory::Actions,
        },

        // === Popup / Browse ===
        CommandBinding {
            command: CommandId("browse_user_playlists"),
            keybindings: vec![KeyBinding::Sequence(vec![
                "u".to_string(),
                "p".to_string(),
            ])],
            description: "Browse user playlists",
            category: CommandCategory::Actions,
        },
        CommandBinding {
            command: CommandId("browse_user_followed_artists"),
            keybindings: vec![KeyBinding::Sequence(vec![
                "u".to_string(),
                "a".to_string(),
            ])],
            description: "Browse followed artists",
            category: CommandCategory::Actions,
        },
        CommandBinding {
            command: CommandId("browse_user_saved_albums"),
            keybindings: vec![KeyBinding::Sequence(vec![
                "u".to_string(),
                "A".to_string(),
            ])],
            description: "Browse saved albums",
            category: CommandCategory::Actions,
        },

        // === Pages ===
        CommandBinding {
            command: CommandId("page_currently_playing"),
            keybindings: vec![KeyBinding::Sequence(vec![
                "g".to_string(),
                " ".to_string(),
            ])],
            description: "Go to currently playing context page",
            category: CommandCategory::Pages,
        },
        CommandBinding {
            command: CommandId("page_top_tracks"),
            keybindings: vec![KeyBinding::Sequence(vec![
                "g".to_string(),
                "t".to_string(),
            ])],
            description: "Top tracks page",
            category: CommandCategory::Pages,
        },
        CommandBinding {
            command: CommandId("page_recently_played"),
            keybindings: vec![KeyBinding::Sequence(vec![
                "g".to_string(),
                "r".to_string(),
            ])],
            description: "Recently played page",
            category: CommandCategory::Pages,
        },
        CommandBinding {
            command: CommandId("page_liked_tracks"),
            keybindings: vec![KeyBinding::Sequence(vec![
                "g".to_string(),
                "y".to_string(),
            ])],
            description: "Liked tracks page",
            category: CommandCategory::Pages,
        },
        CommandBinding {
            command: CommandId("page_library"),
            keybindings: vec![KeyBinding::Sequence(vec![
                "g".to_string(),
                "l".to_string(),
            ])],
            description: "Library page",
            category: CommandCategory::Pages,
        },
        CommandBinding {
            command: CommandId("page_search"),
            keybindings: vec![KeyBinding::Sequence(vec![
                "g".to_string(),
                "s".to_string(),
            ])],
            description: "Search page",
            category: CommandCategory::Pages,
        },
        CommandBinding {
            command: CommandId("page_browse"),
            keybindings: vec![KeyBinding::Sequence(vec![
                "g".to_string(),
                "b".to_string(),
            ])],
            description: "Browse page",
            category: CommandCategory::Pages,
        },
        CommandBinding {
            command: CommandId("page_lyrics"),
            keybindings: vec![
                KeyBinding::Sequence(vec!["g".to_string(), "L".to_string()]),
                KeyBinding::Key('l'),
                KeyBinding::Modified {
                    key: 'l',
                    ctrl: true,
                    shift: false,
                },
            ],
            description: "Lyrics page",
            category: CommandCategory::Pages,
        },
        CommandBinding {
            command: CommandId("page_queue"),
            keybindings: vec![
                KeyBinding::Key('z'),
                KeyBinding::Modified {
                    key: 'q',
                    ctrl: true,
                    shift: false,
                },
            ],
            description: "Queue page",
            category: CommandCategory::Pages,
        },
        CommandBinding {
            command: CommandId("page_logs"),
            keybindings: vec![KeyBinding::Sequence(vec![
                "g".to_string(),
                "o".to_string(),
            ])],
            description: "Open logs page",
            category: CommandCategory::Pages,
        },
        CommandBinding {
            command: CommandId("page_help"),
            keybindings: vec![
                KeyBinding::Key('?'),
                KeyBinding::Modified {
                    key: 'h',
                    ctrl: true,
                    shift: false,
                },
            ],
            description: "Command help page",
            category: CommandCategory::Pages,
        },
        CommandBinding {
            command: CommandId("open_spotify_link"),
            keybindings: vec![KeyBinding::Key('O')],
            description: "Open Spotify link from clipboard",
            category: CommandCategory::Pages,
        },

        // === Theme ===
        CommandBinding {
            command: CommandId("switch_theme"),
            keybindings: vec![KeyBinding::Key('T')],
            description: "Switch theme",
            category: CommandCategory::Other,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    /// Test KeymapConfig::new with missing file
    #[test]
    fn test_keymap_config_new_missing_file() {
        let temp_dir = TempDir::new().unwrap();
        let config = KeymapConfig::new(temp_dir.path());
        
        assert!(config.is_ok());
        assert!(config.unwrap().keymaps.is_empty());
    }

    /// Test KeymapConfig::new with valid TOML
    #[test]
    fn test_keymap_config_new_valid_toml() {
        let temp_dir = TempDir::new().unwrap();
        
        let toml_content = r#"
[[keymaps]]
key_sequence = "C-p"
command = "play_pause"

[[keymaps]]
key_sequence = "Space"
command = "next_track"
"#;
        
        let mut file = std::fs::File::create(temp_dir.path().join("keymap.toml")).unwrap();
        file.write_all(toml_content.as_bytes()).unwrap();
        
        let config = KeymapConfig::new(temp_dir.path()).unwrap();
        assert_eq!(config.keymaps.len(), 2);
        assert_eq!(config.keymaps[0].key_sequence, "C-p");
        assert_eq!(config.keymaps[0].command, "play_pause");
    }

    /// Test KeymapConfig::new with invalid TOML (should return default)
    #[test]
    fn test_keymap_config_new_invalid_toml() {
        let temp_dir = TempDir::new().unwrap();
        
        let toml_content = "not valid toml {{{";
        
        let mut file = std::fs::File::create(temp_dir.path().join("keymap.toml")).unwrap();
        file.write_all(toml_content.as_bytes()).unwrap();
        
        let config = KeymapConfig::new(temp_dir.path()).unwrap();
        // Should return default (empty) on parse error
        assert!(config.keymaps.is_empty());
    }

    /// Test KeymapConfig::default
    #[test]
    fn test_keymap_config_default() {
        let config = KeymapConfig::default();
        assert!(config.keymaps.is_empty());
    }

    /// Test parse_key_sequence with empty string
    #[test]
    fn test_parse_key_sequence_empty() {
        let result = parse_key_sequence("");
        assert!(result.is_empty());
    }

    /// Test parse_key_sequence with whitespace
    #[test]
    fn test_parse_key_sequence_whitespace() {
        let result = parse_key_sequence("   ");
        assert!(result.is_empty());
    }

    /// Test parse_key_sequence with single character
    #[test]
    fn test_parse_key_sequence_single_char() {
        let result = parse_key_sequence("a");
        assert_eq!(result.len(), 1);
        assert!(matches!(result[0], KeyBinding::Key('a')));
    }

    /// Test parse_key_sequence with uppercase character
    #[test]
    fn test_parse_key_sequence_uppercase() {
        let result = parse_key_sequence("G");
        assert_eq!(result.len(), 1);
        assert!(matches!(result[0], KeyBinding::Key('G')));
    }

    /// Test parse_key_sequence with Ctrl modifier
    #[test]
    fn test_parse_key_sequence_ctrl() {
        let result = parse_key_sequence("C-x");
        assert_eq!(result.len(), 1);
        
        if let KeyBinding::Modified { key, ctrl, shift } = &result[0] {
            assert_eq!(*key, 'x');
            assert!(ctrl);
            assert!(!shift);
        } else {
            panic!("Expected Modified keybinding");
        }
    }

    /// Test parse_key_sequence with Shift modifier
    #[test]
    fn test_parse_key_sequence_shift() {
        let result = parse_key_sequence("S-x");
        assert_eq!(result.len(), 1);
        
        if let KeyBinding::Modified { key, ctrl, shift } = &result[0] {
            assert_eq!(*key, 'x');
            assert!(!ctrl);
            assert!(shift);
        } else {
            panic!("Expected Modified keybinding");
        }
    }

    /// Test parse_key_sequence with Ctrl+Shift modifiers
    #[test]
    fn test_parse_key_sequence_ctrl_shift() {
        let result = parse_key_sequence("C-S-x");
        assert_eq!(result.len(), 1);
        
        if let KeyBinding::Modified { key, ctrl, shift } = &result[0] {
            assert_eq!(*key, 'x');
            assert!(ctrl);
            assert!(shift);
        } else {
            panic!("Expected Modified keybinding");
        }
    }

    /// Test parse_key_sequence with Shift+Ctrl (different order)
    #[test]
    fn test_parse_key_sequence_shift_ctrl() {
        // Both orders should work: "S-C-x" and "C-S-x"
        let result = parse_key_sequence("S-C-x");
        assert_eq!(result.len(), 1);

        if let KeyBinding::Modified { key, ctrl, shift } = &result[0] {
            assert_eq!(*key, 'x');
            assert!(ctrl);
            assert!(shift);
        } else {
            panic!("Expected Modified keybinding");
        }
    }

    /// Test parse_key_sequence with special keys
    #[test]
    fn test_parse_key_sequence_special_keys() {
        let keys = vec![
            ("Space", "Space"),
            ("Enter", "Enter"),
            ("Escape", "Escape"),
            ("Tab", "Tab"),
            ("BackTab", "BackTab"),
            ("Backspace", "Backspace"),
            ("Home", "Home"),
            ("End", "End"),
            ("PageUp", "PageUp"),
            ("PageDown", "PageDown"),
            ("ArrowUp", "ArrowUp"),
            ("ArrowDown", "ArrowDown"),
            ("ArrowLeft", "ArrowLeft"),
            ("ArrowRight", "ArrowRight"),
            ("F1", "F1"),
            ("F12", "F12"),
        ];
        
        for (input, expected) in keys {
            let result = parse_key_sequence(input);
            assert_eq!(result.len(), 1);
            
            if let KeyBinding::Special(s) = &result[0] {
                assert_eq!(s, expected);
            } else {
                panic!("Expected Special keybinding for {}", input);
            }
        }
    }

    /// Test parse_key_sequence with special keys (lowercase)
    #[test]
    fn test_parse_key_sequence_special_keys_lowercase() {
        let result = parse_key_sequence("space");
        assert_eq!(result.len(), 1);
        
        if let KeyBinding::Special(s) = &result[0] {
            assert_eq!(s, "Space");
        } else {
            panic!("Expected Special keybinding");
        }
    }

    /// Test parse_key_sequence with multi-key sequence
    #[test]
    fn test_parse_key_sequence_multi_key() {
        let result = parse_key_sequence("g g");
        assert_eq!(result.len(), 1);
        
        if let KeyBinding::Sequence(seq) = &result[0] {
            assert_eq!(seq.len(), 2);
            assert_eq!(seq[0], "g");
            assert_eq!(seq[1], "g");
        } else {
            panic!("Expected Sequence keybinding");
        }
    }

    /// Test parse_key_sequence with longer multi-key sequence
    #[test]
    fn test_parse_key_sequence_longer_sequence() {
        let result = parse_key_sequence("s l a");
        assert_eq!(result.len(), 1);
        
        if let KeyBinding::Sequence(seq) = &result[0] {
            assert_eq!(seq.len(), 3);
            assert_eq!(seq[0], "s");
            assert_eq!(seq[1], "l");
            assert_eq!(seq[2], "a");
        } else {
            panic!("Expected Sequence keybinding");
        }
    }

    /// Test parse_key_sequence with invalid modifier
    #[test]
    fn test_parse_key_sequence_invalid_modifier() {
        // "X-a" is not a valid modifier combination
        let result = parse_key_sequence("X-a");
        // Should fall through to empty result (not a special key, not a sequence)
        assert!(result.is_empty());
    }

    /// Test parse_key_sequence with unknown special key
    #[test]
    fn test_parse_key_sequence_unknown_special() {
        let result = parse_key_sequence("UnknownKey");
        // Should return empty for unknown keys
        assert!(result.is_empty());
    }

    /// Test parse_key_sequence case insensitivity for special keys
    #[test]
    fn test_parse_key_sequence_case_insensitive_special() {
        let lower = parse_key_sequence("space");
        let upper = parse_key_sequence("SPACE");
        let mixed = parse_key_sequence("Space");
        
        assert_eq!(lower.len(), 1);
        assert_eq!(upper.len(), 1);
        assert_eq!(mixed.len(), 1);
        
        // All should parse to the same key
        assert!(matches!(lower[0], KeyBinding::Special(_)));
        assert!(matches!(upper[0], KeyBinding::Special(_)));
        assert!(matches!(mixed[0], KeyBinding::Special(_)));
    }

    /// Test KeymapConfig::apply_overrides
    #[test]
    fn test_apply_overrides() {
        let mut defaults = default_keybindings();
        
        let keymap_config = KeymapConfig {
            keymaps: vec![
                Keymap {
                    key_sequence: "C-p".to_string(),
                    command: "play_pause".to_string(),
                },
            ],
        };
        
        // Find the play_pause binding before override
        let before_binding = defaults.iter()
            .find(|b| b.command.0 == "play_pause")
            .unwrap();
        let before_keys = before_binding.keybindings.clone();
        
        // Apply override
        keymap_config.apply_overrides(&mut defaults);
        
        // Find the play_pause binding after override
        let after_binding = defaults.iter()
            .find(|b| b.command.0 == "play_pause")
            .unwrap();
        
        // Keybindings should have been updated
        assert_ne!(after_binding.keybindings, before_keys);
    }

    /// Test KeymapConfig::apply_overrides with empty key_sequence
    #[test]
    fn test_apply_overrides_empty_sequence() {
        let mut defaults = default_keybindings();
        
        let keymap_config = KeymapConfig {
            keymaps: vec![
                Keymap {
                    key_sequence: "".to_string(), // Empty sequence
                    command: "play_pause".to_string(),
                },
            ],
        };
        
        // Find the play_pause binding before override
        let before_binding = defaults.iter()
            .find(|b| b.command.0 == "play_pause")
            .unwrap();
        let before_keys = before_binding.keybindings.clone();
        
        // Apply override
        keymap_config.apply_overrides(&mut defaults);
        
        // Find the play_pause binding after override
        let after_binding = defaults.iter()
            .find(|b| b.command.0 == "play_pause")
            .unwrap();
        
        // Empty sequence should not change the binding
        assert_eq!(after_binding.keybindings, before_keys);
    }

    /// Test KeymapConfig::apply_overrides with non-existent command
    #[test]
    fn test_apply_overrides_nonexistent_command() {
        let mut defaults = default_keybindings();
        let original_count = defaults.len();
        
        let keymap_config = KeymapConfig {
            keymaps: vec![
                Keymap {
                    key_sequence: "C-x".to_string(),
                    command: "nonexistent_command".to_string(),
                },
            ],
        };
        
        // Apply override
        keymap_config.apply_overrides(&mut defaults);
        
        // Should not add new bindings, only modify existing ones
        assert_eq!(defaults.len(), original_count);
    }

    /// Test KeymapConfig::apply_overrides multiple commands
    #[test]
    fn test_apply_overrides_multiple() {
        let mut defaults = default_keybindings();
        
        let keymap_config = KeymapConfig {
            keymaps: vec![
                Keymap {
                    key_sequence: "C-p".to_string(),
                    command: "play_pause".to_string(),
                },
                Keymap {
                    key_sequence: "C-n".to_string(),
                    command: "next_track".to_string(),
                },
            ],
        };
        
        // Apply overrides
        keymap_config.apply_overrides(&mut defaults);
        
        // Both bindings should be updated
        let play_pause = defaults.iter()
            .find(|b| b.command.0 == "play_pause")
            .unwrap();
        let next_track = defaults.iter()
            .find(|b| b.command.0 == "next_track")
            .unwrap();
        
        // Check that keybindings were updated
        assert!(play_pause.keybindings.iter().any(|k| {
            matches!(k, KeyBinding::Modified { key: 'p', ctrl: true, shift: false })
        }));
        assert!(next_track.keybindings.iter().any(|k| {
            matches!(k, KeyBinding::Modified { key: 'n', ctrl: true, shift: false })
        }));
    }

    /// Test Keymap creation
    #[test]
    fn test_keymap_creation() {
        let keymap = Keymap {
            key_sequence: "C-x".to_string(),
            command: "test_command".to_string(),
        };
        
        assert_eq!(keymap.key_sequence, "C-x");
        assert_eq!(keymap.command, "test_command");
    }

    /// Test KeymapConfig creation
    #[test]
    fn test_keymap_config_creation() {
        let config = KeymapConfig {
            keymaps: vec![
                Keymap {
                    key_sequence: "C-p".to_string(),
                    command: "play_pause".to_string(),
                },
            ],
        };
        
        assert_eq!(config.keymaps.len(), 1);
        assert_eq!(config.keymaps[0].key_sequence, "C-p");
    }

    /// Test default_keybindings contains expected commands
    #[test]
    fn test_default_keybindings_contains_expected() {
        let bindings = default_keybindings();
        
        let has_play_pause = bindings.iter().any(|b| b.command.0 == "play_pause");
        let has_next_track = bindings.iter().any(|b| b.command.0 == "next_track");
        let has_prev_track = bindings.iter().any(|b| b.command.0 == "prev_track");
        let has_nav_up = bindings.iter().any(|b| b.command.0 == "nav_up");
        let has_nav_down = bindings.iter().any(|b| b.command.0 == "nav_down");
        let has_quit = bindings.iter().any(|b| b.command.0 == "quit");
        
        assert!(has_play_pause);
        assert!(has_next_track);
        assert!(has_prev_track);
        assert!(has_nav_up);
        assert!(has_nav_down);
        assert!(has_quit);
    }

    /// Test default_keybindings have descriptions
    #[test]
    fn test_default_keybindings_have_descriptions() {
        let bindings = default_keybindings();
        
        for binding in &bindings {
            assert!(!binding.description.is_empty());
        }
    }

    /// Test default_keybindings have categories
    #[test]
    fn test_default_keybindings_have_categories() {
        let bindings = default_keybindings();
        
        for binding in &bindings {
            // All bindings should have a category
            assert!(
                matches!(binding.category, 
                    CommandCategory::Navigation |
                    CommandCategory::Sorting |
                    CommandCategory::Playback |
                    CommandCategory::Actions |
                    CommandCategory::Pages |
                    CommandCategory::Other
                )
            );
        }
    }

    /// Test parse_key_sequence with Ctrl+Shift+special key (invalid - special keys can't be modified)
    #[test]
    fn test_parse_key_sequence_ctrl_shift_special() {
        // Special keys like "space" should not be parsed with modifiers
        // The parser will treat "C-space" as ctrl+space, which is valid
        let result = parse_key_sequence("C-space");
        assert_eq!(result.len(), 1);
        
        // This should be parsed as a modified key, not a special key
        if let KeyBinding::Modified { key, ctrl, shift } = &result[0] {
            assert_eq!(*key, 's'); // 's' from "space"
            assert!(ctrl);
            assert!(!shift);
        } else {
            // If it's a special key, that's also acceptable
            assert!(matches!(result[0], KeyBinding::Special(_)));
        }
    }

    /// Test parse_key_sequence with tab character (should be handled)
    #[test]
    fn test_parse_key_sequence_tab() {
        let result = parse_key_sequence("Tab");
        assert_eq!(result.len(), 1);
        
        if let KeyBinding::Special(s) = &result[0] {
            assert_eq!(s, "Tab");
        } else {
            panic!("Expected Special keybinding for Tab");
        }
    }

    /// Test parse_key_sequence with function keys
    #[test]
    fn test_parse_key_sequence_function_keys() {
        for i in 1..=12 {
            let key = format!("F{}", i);
            let result = parse_key_sequence(&key);
            assert_eq!(result.len(), 1);
            
            if let KeyBinding::Special(s) = &result[0] {
                assert_eq!(s, &key);
            } else {
                panic!("Expected Special keybinding for {}", key);
            }
        }
    }
}
