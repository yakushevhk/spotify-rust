use std::borrow::Cow;

pub fn map_join<T, F>(v: &[T], f: F, sep: &str) -> String
where
    F: Fn(&T) -> &str,
{
    v.iter().map(f).collect::<Vec<_>>().join(sep)
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

pub fn parse_uri(uri: &str) -> Cow<'_, str> {
    let parts = uri.split(':').collect::<Vec<_>>();
    if parts.len() == 5 {
        Cow::Owned([parts[0], parts[3], parts[4]].join(":"))
    } else {
        Cow::Borrowed(uri)
    }
}
