use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use anyhow::Context;
use rspotify::model::Id;
use tracing::Instrument;

use crate::{
    config,
    state::{ContextId, ContextPageType, ContextPageUIState, PageState, PlayableId, SharedState},
};

use crate::utils::map_join;

use super::ClientRequest;

struct PlayerEventHandlerState {
    get_context_timer: Instant,
    last_playback_refresh_timer: Instant,
    last_track_end_check: Instant,
    last_queue_check: Instant,
}

/// starts the client's request handler
pub async fn start_client_handler(
    state: &SharedState,
    client: &super::AppClient,
    client_sub: &flume::Receiver<ClientRequest>,
) {
    while let Ok(request) = client_sub.recv_async().await {
        if let Err(err) = client.check_valid_session(state).await {
            tracing::error!("{err:#}");
            state.toast_queue.lock().push_back(format!("Session error: {err:#}"));
            continue;
        }

        let state = state.clone();
        let client = client.clone();
        let span = tracing::info_span!("client_request", request = ?request);

        tokio::task::spawn(
            async move {
                if let Err(err) = client.handle_request(&state, request).await {
                    let msg = format!("Request failed: {err:#}");
                    tracing::error!("{msg}");
                    state.toast_queue.lock().push_back(msg);
                    let mut data = state.data.write();
                    data.shows_loading = false;
                    data.browse.categories_loading = false;
                }
            }
            .instrument(span),
        );
    }
}

fn handle_playback_change_event(
    state: &SharedState,
    client_pub: &flume::Sender<ClientRequest>,
    handler_state: &mut PlayerEventHandlerState,
) -> anyhow::Result<()> {
    let player = state.player.read();
    let (playback, id, duration) = match (
        player.buffered_playback.as_ref(),
        player.currently_playing(),
    ) {
        (Some(playback), Some(rspotify::model::PlayableItem::Track(track))) => {
            let id = match track.id.clone() {
                Some(id) => PlayableId::Track(id),
                None => return Ok(()),
            };
            (playback, id, track.duration)
        },
        (Some(playback), Some(rspotify::model::PlayableItem::Episode(episode))) => (
            playback,
            PlayableId::Episode(episode.id.clone()),
            episode.duration,
        ),
        _ => return Ok(()),
    };

    let now = Instant::now();
    if let Some(progress) = player.playback_progress() {
        if progress >= duration && playback.is_playing
            && now.duration_since(handler_state.last_track_end_check) >= Duration::from_secs(3)
        {
            handler_state.last_track_end_check = now;
            client_pub.send(ClientRequest::GetCurrentPlayback)?;
        }
    }

    if let Some(queue) = player.queue.as_ref() {
        if let Some(queue_track) = queue.currently_playing.as_ref() {
            if let Some(qid) = queue_track.id() {
                if qid != id
                    && now.duration_since(handler_state.last_queue_check) >= Duration::from_secs(5)
                {
                    handler_state.last_queue_check = now;
                    client_pub.send(ClientRequest::GetCurrentUserQueue)?;
                }
            } else if now.duration_since(handler_state.last_queue_check) >= Duration::from_secs(5) {
                handler_state.last_queue_check = now;
                client_pub.send(ClientRequest::GetCurrentUserQueue)?;
            }
        }
    }

    Ok(())
}

