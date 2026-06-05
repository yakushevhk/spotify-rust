use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use anyhow::Context;
use tracing::Instrument;

use crate::{
    config,
    state::{PlayableId, SharedState},
};

use super::ClientRequest;

struct PlayerEventHandlerState {
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
    loop {
        match client_sub.recv_async().await {
            Ok(request) => {
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
            Err(err) => {
                tracing::error!("Client request channel closed: {err}");
                break;
            }
        }
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

fn handle_player_event(
    state: &SharedState,
    client_pub: &flume::Sender<ClientRequest>,
    handler_state: &mut PlayerEventHandlerState,
) -> anyhow::Result<()> {
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
