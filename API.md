# API Documentation

This document describes the request and command types used in the Spotify Player GUI.

## ClientRequest

`ClientRequest` is the main request type sent from the GUI to the client handler. These requests are processed asynchronously by the client handler task.

```rust
pub enum ClientRequest {
    // User data
    GetCurrentUser,
    GetDevices,
    
    // Browse
    GetBrowseCategories,
    GetBrowseCategoryPlaylists(Category),
    
    // Library
    GetUserPlaylists,
    GetUserSavedAlbums,
    GetUserSavedShows,
    GetUserFollowedArtists,
    
    // Context
    GetContext(ContextId),
    GetCurrentPlayback,
    
    // Search
    Search(String),
    
    // Queue
    AddPlayableToQueue(PlayableId<'static>),
    AddAlbumToQueue(AlbumId<'static>),
    GetCurrentUserQueue,
    
    // Playlist management
    AddPlayableToPlaylist(PlaylistId<'static>, PlayableId<'static>),
    DeleteTrackFromPlaylist(PlaylistId<'static>, TrackId<'static>),
    ReorderPlaylistItems {
        playlist_id: PlaylistId<'static>,
        insert_index: usize,
        range_start: usize,
        range_length: Option<usize>,
        snapshot_id: Option<String>,
    },
    CreatePlaylist {
        playlist_name: String,
        public: bool,
        collab: bool,
        desc: String,
    },
    
    // Library management
    AddToLibrary(Item),
    DeleteFromLibrary(ItemId),
    
    // Player control
    Player(PlayerRequest),
    
    // Lyrics
    GetLyrics {
        track_id: TrackId<'static>,
    },
    
    // Streaming
    #[cfg(feature = "streaming")]
    RestartIntegratedClient,
    
    // Auth
    Logout,
}
```

### Examples

#### Get User Playlists
```rust
client_pub.send(ClientRequest::GetUserPlaylists)?;
```

#### Play a Track
```rust
let playback = Playback::Context(
    ContextId::Playlist(playlist_id),
    Some(Offset::Position(0)),
);
client_pub.send(ClientRequest::Player(
    PlayerRequest::StartPlayback(playback, None)
))?;
```

#### Add to Queue
```rust
client_pub.send(ClientRequest::AddPlayableToQueue(
    PlayableId::Track(track_id)
))?;
```

#### Search
```rust
client_pub.send(ClientRequest::Search("artist:radiohead".to_string()))?;
```

#### Create Playlist
```rust
client_pub.send(ClientRequest::CreatePlaylist {
    playlist_name: "My New Playlist".to_string(),
    public: true,
    collab: false,
    desc: "Created with Spotify Player GUI".to_string(),
})?;
```

## PlayerRequest

`PlayerRequest` controls playback state. These are sent wrapped in `ClientRequest::Player`.

```rust
pub enum PlayerRequest {
    // Track navigation
    NextTrack,
    PreviousTrack,
    
    // Playback state
    Resume,
    Pause,
    ResumePause,  // Toggle between play/pause
    
    // Seeking
    SeekTrack(chrono::Duration),
    
    // Playback modes
    Repeat,   // Cycle: Off -> Track -> Context -> Off
    Shuffle,  // Toggle shuffle on/off
    
    // Volume
    Volume(u8),     // Set volume (0-100)
    ToggleMute,     // Toggle mute (stores restore volume)
    
    // Device transfer
    TransferPlayback(String, bool),  // device_id, force_play
    
    // Start playback
    StartPlayback(Playback, Option<bool>),  // playback, shuffle
}
```

### Examples

#### Toggle Play/Pause
```rust
client_pub.send(ClientRequest::Player(PlayerRequest::ResumePause))?;
```

#### Seek Forward 30 Seconds
```rust
let pos = chrono::Duration::seconds(30);
client_pub.send(ClientRequest::Player(PlayerRequest::SeekTrack(pos)))?;
```

#### Set Volume
```rust
client_pub.send(ClientRequest::Player(PlayerRequest::Volume(75)))?;
```

#### Transfer Playback to Device
```rust
client_pub.send(ClientRequest::Player(
    PlayerRequest::TransferPlayback("device_id".to_string(), true)
))?;
```

#### Start Playlist Playback
```rust
let playback = Playback::Context(
    ContextId::Playlist(playlist_id),
    Some(Offset::Uri(track_uri)),
);
client_pub.send(ClientRequest::Player(
    PlayerRequest::StartPlayback(playback, Some(true))
))?;
```

## Command

`Command` represents high-level user commands. These are resolved from key sequences and executed by the GUI.

