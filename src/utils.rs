use std::borrow::Cow;

pub fn map_join<T, F>(v: &[T], f: F, sep: &str) -> String
where
    F: Fn(&T) -> &str,
{
    let mut iter = v.iter().map(f);
    let mut out = String::new();
    if let Some(first) = iter.next() {
        out.push_str(first);
    }
    for item in iter {
        out.push_str(sep);
        out.push_str(item);
    }
    out
}

pub fn get_track_album_image_url(track: &rspotify::model::FullTrack) -> Option<&str> {
    if track.album.images.is_empty() {
        None
    } else {
        Some(&track.album.images[0].url)
    }
}

pub fn get_episode_show_image_url(episode: &rspotify::model::FullEpisode) -> Option<&str> {
    if episode.show.images.is_empty() {
        None
    } else {
        Some(&episode.show.images[0].url)
    }
}

/// Parses a Spotify URI of the form "spotify:<type>:<scope>:<id>:<extra>"
/// and returns a shortened form "spotify:<type>:<id>:<extra>".
/// NOTE: On malformed input not matching 5 colon-separated parts, the original
/// URI is returned unchanged (not garbage — this is intentional for backward compat).
pub fn parse_uri(uri: &str) -> Cow<'_, str> {
    let parts = uri.split(':').collect::<Vec<_>>();
    if parts.len() == 5 {
        Cow::Owned([parts[0], parts[3], parts[4]].join(":"))
    } else {
        Cow::Borrowed(uri)
    }
}
