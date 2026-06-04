mod auth;
mod cli;
mod client;
mod command;
mod config;
mod key;
mod log_layer;
mod playlist_folders;
mod state;
mod token;
mod ui;
mod utils;

mod gui;
#[cfg(feature = "media-control")]
mod media_control;

use anyhow::{Context, Result};
use parking_lot::Mutex;
use std::{collections::VecDeque, sync::Arc};

fn init_spotify(
    client_pub: &flume::Sender<client::ClientRequest>,
    client: &client::AppClient,
    state: &state::SharedState,
) -> Result<()> {
    client.initialize_playback(state);
    client_pub.send(client::ClientRequest::GetCurrentUser)?;
    client_pub.send(client::ClientRequest::GetUserPlaylists)?;
    client_pub.send(client::ClientRequest::GetUserFollowedArtists)?;
    client_pub.send(client::ClientRequest::GetUserSavedAlbums)?;
    client_pub.send(client::ClientRequest::GetContext(state::ContextId::Tracks(
        state::USER_LIKED_TRACKS_ID.to_owned(),
    )))?;
    client_pub.send(client::ClientRequest::GetUserSavedShows)?;
    Ok(())
}

fn init_logging(
    log_folder: &std::path::Path,
    log_buffer: Arc<Mutex<VecDeque<String>>>,
) -> Result<()> {
    use std::io::Write;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    if std::env::var_os("RUST_LOG").is_some_and(|x| x == "off") {
        return Ok(());
    }

    let log_prefix = format!(
        "spotify-player-gui-{}",
        chrono::Local::now().format("%y-%m-%d-%H-%M")
    );

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "spotify_player=info,librespot=info");
    }
    if !log_folder.exists() {
        std::fs::create_dir_all(log_folder)?;
    }
    let log_file = std::fs::File::create(log_folder.join(format!("{log_prefix}.log")))
        .context("failed to create log file")?;

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(std::sync::Mutex::new(log_file));

    let buffer_layer = log_layer::BufferLayer::new(log_buffer, 1000);

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(fmt_layer)
        .with(buffer_layer)
        .init();

    let backtrace_file = std::fs::File::create(log_folder.join(format!("{log_prefix}.backtrace")))
        .context("failed to create backtrace file")?;
    let backtrace_file = std::sync::Mutex::new(backtrace_file);
    std::panic::set_hook(Box::new(move |info| {
        let mut file = backtrace_file.lock().unwrap();
        let backtrace = backtrace::Backtrace::new();
        writeln!(&mut file, "Got a panic: {info:#?}\n").unwrap();
        writeln!(&mut file, "Stack backtrace:\n{backtrace:?}").unwrap();
    }));

    Ok(())
}

#[tokio::main]
async fn start_app(state: &state::SharedState) -> Result<()> {
    let (client_pub, client_sub) = flume::unbounded::<client::ClientRequest>();

    let client = client::AppClient::new()
        .await
        .context("construct app client")?;
    client
        .new_session(Some(state), true)
        .await
        .context("initialize new Spotify session")?;

    init_spotify(&client_pub, &client, state).context("Failed to initialize the Spotify data")?;

    // client event handler task
    tokio::task::spawn({
        let state = state.clone();
        async move {
            client::start_client_handler(&state, &client, &client_sub).await;
        }
    });

    // player event watcher task
    std::thread::Builder::new()
        .name("player-event-watcher".to_string())
        .spawn({
            let state = state.clone();
            let client_pub = client_pub.clone();
            move || {
                client::start_player_event_watcher(&state, &client_pub);
            }
        })?;

    // media control task (MPRIS on Linux, native on macOS/Windows)
    #[cfg(feature = "media-control")]
    {
        let configs = config::get_config();
        if configs.app_config.enable_media_control {
            let media_client_pub = client_pub.clone();
            std::thread::Builder::new()
                .name("media-control".to_string())
                .spawn({
                    let state = state.clone();
                    move || {
                        if let Err(err) = media_control::start_event_watcher(&state, media_client_pub) {
                            tracing::error!("Media control event watcher failed: {err:#}");
                        }
                    }
                })?;
        }
    }

    // Launch the GUI
    let gui_state = state.clone();
    let gui_client_pub = client_pub.clone();

    eframe::run_native(
        "Spotify Player",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1200.0, 800.0])
                .with_min_inner_size([800.0, 600.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            let app = gui::SpotifyApp::new(cc, gui_state, gui_client_pub);
            Ok(Box::new(app))
        }),
    )
    .map_err(|e| anyhow::anyhow!("GUI error: {e}"))?;

    Ok(())
}

fn main() -> Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .unwrap();

    let config_folder = config::get_config_folder_path()?;
    if !config_folder.exists() {
        std::fs::create_dir_all(&config_folder)?;
    }

    let cache_folder = config::get_cache_folder_path()?;
    let cache_audio_folder = cache_folder.join("audio");
    if !cache_audio_folder.exists() {
        std::fs::create_dir_all(&cache_audio_folder)?;
    }
    let cache_image_folder = cache_folder.join("image");
    if !cache_image_folder.exists() {
        std::fs::create_dir_all(&cache_image_folder)?;
    }

    {
        let mut configs = config::Configs::new(&config_folder, &cache_folder)?;
        if configs.app_config.log_folder.is_none() {
            configs.app_config.log_folder = Some(cache_folder.clone());
        }
        config::set_config(configs);
    }

    let log_folder = config::get_config()
        .app_config
        .log_folder
        .as_deref()
        .expect("log_folder is set");

    let log_buffer: Arc<Mutex<VecDeque<String>>> =
        Arc::new(Mutex::new(VecDeque::with_capacity(1000)));

    init_logging(log_folder, log_buffer.clone())
        .context("failed to initialize application's logging")?;

    tracing::info!("Starting Spotify Player GUI");

    let state = Arc::new(state::State::new(false, log_buffer));
    start_app(&state)
}
