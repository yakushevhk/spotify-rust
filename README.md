# Spotify Player GUI

A native macOS Spotify player with a dark, modern GUI built in Rust using the [egui](https://github.com/emilk/egui) framework.

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-2021-orange.svg)
![Platform](https://img.shields.io/badge/platform-macos%20%7C%20linux%20%7C%20windows-lightgrey.svg)

## Features

- **Modern Dark GUI**: Clean, Spotify-inspired dark interface with customizable themes
- **Full Playback Control**: Play, pause, skip, seek, shuffle, repeat, volume control
- **Library Management**: Browse playlists, albums, artists, and saved tracks
- **Search**: Search for tracks, albums, artists, playlists, and podcasts
- **Browse**: Explore Spotify's browse categories and featured playlists
- **Podcast Support**: Browse and play podcasts and episodes
- **Keyboard Shortcuts**: Vim-inspired keybindings with extensive customization
- **Audio Streaming**: Optional local audio streaming via librespot (with `streaming` feature)
- **Media Controls**: System media key support (with `media-control` feature)
- **Notifications**: Desktop notifications on track change (with `notify` feature)
- **Lyrics**: Display synchronized lyrics for tracks
- **Queue Management**: View and manage playback queue
- **Device Switching**: Switch between Spotify Connect devices
- **Playlist Management**: Create playlists, add/remove tracks, reorder items

## Screenshots

*Screenshots will be added here*

## Installation

### Prerequisites

- **Rust**: Install via [rustup](https://rustup.rs/)
- **Spotify Account**: Free or Premium account required

### Building from Source

```bash
# Clone the repository
git clone https://github.com/yourusername/spotify-player-gui.git
cd spotify-player-gui

# Build with default features (streaming, media-control, notify)
cargo build --release

# Or build with specific features
cargo build --release --features "streaming,media-control,notify"
```

### Running

```bash
# Run the application
cargo run --release

# Or run the built binary
./target/release/spotify-player-gui
```

## Configuration

Configuration files are stored in `~/.config/spotify-player/`:

- `app.toml` - Application settings
- `theme.toml` - Custom color themes
- `keymap.toml` - Custom keyboard shortcuts

### app.toml

```toml
# Theme name (built-in: "Spotify", "Dracula", or custom from theme.toml)
theme = "Spotify"

# Spotify OAuth client ID (optional, uses ncspot's ID by default)
# client_id = "your-client-id"

# OAuth redirect URI
login_redirect_uri = "http://127.0.0.1:8989/login"

# Playback settings
tracks_playback_limit = 50
seek_duration_secs = 5
volume_scroll_step = 5

# Refresh intervals (milliseconds)
app_refresh_duration_in_ms = 32
playback_refresh_duration_in_ms = 5000

# Device settings
default_device = "spotify-player"

# Feature flags
enable_streaming = "Always"  # "Always", "DaemonOnly", "Never"
enable_media_control = true
enable_notify = true
enable_cover_image_cache = true
custom_queue = true

# Device configuration
[device]
name = "spotify-player"
device_type = "speaker"
volume = 70
bitrate = 320
audio_cache = false
normalization = false
autoplay = false

# Layout configuration
[layout]
[layout.library]
playlist_percent = 40
album_percent = 40
```

### theme.toml

```toml
[[themes]]
name = "MyCustomTheme"
[palette]
background = "#121212"
foreground = "#ffffff"
accent = "#1ed760"
accent_hover = "#2de16e"
accent_dark = "#19b450"
bg_dark = "#000000"
bg_card = "#181818"
bg_hover = "#282828"
bg_active = "#333333"
bg_elevated = "#242424"
bg_input = "#2a2a2a"
bg_selected = "#1a1a1a"
text_primary = "#ffffff"
text_secondary = "#b3b3b3"
text_dim = "#737373"
text_muted = "#4d4d4d"
text_hint = "#666666"
border = "#2a2a2a"
divider = "#333333"
success = "#1ed760"
error = "#ff5050"
warning = "#ffc832"
lyrics_current = "#1db954"
lyrics_played = "#555555"
lyrics_upcoming = "#cccccc"
lyrics_bg = "#000000"
```

### keymap.toml

```toml
[[keymaps]]
key_sequence = "C-p"
command = "play_pause"

[[keymaps]]
key_sequence = "C-n"
command = "next_track"

[[keymaps]]
key_sequence = "C-b"
command = "prev_track"
```

## Keyboard Shortcuts

### Navigation

| Key | Action |
|-----|--------|
| `j` / `↓` | Move selection down |
| `k` / `↑` | Move selection up |
| `gg` / `Home` | Go to first item |
| `G` / `End` | Go to last item |
| `Ctrl+f` / `PageDown` | Page down |
| `Ctrl+b` / `PageUp` | Page up |
| `Tab` | Focus next |
| `BackTab` | Focus previous |
| `Backspace` / `Ctrl+q` | Go back |
| `Ctrl+]` | Go forward |
| `Enter` | Play selected / confirm |
| `q` / `Ctrl+c` | Quit |
| `/` | In-page search |

### Playback

| Key | Action |
|-----|--------|
| `Space` | Play / Pause |
| `→` | Next track |
| `←` | Previous track |
| `r` | Refresh playback |
| `R` | Restart integrated client |
| `_` | Toggle mute |
| `^` | Seek to start |
| `>` | Seek forward |
| `<` | Seek backward |
| `.` | Play random track |
| `Ctrl+s` | Toggle shuffle |
| `Ctrl+r` | Toggle repeat |
| `Ctrl+i` | Volume up |
| `Ctrl+d` | Volume down |

### Sorting

| Key | Action |
|-----|--------|
| `s t` | Sort by title |
| `s a` | Sort by artist |
| `s A` | Sort by album |
| `s d` | Sort by duration |
| `s D` | Sort by added date |
| `s r` | Reverse order |
| `s l a` | Sort library alphabetically |
| `s l r` | Sort library by recently added |

### Pages

| Key | Action |
|-----|--------|
| `g ` ` `(space) | Currently playing |
| `g t` | Top tracks |
| `g r` | Recently played |
| `g y` | Liked tracks |
| `g l` | Library |
| `g s` | Search |
| `g b` | Browse |
| `g L` / `l` / `Ctrl+l` | Lyrics |
| `z` / `Ctrl+q` | Queue |
| `g o` | Logs |
| `?` / `Ctrl+h` | Help |
| `O` | Open Spotify link |

### Actions

| Key | Action |
|-----|--------|
| `g a` / `Ctrl+Space` | Show actions on selected |
| `a` | Show actions on current track |
| `A` | Show actions on context |
| `Z` / `Ctrl+z` | Add to queue |
| `N` | Create playlist |
| `g c` | Jump to current track |
| `Ctrl+g` | Jump to highlighted track |
| `Ctrl+Shift+R` | Go to radio |
| `Ctrl+k` | Move playlist item up |
| `Ctrl+j` | Move playlist item down |
| `D` | Switch device |

### Browse

| Key | Action |
|-----|--------|
| `u p` | Browse user playlists |
| `u a` | Browse followed artists |
| `u A` | Browse saved albums |

### Theme

| Key | Action |
|-----|--------|
| `T` | Switch theme |

## Troubleshooting

### Authentication Issues

**Problem**: "No cached credentials found" error

**Solution**: 
1. Delete `~/.cache/spotify-player/credentials.json`
2. Restart the application
3. Authenticate via the browser popup

### Streaming Issues

**Problem**: No audio when streaming locally

**Solution**:
1. Check that `enable_streaming = "Always"` in `app.toml`
2. Verify your audio device is working
3. Check logs with `g o` for audio backend errors

### Connection Issues

**Problem**: "Failed to connect to session"

**Solution**:
1. Check your internet connection
2. Verify Spotify's servers are up
3. Try restarting the application
4. Check if a firewall is blocking connections

### Media Keys Not Working

**Problem**: System media keys don't control playback

**Solution**:
1. Enable `enable_media_control = true` in `app.toml`
2. On macOS/Windows, you may need to grant accessibility permissions
3. Restart the application after enabling

### High CPU Usage

**Problem**: Application uses excessive CPU

**Solution**:
1. Disable audio visualization: `enable_audio_visualization = false`
2. Increase refresh intervals in `app.toml`
3. Check logs for any error loops

## Contributing

Contributions are welcome! Please follow these guidelines:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Run tests (`cargo test`)
5. Commit your changes (`git commit -m 'Add amazing feature'`)
6. Push to the branch (`git push origin feature/amazing-feature`)
7. Open a Pull Request

### Development

```bash
# Run with debug logging
RUST_LOG=debug cargo run

# Run tests
cargo test

# Run with specific features
cargo run --features "streaming,notify"

# Build documentation
cargo doc --open
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [rspotify](https://github.com/ramsayleung/rspotify) - Spotify Web API wrapper for Rust
- [librespot](https://github.com/librespot-org/librespot) - Open source Spotify client library
- [egui](https://github.com/emilk/egui) - Immediate mode GUI library for Rust
- [ncspot](https://github.com/hrkfdn/ncspot) - Inspiration and ncspot client ID

## Support

For issues, questions, or feature requests, please [open an issue](https://github.com/yourusername/spotify-player-gui/issues).
