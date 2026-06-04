use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig};

use crate::client::{ClientRequest, PlayerRequest};
use crate::state::SharedState;
use crate::utils::map_join;

fn update_control_metadata(
    state: &SharedState,
    controls: &mut MediaControls,
    prev_info: &mut String,
) -> Result<(), souvlaki::Error> {
    let player = state.player.read();

    match player.currently_playing() {
        None => {}
        Some(item) => {
            let progress = player
                .playback_progress()
                .and_then(|p| Some(MediaPosition(p.to_std().ok()?)));

            if player.playback.as_ref().expect("playback").is_playing {
                controls.set_playback(MediaPlayback::Playing { progress })?;
            } else {
                controls.set_playback(MediaPlayback::Paused { progress })?;
            }

            match item {
                rspotify::model::PlayableItem::Unknown(_) => {}
                rspotify::model::PlayableItem::Track(track) => {
                    let track_info = format!("{}/{}", track.name, track.album.name);
                    if track_info != *prev_info {
                        controls.set_metadata(MediaMetadata {
                            title: Some(&track.name),
                            album: Some(&track.album.name),
                            artist: Some(&map_join(&track.artists, |a| &a.name, ", ")),
                            duration: track.duration.to_std().ok(),
                            cover_url: crate::utils::get_track_album_image_url(track),
                        })?;
                        *prev_info = track_info;
                    }
                }
                rspotify::model::PlayableItem::Episode(episode) => {
                    let episode_info = format!("{}/{}", episode.name, episode.show.name);
                    if episode_info != *prev_info {
                        controls.set_metadata(MediaMetadata {
                            title: Some(&episode.name),
                            album: Some(&episode.show.name),
                            artist: Some(&episode.show.publisher),
                            duration: episode.duration.to_std().ok(),
                            cover_url: crate::utils::get_episode_show_image_url(episode),
                        })?;
                        *prev_info = episode_info;
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
                client_pub
                    .send(ClientRequest::Player(PlayerRequest::Resume))
                    .unwrap_or_default();
            }
            MediaControlEvent::Pause => {
                client_pub
                    .send(ClientRequest::Player(PlayerRequest::Pause))
                    .unwrap_or_default();
            }
            MediaControlEvent::Toggle => {
                client_pub
                    .send(ClientRequest::Player(PlayerRequest::ResumePause))
                    .unwrap_or_default();
            }
            MediaControlEvent::SetPosition(MediaPosition(dur)) => {
                if let Ok(dur) = chrono::Duration::from_std(dur) {
                    client_pub
                        .send(ClientRequest::Player(PlayerRequest::SeekTrack(dur)))
                        .unwrap_or_default();
                }
            }
            MediaControlEvent::Next => {
                client_pub
                    .send(ClientRequest::Player(PlayerRequest::NextTrack))
                    .unwrap_or_default();
            }
            MediaControlEvent::Previous => {
                client_pub
                    .send(ClientRequest::Player(PlayerRequest::PreviousTrack))
                    .unwrap_or_default();
            }
            MediaControlEvent::SetVolume(volume) => {
                client_pub
                    .send(ClientRequest::Player(PlayerRequest::Volume(
                        (volume * 100.0) as u8,
                    )))
                    .unwrap_or_default();
            }
            _ => {}
        }
    })?;

    controls.set_playback(MediaPlayback::Playing { progress: None })?;

    let refresh_duration = std::time::Duration::from_secs(1);
    let mut info = String::new();
    loop {
        if let Err(e) = update_control_metadata(state, &mut controls, &mut info) {
            tracing::warn!("Media control update error: {e}");
        }
        std::thread::sleep(refresh_duration);
    }
}