```rust
pub enum Command {
    Navigation(NavCommand),
    Playback(PlaybackCommand),
    Sorting(SortCommand),
    Page(PageCommand),
    Action(ActionCommand),
    Popup(PopupCommand),
    Theme(ThemeCommand),
}
```

### Navigation Commands

```rust
pub enum NavCommand {
    Up,           // Move selection up
    Down,         // Move selection down
    PageUp,       // Page up
    PageDown,     // Page down
    First,        // Go to first item (gg)
    Last,         // Go to last item (G)
    FocusNext,    // Focus next window (Tab)
    FocusPrev,    // Focus previous window (BackTab)
    Back,         // Go back (Backspace)
    Forward,      // Go forward (Ctrl+])
    Enter,        // Play selected / confirm
    Quit,         // Quit application (q)
    InPageSearch, // Search within current view (/)
}
```

### Playback Commands

```rust
pub enum PlaybackCommand {
    PlayPause,         // Toggle play/pause (Space)
    NextTrack,         // Next track (→)
    PrevTrack,         // Previous track (←)
    RefreshPlayback,   // Refresh playback state (r)
    RestartClient,     // Restart integrated client (R)
    MuteToggle,        // Toggle mute (_)
    SeekToStart,       // Seek to track start (^)
    SeekForward,       // Seek forward (>)
    SeekBackward,      // Seek backward (<)
    PlayRandom,        // Play random track (.)
    Shuffle,           // Toggle shuffle (Ctrl+s)
    Repeat,            // Toggle repeat (Ctrl+r)
    VolumeUp,          // Volume up (Ctrl+i)
    VolumeDown,        // Volume down (Ctrl+d)
}
```

### Sorting Commands

```rust
pub enum SortCommand {
    ByTitle,             // Sort by title (s t)
    ByArtist,            // Sort by artist (s a)
    ByAlbum,             // Sort by album (s A)
    ByDuration,          // Sort by duration (s d)
    ByAddedDate,         // Sort by added date (s D)
    Reverse,             // Reverse order (s r)
    LibraryAlphabetical, // Sort library A-Z (s l a)
    LibraryRecentlyAdded,// Sort library by date added (s l r)
}
```

### Page Commands

```rust
pub enum PageCommand {
    CurrentlyPlaying,  // Go to current context (g space)
    TopTracks,         // Top tracks page (g t)
    RecentlyPlayed,    // Recently played page (g r)
    LikedTracks,       // Liked tracks page (g y)
    Library,           // Library page (g l)
    Search,            // Search page (g s)
    Browse,            // Browse page (g b)
    Lyrics,            // Lyrics page (g L / l / Ctrl+l)
    Queue,             // Queue page (z / Ctrl+q)
    Logs,              // Logs page (g o)
    Help,              // Help page (? / Ctrl+h)
    OpenSpotifyLink,   // Open Spotify link from clipboard (O)
}
```

### Action Commands

```rust
pub enum ActionCommand {
    ShowActionsOnSelected,    // Show actions for selected item (g a / Ctrl+Space)
    ShowActionsOnCurrent,   // Show actions for current track (a)
    ShowActionsOnContext,   // Show actions for current context (A)
    AddToQueue,             // Add selected to queue (Z / Ctrl+z)
    CreatePlaylist,         // Create new playlist (N)
    JumpToCurrentInContext, // Jump to current track in context (g c)
    JumpToHighlightedInContext, // Jump to highlighted track (Ctrl+g)
    GoToRadio,              // Go to radio for selected (Ctrl+Shift+R)
    MovePlaylistItemUp,     // Move item up in playlist (Ctrl+k)
    MovePlaylistItemDown,   // Move item down in playlist (Ctrl+j)
    SwitchDevice,           // Switch playback device (D)
}
```

### Popup Commands

```rust
pub enum PopupCommand {
    BrowseUserPlaylists,       // Browse playlists (u p)
    BrowseUserFollowedArtists, // Browse artists (u a)
    BrowseUserSavedAlbums,     // Browse albums (u A)
}
```

### Theme Commands

```rust
pub enum ThemeCommand {
    SwitchTheme,  // Switch theme (T)
}
```

## ContextId

`ContextId` identifies a Spotify context (playlist, album, artist, etc.).

```rust
pub enum ContextId {
    Playlist(PlaylistId<'static>),
    Album(AlbumId<'static>),
    Artist(ArtistId<'static>),
    Tracks(TracksId),      // Special tracks collection
    Show(ShowId<'static>), // Podcast show
}
```

### Examples

