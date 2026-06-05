//! Request types for client communication
//!
//! This module defines the request types used to communicate between the GUI
//! and the client handler. All requests are sent via a channel and processed
//! asynchronously.
//!
//! # Request Flow
//!
//! ```
//! GUI Thread -> Channel -> Client Handler Task -> Spotify API
//! ```
//!
//! # Example
//!
//! ```rust
//! // Send a playback request
//! client_pub.send(ClientRequest::Player(PlayerRequest::NextTrack))?;
//!
//! // Send a search request
//! client_pub.send(ClientRequest::Search("radiohead".to_string()))?;
//! ```

use crate::state::{
    AlbumId, Category, ContextId, Item, ItemId, PlayableId, Playback, PlaylistId, TrackId,
};

#[derive(Clone, Debug)]
#[non_exhaustive]
/// A request that modifies the player's playback
///
/// These requests control the playback state and are sent via
/// `ClientRequest::Player(PlayerRequest)`.
///
/// # Examples
///
/// ```rust
/// // Toggle play/pause
/// client_pub.send(ClientRequest::Player(PlayerRequest::ResumePause))?;
///
/// // Next track
/// client_pub.send(ClientRequest::Player(PlayerRequest::NextTrack))?;
///
/// // Set volume to 75%
/// client_pub.send(ClientRequest::Player(PlayerRequest::Volume(75)))?;
/// ```
pub enum PlayerRequest {
    NextTrack,
    PreviousTrack,
    Resume,
    Pause,
    ResumePause,
    SeekTrack(chrono::Duration),
    Repeat,
    Shuffle,
    Volume(u8),
    /// Toggle mute on/off.
    ///
    /// Note: When the current volume is 0, toggling mute will restore the
    /// volume to 50% (the default). This is because there is no "previous
    /// volume" stored — mute state is tracked separately from volume level
    /// in `PlaybackMetadata::mute_state`.
    ToggleMute,
    TransferPlayback(String, bool),
    StartPlayback(Playback, Option<bool>),
}

#[derive(Clone, Debug)]
#[non_exhaustive]
/// A request to the client
///
/// These requests are sent from the GUI to the client handler via a channel.
/// The client handler processes them asynchronously and updates state accordingly.
///
/// # Examples
///
/// ```rust
/// // Get user playlists
/// client_pub.send(ClientRequest::GetUserPlaylists)?;
///
/// // Search for tracks
/// client_pub.send(ClientRequest::Search("radiohead".to_string()))?;
///
/// // Add track to queue
/// client_pub.send(ClientRequest::AddPlayableToQueue(
///     PlayableId::Track(track_id)
/// ))?;
/// ```
///
/// # Error Handling
///
/// Errors are logged and may trigger toast notifications. The channel
/// has a buffer of 1024 requests; use `try_send` to avoid blocking.
pub enum ClientRequest {
    GetCurrentUser,
    GetDevices,
    GetBrowseCategories,
    GetBrowseCategoryPlaylists(Category),
    GetUserPlaylists,
    GetUserSavedAlbums,
    GetUserSavedShows,
    GetUserFollowedArtists,
    GetContext(ContextId),
    GetCurrentPlayback,
    Search(String),
    AddPlayableToQueue(PlayableId<'static>),
    AddAlbumToQueue(AlbumId<'static>),
    AddPlayableToPlaylist(PlaylistId<'static>, PlayableId<'static>),
    DeleteTrackFromPlaylist(PlaylistId<'static>, TrackId<'static>),
    ReorderPlaylistItems {
        playlist_id: PlaylistId<'static>,
        insert_index: usize,
        range_start: usize,
        range_length: Option<usize>,
        snapshot_id: Option<String>,
    },
    AddToLibrary(Box<Item>),
    DeleteFromLibrary(ItemId),
    Player(PlayerRequest),
    GetCurrentUserQueue,
    GetLyrics {
        track_id: TrackId<'static>,
    },
    #[cfg(feature = "streaming")]
    RestartIntegratedClient,
    CreatePlaylist {
        playlist_name: String,
        public: bool,
        collab: bool,
        desc: String,
    },
    #[allow(dead_code)]
    Logout,
}
