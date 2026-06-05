# Architecture Overview

This document describes the high-level architecture of the Spotify Player GUI application.

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Spotify Player GUI                              │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │     GUI      │  │   Command    │  │    State     │  │    Config    │     │
│  │   (egui)     │  │   Handler    │  │   Manager    │  │   Manager    │     │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘     │
│         │                │                │                │               │
│  ┌──────┴────────────────┴────────────────┴────────────────┴───────┐    │
│  │                         Shared State (Arc<State>)                    │    │
│  └──────────────────────────────────────────────────────────────────────┘    │
│                                    │                                       │
│  ┌─────────────────────────────────┴──────────────────────────────────┐    │
│  │                         Client Handler Task                         │    │
│  │  ┌──────────────┐  ┌──────────────┐  ┌────────────────────────────┐  │    │
│  │  │  AppClient   │  │   Spotify    │  │   UserClient (rspotify)  │  │    │
│  │  │  (librespot) │  │   Session    │  │   (OAuth PKCE)           │  │    │
│  │  └──────────────┘  └──────────────┘  └────────────────────────────┘  │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                    │                                       │
│  ┌─────────────────────────────────┴──────────────────────────────────┐    │
│  │                      Player Event Watcher Thread                    │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                    │                                       │
│  ┌─────────────────────────────────┴──────────────────────────────────┐    │
│  │                    Media Control Thread (optional)                  │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                              Spotify API
                                    │
                              Spotify Servers
```

## Component Overview

### 1. GUI Layer (`src/gui/`)

The GUI layer is built on [egui](https://github.com/emilk/egui), an immediate mode GUI library for Rust.

**Key Components:**
- `SpotifyApp` - Main application struct implementing `eframe::App`
- `View` enum - Different views (Library, Tracks, Search, Browse, etc.)
- `views.rs` - View rendering implementations
- `sidebar.rs` - Navigation sidebar
- `playback_bar.rs` - Playback controls at bottom
- `command_palette.rs` - Quick command access
- `context_menu.rs` - Right-click context menus
- `image_cache.rs` - Album art caching

**Responsibilities:**
- Render the user interface
- Handle user input (keyboard, mouse)
- Display application state
- Send commands to client handler

### 2. State Management (`src/state/`)

Centralized state management using shared state pattern.

**Key Components:**
- `State` - Main application state container
- `UIState` - UI-specific state (current view, selections)
- `PlayerState` - Playback state (current track, position, devices)
- `AppData` - Application data (playlists, albums, cache)
- `UserData` - User-specific data

**State Access Pattern:**
```rust
// UI State - Mutex for exclusive access
let ui = state.ui.lock();

// Player State - RwLock for read-heavy access
let player = state.player.read();
let mut player = state.player.write();

// Data - RwLock for read-heavy access
let data = state.data.read();
```

**Lock Hierarchy (to prevent deadlocks):**
1. `state.ui` (Mutex)
2. `state.player` (RwLock)
3. `state.data` (RwLock)
4. `state.toast_queue` (Mutex)
5. `stream_conn` (Mutex) - streaming feature only

### 3. Client Layer (`src/client/`)

Handles all communication with Spotify.

**Key Components:**
- `AppClient` - Main client struct
- `spotify.rs` - librespot integration
- `handlers.rs` - Request handlers
- `request.rs` - Request types (`ClientRequest`, `PlayerRequest`)

**Dual Client Architecture:**
1. **librespot Client** - For streaming and low-level Spotify protocol
2. **rspotify Client** - For Web API calls (OAuth PKCE)

**Communication Pattern:**
```rust
// GUI sends requests via channel
client_pub.send(ClientRequest::Player(PlayerRequest::NextTrack))?;

// Client handler processes requests asynchronously
async fn handle_request(&self, state: &SharedState, request: ClientRequest) -> Result<()>
```

### 4. Configuration (`src/config/`)

Configuration management with file-based persistence.

**Key Components:**
- `Configs` - Configuration container
- `AppConfig` - Application settings
- `ThemeConfig` - Color themes
- `KeymapConfig` - Keyboard shortcuts

**Configuration Files:**
- `~/.config/spotify-player/app.toml` - Application settings
- `~/.config/spotify-player/theme.toml` - Custom themes
- `~/.config/spotify-player/keymap.toml` - Key bindings

### 5. Authentication (`src/auth.rs`)

OAuth authentication using librespot's OAuth flow.

**Key Components:**
- `AuthConfig` - Authentication configuration
- `get_creds()` - Credential retrieval
- OAuth scopes for Spotify API access

**Authentication Flow:**
1. Check for cached credentials
2. If none, open browser for OAuth
3. User authenticates with Spotify
4. Token is cached for future use

### 6. Command System (`src/command.rs`, `src/key.rs`)

Vim-inspired command system with key sequences.

**Key Components:**
- `Command` enum - All available commands
- `KeySequenceState` - Multi-key sequence handling
- `CommandBinding` - Key binding definitions

**Command Categories:**
- Navigation (up, down, page up, etc.)
- Playback (play, pause, next, etc.)
- Sorting (sort by title, artist, etc.)
- Pages (switch views)
- Actions (add to queue, create playlist, etc.)

### 7. Streaming (`src/streaming.rs`, `src/ui/streaming.rs`)

Optional local audio streaming using librespot.

**Key Components:**
- `new_connection()` - Create streaming connection
- `VisualizationSink` - Audio visualization support
- `VisBands` - FFT frequency band data

**Feature Flag:** `streaming`

### 8. Media Control (`src/media_control.rs`)

System media key support using souvlaki.

**Feature Flag:** `media-control`

## Data Flow

### 1. User Action to Spotify API

```
User Input (Keyboard/Mouse)
    ↓
