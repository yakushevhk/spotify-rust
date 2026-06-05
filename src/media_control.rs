use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig};
use rspotify::prelude::Id;
use std::sync::atomic::Ordering;

use crate::client::{ClientRequest, PlayerRequest};
use crate::state::SharedState;
use crate::utils::map_join;

fn update_control_metadata(
    state: &SharedState,
    controls: &mut MediaControls,
    prev_info: &mut Option<String>,
) -> Result<(), souvlaki::Error> {
    let player = state.player.read();

    match player.currently_playing() {
        None => {}
        Some(item) => {
            let progress = player
                .playback_progress()
                .and_then(|p| Some(MediaPosition(p.to_std().ok()?)));

            let is_playing = player.playback.as_ref().is_some_and(|p| p.is_playing);
            if is_playing {
                controls.set_playback(MediaPlayback::Playing { progress })?;
            } else {
                controls.set_playback(MediaPlayback::Paused { progress })?;
            }

            match item {
                rspotify::model::PlayableItem::Unknown(_) => {}
                rspotify::model::PlayableItem::Track(track) => {
                    // MC8: use track.id for dedup instead of name
                    let track_id = track.id.as_ref().map(|id| id.uri()).unwrap_or_default();
                    if Some(&track_id) != prev_info.as_ref() {
                        controls.set_metadata(MediaMetadata {
                            title: Some(&track.name),
                            album: Some(&track.album.name),
                            artist: Some(&map_join(&track.artists, |a| &a.name, ", ")),
                            duration: track.duration.to_std().ok(),
                            cover_url: crate::utils::get_track_album_image_url(track),
                        })?;
                        *prev_info = Some(track_id);
                    }
                }
                rspotify::model::PlayableItem::Episode(episode) => {
                    // MC8: use episode.id for dedup instead of name
                    let episode_id = episode.id.uri();
                    if Some(&episode_id) != prev_info.as_ref() {
                        controls.set_metadata(MediaMetadata {
                            title: Some(&episode.name),
                            album: Some(&episode.show.name),
                            artist: Some(&episode.show.publisher),
                            duration: episode.duration.to_std().ok(),
                            cover_url: crate::utils::get_episode_show_image_url(episode),
                        })?;
                        *prev_info = Some(episode_id);
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn start_event_watcher(
    state: &SharedState,
    client_pub: flume::Sender<ClientRequest>,
) -> Result<(), souvlaki::Error> {
    tracing::info!("Initializing media control event watcher...");

    let config = PlatformConfig {
        dbus_name: "spotify_player_gui",
        display_name: "Spotify Player",
        hwnd: None,
    };
    let mut controls = MediaControls::new(config)?;

    controls.attach(move |e| {
        tracing::info!("Media control event: {e:?}");
        match e {
            MediaControlEvent::Play => {
                if let Err(e) = client_pub.send(ClientRequest::Player(PlayerRequest::Resume)) {
                    tracing::warn!("Failed to send media control Play request: {e:#}");
                }
            }
            MediaControlEvent::Pause => {
                if let Err(e) = client_pub.send(ClientRequest::Player(PlayerRequest::Pause)) {
                    tracing::warn!("Failed to send media control Pause request: {e:#}");
                }
            }
            MediaControlEvent::Toggle => {
                if let Err(e) = client_pub.send(ClientRequest::Player(PlayerRequest::ResumePause)) {
                    tracing::warn!("Failed to send media control Toggle request: {e:#}");
                }
            }
            MediaControlEvent::SetPosition(MediaPosition(dur)) => {
                if let Ok(dur) = chrono::Duration::from_std(dur) {
                    if let Err(e) = client_pub.send(ClientRequest::Player(PlayerRequest::SeekTrack(dur))) {
                        tracing::warn!("Failed to send media control SetPosition request: {e:#}");
                    }
                }
            }
            MediaControlEvent::Next => {
                if let Err(e) = client_pub.send(ClientRequest::Player(PlayerRequest::NextTrack)) {
                    tracing::warn!("Failed to send media control Next request: {e:#}");
                }
            }
            MediaControlEvent::Previous => {
                if let Err(e) = client_pub.send(ClientRequest::Player(PlayerRequest::PreviousTrack)) {
                    tracing::warn!("Failed to send media control Previous request: {e:#}");
                }
            }
            MediaControlEvent::SetVolume(volume) => {
                if let Err(e) = client_pub.send(ClientRequest::Player(PlayerRequest::Volume(
                    (volume * 100.0) as u8,
                ))) {
                    tracing::warn!("Failed to send media control SetVolume request: {e:#}");
                }
            }
            _ => {}
        }
    })?;

    // MC11: start with Stopped instead of Playing to avoid incorrect initial state
    controls.set_playback(MediaPlayback::Stopped)?;

    let running = state.running.clone();
    // MC5: reduce polling interval from 1s to 500ms
    let refresh_duration = std::time::Duration::from_millis(500);
    let mut info: Option<String> = None;
    // MC6: use short sleep intervals and check running flag to reduce shutdown latency
    while running.load(Ordering::Acquire) {
        if let Err(e) = update_control_metadata(state, &mut controls, &mut info) {
            tracing::warn!("Media control update error: {e}");
        }
        // Sleep in small increments to check for shutdown more frequently
        let deadline = std::time::Instant::now() + refresh_duration;
        while running.load(Ordering::Acquire) && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }
    Ok(())
}