fn handle_page_change_event(
    state: &SharedState,
    client_pub: &flume::Sender<ClientRequest>,
    handler_state: &mut PlayerEventHandlerState,
) -> anyhow::Result<()> {
    match state.ui.lock().current_page_mut() {
        PageState::Context {
            id,
            context_page_type,
            state: page_state,
        } => {
            let expected_id = match context_page_type {
                ContextPageType::Browsing(context_id) => Some(context_id.clone()),
                ContextPageType::CurrentPlaying => state.player.read().playing_context_id(),
            };

            let new_id = if *id == expected_id {
                false
            } else {
                // update the context state and request new data when moving to a new context page
                tracing::info!("Current context ID ({:?}) is different from the expected ID ({:?}), update the context state", id, expected_id);

                *id = expected_id;

                // update the UI page state based on the context's type
                match id {
                    Some(id) => {
                        *page_state = Some(match id {
                            ContextId::Album(_) => ContextPageUIState::new_album(),
                            ContextId::Artist(_) => ContextPageUIState::new_artist(),
                            ContextId::Playlist(_) => ContextPageUIState::new_playlist(),
                            ContextId::Tracks(_) => ContextPageUIState::new_tracks(),
                            ContextId::Show(_) => ContextPageUIState::new_show(),
                        });
                    }
                    None => {
                        *page_state = None;
                    }
                }
                true
            };

            // request new context's data if not found in memory
            // To avoid making too many requests, only request if context id is changed
            // or it's been a while since the last request.
            if let Some(id) = id {
                if !matches!(id, ContextId::Tracks(_))
                    && !state.data.read().caches.context.contains_key(&id.uri())
                    && (new_id
                        || handler_state.get_context_timer.elapsed() > Duration::from_secs(5))
                {
                    client_pub.send(ClientRequest::GetContext(id.clone()))?;
                    handler_state.get_context_timer = Instant::now();
                }
            }
        }

        PageState::Lyrics {
            track_uri,
            track,
            artists,
        } => {
            if let Some(rspotify::model::PlayableItem::Track(current_track)) =
                state.player.read().currently_playing()
            {
                if current_track.name != *track {
                    if let Some(id) = &current_track.id {
                        tracing::info!("Currently playing track \"{}\" is different from the track \"{track}\" shown up in the lyrics page. Fetching new track's lyrics...", current_track.name);
                        track.clone_from(&current_track.name);
                        *artists = map_join(&current_track.artists, |a| &a.name, ", ");
                        *track_uri = id.uri();
                        client_pub.send(ClientRequest::GetLyrics {
                            track_id: id.clone_static(),
                        })?;
                    }
                }
            }
        }
        _ => {}
    }

    Ok(())
}

fn handle_player_event(
    state: &SharedState,
    client_pub: &flume::Sender<ClientRequest>,
    handler_state: &mut PlayerEventHandlerState,
) -> anyhow::Result<()> {
    handle_page_change_event(state, client_pub, handler_state)
        .context("handle page change event")?;
    handle_playback_change_event(state, client_pub, handler_state)
        .context("handle playback change event")?;

    Ok(())
}

/// Starts event watcher listening to events and making update requests to the client if needed
pub fn start_player_event_watcher(state: &SharedState, client_pub: &flume::Sender<ClientRequest>) {
    let configs = config::get_config();
    let running = state.running.clone();

    let refresh_duration = Duration::from_millis(100);
    let playback_refresh_duration =
        Duration::from_millis(configs.app_config.playback_refresh_duration_in_ms);
    let mut handler_state = PlayerEventHandlerState {
        get_context_timer: Instant::now(),
        last_playback_refresh_timer: Instant::now(),
        last_track_end_check: Instant::now(),
        last_queue_check: Instant::now(),
    };

    while running.load(Ordering::Relaxed) {
        // periodically refresh the playback state (if enabled in config)
        if configs.app_config.playback_refresh_duration_in_ms > 0
            && handler_state.last_playback_refresh_timer.elapsed() >= playback_refresh_duration
        {
            if let Err(e) = client_pub.send(ClientRequest::GetCurrentPlayback) {
                tracing::warn!("Failed to send GetCurrentPlayback request: {e:#}");
            }
            handler_state.last_playback_refresh_timer = Instant::now();
        }

        if let Err(err) = handle_player_event(state, client_pub, &mut handler_state) {
            tracing::error!("Encounter error when handling player event: {err:#}");
        }

        std::thread::sleep(refresh_duration);
    }
}
