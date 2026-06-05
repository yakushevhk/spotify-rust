//! Command system for user actions
//!
//! This module defines the command types used throughout the application.
//! Commands are resolved from key sequences and executed by the GUI.
//!
//! # Command Categories
//!
//! - **Navigation** - Moving through the UI (up, down, page up, etc.)
//! - **Playback** - Controlling playback (play, pause, next, etc.)
//! - **Sorting** - Sorting tracks and library items
//! - **Pages** - Switching between views
//! - **Actions** - Context menu actions and operations
//! - **Popup** - Browse popups
//! - **Theme** - Theme switching
//!
//! # Example
//!
//! ```rust
//! // Resolve a command from a key sequence
//! if let Some((cmd, count)) = resolve_command(&command_id, 1) {
//!     execute_command(cmd, count);
//! }
//! ```

use crate::key::CommandId;

#[derive(Clone, Debug)]
pub enum Command {
    Navigation(NavCommand),
    Playback(PlaybackCommand),
    Sorting(SortCommand),
    Page(PageCommand),
    Action(ActionCommand),
    Popup(PopupCommand),
    Theme(ThemeCommand),
}

#[derive(Clone, Debug)]
pub enum ThemeCommand {
    SwitchTheme,
}

#[derive(Clone, Debug)]
pub enum NavCommand {
    Up,
    Down,
    PageUp,
    PageDown,
    First,
    Last,
    FocusNext,
    FocusPrev,
    Back,
    Forward,
    Enter,
    Quit,
    InPageSearch,
}

#[derive(Clone, Debug)]
pub enum PlaybackCommand {
    PlayPause,
    NextTrack,
    PrevTrack,
    RefreshPlayback,
    RestartClient,
    MuteToggle,
    SeekToStart,
    SeekForward,
    SeekBackward,
    PlayRandom,
    Shuffle,
    Repeat,
    VolumeUp,
    VolumeDown,
}

#[derive(Clone, Debug)]
pub enum SortCommand {
    ByTitle,
    ByArtist,
    ByAlbum,
    ByDuration,
    ByAddedDate,
    Reverse,
    LibraryAlphabetical,
    LibraryRecentlyAdded,
}

#[derive(Clone, Debug)]
pub enum PageCommand {
    CurrentlyPlaying,
    TopTracks,
    RecentlyPlayed,
    LikedTracks,
    Library,
    Search,
    Browse,
    Lyrics,
    Queue,
    Logs,
    Help,
    OpenSpotifyLink,
}

#[derive(Clone, Debug)]
pub enum ActionCommand {
    ShowActionsOnSelected,
    ShowActionsOnCurrent,
    ShowActionsOnContext,
    AddToQueue,
    CreatePlaylist,
    JumpToCurrentInContext,
    JumpToHighlightedInContext,
    GoToRadio,
    MovePlaylistItemUp,
    MovePlaylistItemDown,
    SwitchDevice,
}

#[derive(Clone, Debug)]
pub enum PopupCommand {
    Playlists,
    FollowedArtists,
    SavedAlbums,
}

