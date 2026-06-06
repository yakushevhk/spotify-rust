use super::model::TracksId;
use std::sync::LazyLock;

pub const USER_TOP_TRACKS_URI: &str = "tracks:user-top-tracks";
pub const USER_RECENTLY_PLAYED_TRACKS_URI: &str = "tracks:user-recently-played-tracks";
pub const USER_LIKED_TRACKS_URI: &str = "tracks:user-liked-tracks";

pub static USER_LIKED_TRACKS_ID: LazyLock<TracksId> =
    LazyLock::new(|| TracksId::new(USER_LIKED_TRACKS_URI, "Liked Tracks"));