```rust
// Playlist context
let ctx = ContextId::Playlist(PlaylistId::from_id("37i9dQZF1DXcBWIGoYBM5M")?.into_static());

// Album context
let ctx = ContextId::Album(AlbumId::from_id("4uLU6hMCjMI75M1A2tKUQC")?.into_static());

// Artist context
let ctx = ContextId::Artist(ArtistId::from_id("0TnOYISbd1XYRBk9myaseg")?.into_static());

// Special tracks (liked, recently played, etc.)
let ctx = ContextId::Tracks(TracksId::new(
    "spotify:user:liked_tracks",
    "Liked Tracks"
));
```

## Playback

`Playback` defines what to play and where to start.

```rust
pub enum Playback {
    /// Play a context with optional offset
    Context(ContextId, Option<Offset>),
    
    /// Play specific track URIs with optional offset
    URIs(Vec<PlayableId<'static>>, Option<Offset>),
}
```

### Offset

```rust
// From rspotify::model::Offset
pub enum Offset {
    Position(u32),  // Start at position N
    Uri(String),    // Start at specific track URI
}
```

### Examples

```rust
// Play playlist from beginning
let playback = Playback::Context(
    ContextId::Playlist(playlist_id),
    None
);

// Play album starting at track 5
let playback = Playback::Context(
    ContextId::Album(album_id),
    Some(Offset::Position(4))  // 0-indexed
);

// Play specific tracks starting at second track
let tracks = vec![
    PlayableId::Track(track1_id),
    PlayableId::Track(track2_id),
    PlayableId::Track(track3_id),
];
let playback = Playback::URIs(tracks, Some(Offset::Position(1)));
```

## Item and ItemId

These types represent Spotify items that can be added to the library.

```rust
pub enum Item {
    Track(Track),
    Album(Album),
    Artist(Artist),
    Playlist(Playlist),
    Show(Show),
}

pub enum ItemId {
    Track(TrackId<'static>),
    Album(AlbumId<'static>),
    Artist(ArtistId<'static>),
    Playlist(PlaylistId<'static>),
    Show(ShowId<'static>),
}
```

### Examples

```rust
// Add track to library
let item = Item::Track(track);
client_pub.send(ClientRequest::AddToLibrary(item))?;

// Remove album from library
let item_id = ItemId::Album(album_id);
client_pub.send(ClientRequest::DeleteFromLibrary(item_id))?;
```

## KeyBinding

Key bindings define how user input maps to commands.

```rust
pub enum KeyBinding {
    /// Single character key (e.g., 'j', 'k')
    Key(char),
    
    /// Key with modifiers (e.g., Ctrl+f, Shift+G)
    Modified { key: char, ctrl: bool, shift: bool },
    
    /// Special key (e.g., "Space", "Enter", "Home")
    Special(String),
    
    /// Multi-key sequence (e.g., "gg", "g t")
    Sequence(Vec<String>),
}
```

### Key Sequence Format

Key sequences in `keymap.toml` use the following format:

| Format | Description | Example |
|--------|-------------|---------|
| `x` | Single character | `j`, `k` |
| `C-x` | Ctrl + key | `C-f` |
| `S-x` | Shift + key | `S-g` |
| `C-S-x` | Ctrl + Shift + key | `C-S-r` |
| `Space` | Space key | `Space` |
| `Enter` | Enter key | `Enter` |
| `x y` | Multi-key sequence | `g g` |

### Examples

```toml
# Single key
[[keymaps]]
key_sequence = "j"
command = "nav_down"

# Ctrl modifier
[[keymaps]]
key_sequence = "C-p"
command = "play_pause"

# Multi-key sequence
[[keymaps]]
key_sequence = "g t"
command = "page_top_tracks"

# Special key
[[keymaps]]
key_sequence = "Space"
command = "play_pause"
```

## Error Handling

All client requests return `anyhow::Result<()>`:

```rust
match client_pub.send(request) {
    Ok(()) => println!("Request sent successfully"),
    Err(e) => eprintln!("Failed to send request: {}", e),
}
```

Common error types:
- **Authentication errors** - Session expired, re-authentication needed
- **Network errors** - Connection issues, timeout
- **API errors** - Rate limiting, invalid parameters
- **Not found errors** - Track/playlist doesn't exist

## Best Practices

1. **Use try_send for non-blocking** - The channel has a buffer of 1024, use `try_send` to avoid blocking the UI
2. **Handle errors gracefully** - Show user-friendly error messages
3. **Use proper IDs** - Always use `'static` lifetime for IDs sent to the client
4. **Check feature flags** - Some requests require specific features (e.g., `RestartIntegratedClient` needs `streaming`)
5. **Debounce rapid requests** - Don't flood the API with requests
