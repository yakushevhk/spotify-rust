use crate::state::model::ContextId;

#[derive(Clone, Debug)]
pub enum PageState {
    Context {
        id: Option<ContextId>,
        context_page_type: ContextPageType,
        state: Option<ContextPageUIState>,
    },
    Lyrics {
        track_uri: String,
        track: String,
        artists: String,
    },
    _Other,
}

#[derive(Clone, Debug)]
pub enum ContextPageType {
    CurrentPlaying,
    Browsing(ContextId),
}

#[derive(Clone, Debug)]
pub enum ContextPageUIState {
    Album,
    Artist,
    Playlist,
    Tracks,
    Show,
}

impl ContextPageUIState {
    pub fn new_album() -> Self {
        Self::Album
    }
    pub fn new_artist() -> Self {
        Self::Artist
    }
    pub fn new_playlist() -> Self {
        Self::Playlist
    }
    pub fn new_tracks() -> Self {
        Self::Tracks
    }
    pub fn new_show() -> Self {
        Self::Show
    }
}
