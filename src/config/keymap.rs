use serde::Deserialize;

use crate::key::{CommandBinding, CommandCategory, CommandId, KeyBinding};

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
            command: CommandId("enter"),
            keybindings: vec![
                KeyBinding::Special("Enter".to_string()),
            ],
            description: "Play selected / confirm",
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
            keybindings: vec![],
            description: "Toggle shuffle",
            category: CommandCategory::Playback,
        },
        CommandBinding {
            command: CommandId("repeat"),
            keybindings: vec![],
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