/// Resolve a CommandId to a Command for execution
pub fn resolve_command(id: &CommandId, count: usize) -> Option<(Command, usize)> {
    let cmd = match id.0 {
        // Navigation
        "nav_up" => Command::Navigation(NavCommand::Up),
        "nav_down" => Command::Navigation(NavCommand::Down),
        "page_up" => Command::Navigation(NavCommand::PageUp),
        "page_down" => Command::Navigation(NavCommand::PageDown),
        "first" => Command::Navigation(NavCommand::First),
        "last" => Command::Navigation(NavCommand::Last),
        "focus_next" => Command::Navigation(NavCommand::FocusNext),
        "focus_prev" => Command::Navigation(NavCommand::FocusPrev),
        "back" => Command::Navigation(NavCommand::Back),
        "forward" => Command::Navigation(NavCommand::Forward),
        "enter" => Command::Navigation(NavCommand::Enter),
        "quit" => Command::Navigation(NavCommand::Quit),
        "in_page_search" => Command::Navigation(NavCommand::InPageSearch),

        // Playback
        "play_pause" => Command::Playback(PlaybackCommand::PlayPause),
        "next_track" => Command::Playback(PlaybackCommand::NextTrack),
        "prev_track" => Command::Playback(PlaybackCommand::PrevTrack),
        "refresh_playback" => Command::Playback(PlaybackCommand::RefreshPlayback),
        "restart_client" => Command::Playback(PlaybackCommand::RestartClient),
        "mute_toggle" => Command::Playback(PlaybackCommand::MuteToggle),
        "seek_to_start" => Command::Playback(PlaybackCommand::SeekToStart),
        "seek_forward" => Command::Playback(PlaybackCommand::SeekForward),
        "seek_backward" => Command::Playback(PlaybackCommand::SeekBackward),
        "play_random" => Command::Playback(PlaybackCommand::PlayRandom),
        "shuffle" => Command::Playback(PlaybackCommand::Shuffle),
        "repeat" => Command::Playback(PlaybackCommand::Repeat),
        "volume_up" => Command::Playback(PlaybackCommand::VolumeUp),
        "volume_down" => Command::Playback(PlaybackCommand::VolumeDown),

        // Sorting
        "sort_by_title" => Command::Sorting(SortCommand::ByTitle),
        "sort_by_artist" => Command::Sorting(SortCommand::ByArtist),
        "sort_by_album" => Command::Sorting(SortCommand::ByAlbum),
        "sort_by_duration" => Command::Sorting(SortCommand::ByDuration),
        "sort_by_added_date" => Command::Sorting(SortCommand::ByAddedDate),
        "sort_reverse" => Command::Sorting(SortCommand::Reverse),
        "sort_library_alpha" => Command::Sorting(SortCommand::LibraryAlphabetical),
        "sort_library_recent" => Command::Sorting(SortCommand::LibraryRecentlyAdded),

        // Pages
        "page_currently_playing" => Command::Page(PageCommand::CurrentlyPlaying),
        "page_top_tracks" => Command::Page(PageCommand::TopTracks),
        "page_recently_played" => Command::Page(PageCommand::RecentlyPlayed),
        "page_liked_tracks" => Command::Page(PageCommand::LikedTracks),
        "page_library" => Command::Page(PageCommand::Library),
        "page_search" => Command::Page(PageCommand::Search),
        "page_browse" => Command::Page(PageCommand::Browse),
        "page_lyrics" => Command::Page(PageCommand::Lyrics),
        "page_queue" => Command::Page(PageCommand::Queue),
        "page_logs" => Command::Page(PageCommand::Logs),
        "page_help" => Command::Page(PageCommand::Help),
        "open_spotify_link" => Command::Page(PageCommand::OpenSpotifyLink),

        // Actions
        "show_actions_selected" => Command::Action(ActionCommand::ShowActionsOnSelected),
        "show_actions_current" => Command::Action(ActionCommand::ShowActionsOnCurrent),
        "show_actions_context" => Command::Action(ActionCommand::ShowActionsOnContext),
        "add_to_queue" => Command::Action(ActionCommand::AddToQueue),
        "create_playlist" => Command::Action(ActionCommand::CreatePlaylist),
        "jump_to_current" => Command::Action(ActionCommand::JumpToCurrentInContext),
        "jump_to_highlighted" => Command::Action(ActionCommand::JumpToHighlightedInContext),
        "go_to_radio" => Command::Action(ActionCommand::GoToRadio),
        "move_playlist_item_up" => Command::Action(ActionCommand::MovePlaylistItemUp),
        "move_playlist_item_down" => Command::Action(ActionCommand::MovePlaylistItemDown),
        "switch_device" => Command::Action(ActionCommand::SwitchDevice),

        // Popup
        "browse_user_playlists" => Command::Popup(PopupCommand::Playlists),
        "browse_user_followed_artists" => Command::Popup(PopupCommand::FollowedArtists),
        "browse_user_saved_albums" => Command::Popup(PopupCommand::SavedAlbums),

        // Theme
        "switch_theme" => Command::Theme(ThemeCommand::SwitchTheme),

        _ => return None,
    };
    Some((cmd, count))
}