GUI Event Handler (SpotifyApp::update)
    ↓
Command Resolution (key.rs)
    ↓
ClientRequest Creation
    ↓
Channel Send (flume::Sender)
    ↓
Client Handler Task (client/mod.rs)
    ↓
Spotify API Call (rspotify/librespot)
    ↓
State Update
    ↓
GUI Re-render
```

### 2. Playback State Updates

```
Spotify API
    ↓
Player Event Watcher Thread
    ↓
State.player.write()
    ↓
GUI Reads State.player.read()
    ↓
Playback Bar Update
```

### 3. Track Playback

```
User Selects Track
    ↓
GUI Sends ClientRequest::Player(StartPlayback)
    ↓
Client Handler
    ↓
Spotify API: Start Playback
    ↓
Player Event Watcher Picks Up Change
    ↓
State Updated
    ↓
GUI Displays New Track
```

## Threading Model

### Main Thread
- Runs the egui event loop (`eframe::run_native`)
- Handles all UI rendering and user input
- Must not block on I/O

### Tokio Runtime Thread
- Runs async client handler task
- Processes `ClientRequest` messages
- Makes async Spotify API calls

### Player Event Watcher Thread
- Polls Spotify playback state
- Updates `PlayerState` periodically
- Runs independently of UI

### Media Control Thread (Optional)
- Listens to system media key events
- Sends commands to main app

### Signal Handler Task
- Handles Ctrl+C and SIGTERM
- Initiates clean shutdown

## Caching Strategy

### Memory Caches (`MemoryCaches`)
- **Context Cache** - Playlist/album/artist data (TTL: 1 hour)
- **Search Cache** - Search results (TTL: 1 hour)
- **Lyrics Cache** - Track lyrics (TTL: 1 hour)
- **Genres Cache** - Artist genres (TTL: 1 hour)
- **Image Cache** - Album artwork (optional, with `image` feature)

### File Caches
Stored in `~/.cache/spotify-player/`:
- `credentials.json` - Spotify credentials
- `user_client_token.json` - OAuth tokens
- `*_cache.json` - Serialized data (playlists, albums, etc.)
- `audio/` - Cached audio files (if enabled)
- `image/` - Cached album art (if enabled)

## Security Considerations

1. **Credential Storage**
   - Credentials stored in `~/.cache/spotify-player/`
   - Unix permissions set to 0o700 (owner only)
   - Token files set to 0o600

2. **OAuth Flow**
   - Uses PKCE for secure authentication
   - Client ID can be user-provided
   - Falls back to ncspot's client ID

3. **Command Execution**
   - `client_id_command` whitelist for security
   - Validates absolute paths
   - Rejects shell metacharacters

4. **Image Filename Sanitization**
   - Prevents path traversal attacks
   - Sanitizes special characters
   - Limits filename length

## Error Handling

### Error Propagation
- Uses `anyhow` for error handling
- Errors logged with `tracing`
- User-facing errors shown as toast notifications

### Recovery Strategies
1. **Session Invalid** → Re-authenticate automatically
2. **Network Error** → Retry with exponential backoff
3. **API Rate Limit** → Cooldown and retry
4. **Channel Full** → Drop request, show warning

## Performance Optimizations

1. **Lazy Loading**
   - Images loaded on-demand
   - Context data fetched when needed
   - Search debounced

2. **Caching**
   - Memory caches with TTL
   - File-based persistent cache
   - Image disk cache

3. **Efficient Rendering**
   - Cached lowercase strings for sorting
   - Pre-computed display strings
   - egui's immediate mode efficiency

4. **Async Operations**
   - Non-blocking API calls
   - Channel-based communication
   - Background state updates

## Feature Flags

| Feature | Description | Default |
|---------|-------------|---------|
| `streaming` | Local audio streaming | ✓ |
| `media-control` | System media key support | ✓ |
| `notify` | Desktop notifications | ✓ |
| `image` | Image support | ✗ |
| `pixelate` | Pixelated image effect | ✗ |
| `fzf` | Fuzzy matching | ✗ |

## Extension Points

1. **Custom Themes** - Add themes to `theme.toml`
2. **Custom Keybindings** - Override in `keymap.toml`
3. **New Views** - Add to `View` enum and implement rendering
4. **New Commands** - Add to `Command` enum and handler
5. **Custom Streaming** - Implement custom audio sinks
