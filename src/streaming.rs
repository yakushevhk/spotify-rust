use crate::client::AppClient;
use crate::config;
use crate::state::SharedState;
use crate::ui::streaming::VisualizationSink;
use anyhow::Context;
use librespot_connect::{ConnectConfig, Spirc};
use librespot_core::authentication::Credentials;
use librespot_core::config::DeviceType;
use librespot_core::Session;
use librespot_playback::audio_backend;
use librespot_playback::config::{AudioFormat, Bitrate, PlayerConfig};
use librespot_playback::mixer::{self, Mixer, MixerConfig};
use librespot_playback::player;
use std::sync::Arc;

pub async fn new_connection(
    client: AppClient,
    state: SharedState,
    session: Session,
    creds: Credentials,
) -> anyhow::Result<Spirc> {
    let configs = config::get_config();
    let device = &configs.app_config.device;

    // Volume is mapped linearly here (0-100 → 0-65535) because
    // SoftMixer::open defaults to VolumeCurve::Cubic, which applies
    // the dB-mapped power curve internally.  Passing a pre-curved
    // value would double-apply the curve and distort perceived loudness.
    let volume =
        (f64::from(std::cmp::min(device.volume, 100_u8)) / 100.0 * 65535.0).round() as u16;

    let connect_config = ConnectConfig {
        name: device.name.clone(),
        device_type: device.device_type.parse::<DeviceType>().unwrap_or_default(),
        initial_volume: volume,
        is_group: false,
        disable_volume: false,
        volume_steps: 64,
    };

    tracing::info!("Application's connect configurations: {:?}", connect_config);

    let mixer = Arc::new(
        mixer::softmixer::SoftMixer::open(MixerConfig::default()).context("opening softmixer")?,
    );
    mixer.set_volume(volume);

    // TODO: audio device hotplug is not supported – `audio_backend::find(None)` is
    // called once at startup.  If the default output device changes (e.g. Bluetooth
    // headphones connect/disconnect), the backend/sink keeps writing to the old
    // device.  A full fix requires listening for OS device-change notifications and
    // rebuilding the player with a fresh sink.
    let backend = audio_backend::find(None)
        .ok_or_else(|| anyhow::anyhow!("no audio backend found on this system"))?;
    let player_config = PlayerConfig {
        bitrate: device
            .bitrate
            .to_string()
            .parse::<Bitrate>()
            .unwrap_or_default(),
        normalisation: device.normalization,
        ..Default::default()
    };

    tracing::info!(
        "Initializing a new integrated player with device_id={}",
        session.device_id()
    );

    let vis_bands = state.vis_bands.clone();
    let player = player::Player::new(
        player_config,
        session.clone(),
        mixer.get_soft_volume(),
        move || -> Box<dyn audio_backend::Sink> {
            let sink = backend(None, AudioFormat::S16);
            if let Some(ref bands) = vis_bands {
                Box::new(VisualizationSink::new(sink, bands.clone(), 44100.0))
            } else {
                sink
            }
        },
    );

    let player_event_task = tokio::task::spawn({
        let mut channel = player.get_player_event_channel();
        let state = state.clone();
        let client = client.clone();
        let my_generation = state.player.read().streaming_generation;
        async move {
            while let Some(event) = channel.recv().await {
                if state.player.read().streaming_generation != my_generation {
                    tracing::debug!("player_event_task: generation mismatch, stopping");
                    return;
                }
                match event {
                    player::PlayerEvent::Playing { .. } => {
                        let mut player = state.player.write();
                        if let Some(playback) = player.buffered_playback.as_mut() {
                            playback.is_playing = true;
                        }
                        #[cfg(feature = "streaming")]
                        if let Some(ref bands) = state.vis_bands {
                            bands.lock().is_active = true;
                        }
                    }
                    player::PlayerEvent::Paused { .. } => {
                        let mut player = state.player.write();
                        if let Some(playback) = player.buffered_playback.as_mut() {
                            playback.is_playing = false;
                        }
                        #[cfg(feature = "streaming")]
                        if let Some(ref bands) = state.vis_bands {
                            bands.lock().is_active = false;
                        }
                    }
                    player::PlayerEvent::EndOfTrack { .. } => {
                        let advanced = {
                            let mut player = state.player.write();
                            player.custom_queue.as_mut().map(|q| q.advance())
                        };
                        match advanced {
                            Some(result) => {
                                client.handle_custom_queue_advance(&state, result).await;
                            }
                            None => {
                                client.update_playback(&state);
                            }
                        }
                    }
                    _ => {}
                }
            }
            tracing::error!(
                "player_event_task: event channel closed unexpectedly – the Spirc/player \
                 task may have died (possible audio device failure)"
            );
        }
    });

    tracing::info!("Starting an integrated Spotify player using librespot's spirc protocol");

    let (spirc, spirc_task) = Spirc::new(connect_config, session, creds, player, mixer)
        .await
        .context("initialize spirc")?;

    tokio::task::spawn(spirc_task);
    tokio::task::spawn(player_event_task);

    tracing::info!("New streaming connection has been established!");

    Ok(spirc)
}
