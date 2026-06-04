use crate::client::AppClient;
use crate::config;
use crate::state::SharedState;
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

    let backend = audio_backend::find(None).expect("should be able to find an audio backend");
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

    let player = player::Player::new(
        player_config,
        session.clone(),
        mixer.get_soft_volume(),
        move || -> Box<dyn audio_backend::Sink> { backend(None, AudioFormat::default()) },
    );

    let player_event_task = tokio::task::spawn({
        let mut channel = player.get_player_event_channel();
        let state = state.clone();
        let client = client.clone();
        async move {
            while let Some(event) = channel.recv().await {
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
                            Some(crate::state::AdvanceResult::SameBatch) => {
                                client.update_playback(&state);
                            }
                            Some(crate::state::AdvanceResult::NewBatch(_tracks)) => {
                                client.update_playback(&state);
                            }
                            Some(crate::state::AdvanceResult::NeedsRadioTracks) => {
                                client.update_playback(&state);
                            }
                            Some(crate::state::AdvanceResult::EndOfQueue) => {
                                client.update_playback(&state);
                            }
                            None => {
                                client.update_playback(&state);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    });

    tracing::info!("Starting an integrated Spotify player using librespot's spirc protocol");

    let (spirc, spirc_task) = Spirc::new(connect_config, session, creds, player, mixer)
        .await
        .context("initialize spirc")?;

    tokio::task::spawn(async move {
        tokio::select! {
            () = spirc_task => {},
            _ = player_event_task => {}
        }
    });

    tracing::info!("New streaming connection has been established!");

    Ok(spirc)
}
