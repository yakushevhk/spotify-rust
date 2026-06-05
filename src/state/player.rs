use super::model::{
    AlbumId, ArtistId, ContextId, Device, PlaybackMetadata, PlaylistId, ShowId, TracksId,
};
use super::queue::CustomQueue;

/// Player state
#[derive(Default, Debug)]
pub struct PlayerState {
    pub devices: Vec<Device>,

    pub playback: Option<rspotify::model::CurrentPlaybackContext>,
    pub playback_last_updated_time: Option<std::time::Instant>,
    /// A buffered state to speedup the feedback of playback metadata update to user
    // Related issue: https://github.com/aome510/spotify-player/issues/109
    pub buffered_playback: Option<PlaybackMetadata>,

    pub queue: Option<rspotify::model::CurrentUserQueue>,

    /// The currently playing Tracks context (for contexts not tracked by Spotify's playback, e.g. liked/top tracks)
    pub currently_playing_tracks_id: Option<TracksId>,

    /// App-managed custom queue for full playlist/album playback.
    /// Active when the integrated librespot player is streaming and the user
    /// started playback from a track-table context.
    pub custom_queue: Option<CustomQueue>,

    /// Monotonically increasing generation counter incremented each time a new
    /// streaming connection is established. Used by player_event_task to detect
    /// and ignore stale writes after a connection restart.
    pub streaming_generation: u64,

    /// Deadline after which progress estimation resumes after a seek.
    /// Set to `Some(Instant::now() + 500ms)` on seek to prevent the progress
    /// bar from racing ahead of the actual audio position.
    pub seek_deadline: Option<std::time::Instant>,
}

impl PlayerState {
    /// Get the current playback
    ///
    /// # Note
    /// Because playback metadata stored inside the player state is buffered,
    /// the returned playback is estimated based on the available data.
    ///
    /// The `is_playing` field is intentionally sourced from the server state
    /// (not `buffered_playback`) because the server is authoritative for
    /// play/pause. `buffered_playback` may briefly reflect a user-initiated
    /// toggle before the server confirms, but using it here would cause
    /// flickering when the server state lags behind.
    pub fn current_playback(&self) -> Option<rspotify::model::CurrentPlaybackContext> {
        let mut playback = self.playback.clone()?;

        let seeking = self
            .seek_deadline
            .is_some_and(|d| d > std::time::Instant::now());

        playback.progress = match (playback.progress, self.playback_last_updated_time) {
            (Some(d), Some(last_time)) if playback.is_playing && !seeking => {
                chrono::Duration::from_std(last_time.elapsed())
                    .ok()
                    .map(|elapsed| d + elapsed)
            }
            (Some(d), _) => Some(d),
            _ => None,
        };

        // Clamp progress to not exceed the track's duration.
        // This prevents visual overflow after laptop sleep or long pauses.
        if let (Some(ref progress), Some(ref item)) = (playback.progress, &playback.item) {
            let duration = match item {
                rspotify::model::PlayableItem::Track(t) => t.duration,
                rspotify::model::PlayableItem::Episode(e) => e.duration,
                rspotify::model::PlayableItem::Unknown(_) => chrono::Duration::MAX,
            };
            if *progress > duration {
                playback.progress = Some(duration);
            }
        }

        // update the playback's metadata based on the `buffered_playback` metadata
        // NOTE: is_playing is intentionally NOT overridden from buffered_playback.
        // The server is the source of truth for play/pause state; buffered_playback
        // only provides progress estimation, device info, repeat, and shuffle.
        // NOTE: mute_state is also not propagated here because the server's
        // CurrentPlaybackContext has no mute concept — mute_state lives only in
        // buffered_playback (set by ToggleMute handler). Callers that need mute
        // state must read buffered_playback directly.
        if let Some(ref p) = self.buffered_playback {
            playback.device.name.clone_from(&p.device_name);
            playback.device.id.clone_from(&p.device_id);
            playback.device.volume_percent = p.volume;
            playback.repeat_state = p.repeat_state;
            playback.shuffle_state = p.shuffle_state;
        }

        Some(playback)
    }

    pub fn currently_playing(&self) -> Option<&rspotify::model::PlayableItem> {
        self.playback.as_ref().and_then(|p| p.item.as_ref())
    }

    pub fn playback_progress(&self) -> Option<chrono::Duration> {
        match self.playback {
            None => None,
            Some(ref playback) => {
                let base = playback.progress?;
                if !playback.is_playing {
                    return Some(base);
                }
                let seeking = self
                    .seek_deadline
                    .is_some_and(|d| d > std::time::Instant::now());
                if seeking {
                    return Some(base);
                }
                let elapsed = self.playback_last_updated_time.map(|t| t.elapsed())?;
                let delta = chrono::Duration::from_std(elapsed).ok()?;
                Some(base + delta)
            }
        }
    }

    pub fn playing_context_id(&self) -> Option<ContextId> {
        match self.playback {
            Some(ref playback) => match playback.context {
                Some(ref context) => {
                    let uri = crate::utils::parse_uri(&context.uri);
                    match context._type {
                        rspotify::model::Type::Playlist => Some(ContextId::Playlist(
                            PlaylistId::from_uri(&uri).ok()?.into_static(),
                        )),
                        rspotify::model::Type::Album => Some(ContextId::Album(
                            AlbumId::from_uri(&uri).ok()?.into_static(),
                        )),
                        rspotify::model::Type::Artist => Some(ContextId::Artist(
                            ArtistId::from_uri(&uri).ok()?.into_static(),
                        )),
                        rspotify::model::Type::Show => {
                            Some(ContextId::Show(ShowId::from_uri(&uri).ok()?.into_static()))
                        }
                        _ => None,
                    }
                }
                None => self
                    .custom_queue
                    .as_ref()
                    .and_then(|q| q.source_context().cloned())
                    .or_else(|| {
                        self.currently_playing_tracks_id
                            .clone()
                            .map(ContextId::Tracks)
                    }),
            },
            None => None,
        }
    }
}
