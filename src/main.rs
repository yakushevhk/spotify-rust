mod auth;
mod client;
mod cli;
mod command;
mod config;
mod key;
mod log_layer;
mod playlist_folders;
mod state;
mod token;
mod ui;
mod utils;

#[cfg(feature = "streaming")]
mod streaming;

mod gui;
#[cfg(feature = "media-control")]
mod media_control;

use anyhow::{Context, Result};
use clap::Parser;
use parking_lot::Mutex;
use std::{
    collections::VecDeque,
    sync::atomic::{AtomicBool, Ordering},
    sync::Arc,
};

fn init_spotify(
    client_pub: &flume::Sender<client::ClientRequest>,
) -> Result<()> {
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

/// Maximum size for backtrace files (10 MB)
const MAX_BACKTRACE_SIZE: u64 = 10 * 1024 * 1024;
/// Maximum number of backtrace files to keep
const MAX_BACKTRACE_FILES: usize = 5;

/// Rotate backtrace files: remove oldest if we have too many, rename existing
fn rotate_backtrace_files(log_folder: &std::path::Path, log_prefix: &str) -> Result<()> {
    let backtrace_pattern = format!("{}.backtrace", log_prefix);
    let mut backtrace_files: Vec<_> = std::fs::read_dir(log_folder)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .map(|n| n.starts_with(&backtrace_pattern))
                .unwrap_or(false)
        })
        .collect();

    // Sort by modification time (oldest first)
    backtrace_files.sort_by(|a, b| {
        a.metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            .cmp(&b.metadata().and_then(|m| m.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH))
    });

    // Remove oldest files if we exceed the limit
    while backtrace_files.len() >= MAX_BACKTRACE_FILES {
        if let Some(oldest) = backtrace_files.first() {
            let _ = std::fs::remove_file(oldest.path());
            backtrace_files.remove(0);
        }
    }

    Ok(())
}

fn init_logging(
    log_folder: &std::path::Path,
    log_buffer: Arc<Mutex<VecDeque<String>>>,
) -> Result<()> {
    use std::io::Write;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    let log_prefix = format!(
        "spotify-player-gui-{}",
        chrono::Local::now().format("%Y-%m-%d-%H-%M")
    );

    if !log_folder.exists() {
        std::fs::create_dir_all(log_folder)?;
    }

    // Rotate old backtrace files before creating new one
    rotate_backtrace_files(log_folder, &log_prefix)?;

    // Install panic hook BEFORE tracing subscriber init (H2)
    let backtrace_path = log_folder.join(format!("{log_prefix}.backtrace"));
    let backtrace_file = std::fs::File::create(&backtrace_path)
        .context("failed to create backtrace file")?;
    let backtrace_file = std::sync::Mutex::new(backtrace_file);
    std::panic::set_hook(Box::new(move |info| {
        if let Ok(mut file) = backtrace_file.lock() {
            // Check file size before writing
            if let Ok(metadata) = file.metadata() {
                if metadata.len() >= MAX_BACKTRACE_SIZE {
                    // File is too large, skip writing
                    return;
                }
            }
            let backtrace = backtrace::Backtrace::new();
            let _ = writeln!(&mut file, "Got a panic: {info:#?}\n");
            let _ = writeln!(&mut file, "Stack backtrace:\n{backtrace:?}");
        }
    }));

    // Always install the buffer layer so the in-app log viewer works
    // even when RUST_LOG=off (H3)
    let buffer_layer = log_layer::BufferLayer::new(log_buffer, 1000);

    if std::env::var_os("RUST_LOG").is_some_and(|x| x == "off") {
        tracing_subscriber::registry()
            .with(buffer_layer)
            .init();
        return Ok(());
    }

    let env_filter = tracing_subscriber::EnvFilter::try_from_env("RUST_LOG")
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("spotify_player_gui=info,librespot=info"));

    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_folder.join(format!("{log_prefix}.log")))
        .context("failed to create/open log file")?;

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(std::sync::Mutex::new(log_file));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .with(buffer_layer)
        .init();

    Ok(())
}

async fn start_app(state: &state::SharedState) -> Result<()> {
    let (client_pub, client_sub) = flume::bounded::<client::ClientRequest>(1024);

    let configs = config::get_config();
    // Only show "step 1" when a user-provided client_id is configured (ncspot
    // default does not open a separate browser for PKCE auth).
    let has_user_client = configs.app_config.get_user_client_id()?.is_some();
    if has_user_client {
        eprintln!("Opening browser for Spotify login...");
    }
    let client = client::AppClient::new()
        .await
        .context("construct app client")?;
    eprintln!("Authenticating with Spotify...");
    client
        .new_session(Some(state), true)
        .await
        .context("initialize new Spotify session")?;

    // Note (M14): eprintln! is used above because tracing is not yet
    // initialized.  After init_logging() runs, all output should use
    // tracing macros instead.
    if let Err(err) = init_spotify(&client_pub) {
        tracing::error!("{:#}", err);
        return Err(err).context("Failed to initialize the Spotify data");
    }

    // client event handler task
    let client_handler = tokio::task::spawn({
        let state = state.clone();
        async move {
            client::start_client_handler(&state, &client, &client_sub).await;
        }
    });

    // player event watcher task
    let player_watcher_running = Arc::new(AtomicBool::new(true));
    let player_watcher_running_clone = player_watcher_running.clone();
    let player_watcher = std::thread::Builder::new()
        .name("player-event-watcher".to_string())
        .spawn({
            let state = state.clone();
            let client_pub = client_pub.clone();
            move || {
                while player_watcher_running_clone.load(Ordering::Acquire) {
                    client::start_player_event_watcher(&state, &client_pub);
                }
            }
        })?;

    // media control task (MPRIS on Linux, native on macOS/Windows)
    let media_control_running = Arc::new(AtomicBool::new(true));
    let media_control_running_clone = media_control_running.clone();
    #[cfg(feature = "media-control")]
    let media_control_handle: Option<std::thread::JoinHandle<()>> = {
        let configs = config::get_config();
        if configs.app_config.enable_media_control {
            let media_client_pub = client_pub.clone();
            Some(
                std::thread::Builder::new()
                    .name("media-control".to_string())
                    .spawn({
                        let state = state.clone();
                        move || {
                            while media_control_running_clone.load(Ordering::Acquire) {
                                if let Err(err) = media_control::start_event_watcher(&state, media_client_pub.clone()) {
                                    tracing::error!("Media control event watcher failed: {err:#}");
                                }
                            }
                        }
                    })?,
            )
        } else {
            None
        }
    };

    // Issue #7: Signal handler for clean shutdown on Ctrl+C (SIGINT) and SIGTERM
    let signal_state = state.clone();
    let signal_handler = tokio::spawn(async move {
        // Create signal handlers for both SIGINT and SIGTERM (Unix only)
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};
            let mut sigterm = match signal(SignalKind::terminate()) {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("Failed to create SIGTERM handler: {e}");
                    return;
                }
            };
            
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    tracing::info!("Received Ctrl+C (SIGINT), cleaning up...");
                }
                _ = sigterm.recv() => {
                    tracing::info!("Received SIGTERM, cleaning up...");
                }
            }
        }
        
        #[cfg(not(unix))]
        {
            if let Err(e) = tokio::signal::ctrl_c().await {
                tracing::error!("Failed to listen for Ctrl+C: {e}");
                return;
            }
            tracing::info!("Received Ctrl+C, cleaning up...");
        }
        
        signal_state.running.store(false, std::sync::atomic::Ordering::Release);
    });

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

    // Signal background threads to shut down cleanly
    state.running.store(false, std::sync::atomic::Ordering::Release);
    player_watcher_running.store(false, Ordering::Release);
    media_control_running.store(false, Ordering::Release);

    // Shutdown spawned tasks
    signal_handler.abort();
    client_handler.abort();
    if let Err(_panic) = player_watcher.join() {
        tracing::error!("Player event watcher thread panicked");
    }
    #[cfg(feature = "media-control")]
    if let Some(handle) = media_control_handle {
        if let Err(_panic) = handle.join() {
            tracing::error!("Media control thread panicked");
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(e) = rustls::crypto::ring::default_provider().install_default() {
        eprintln!("Warning: failed to install rustls default crypto provider: {e:?}");
    }
    
    let cli_args = cli::CliArgs::parse();
    
    if let Some(command) = cli_args.command {
        run_cli(command).await?;
    } else if cli_args.daemon {
        run_daemon().await?;
    } else {
        run_gui()?;
    }
    
    Ok(())
}

async fn run_cli(command: cli::CliCommand) -> Result<()> {
    let config_folder = config::get_config_folder_path()?;
    if !config_folder.exists() {
        std::fs::create_dir_all(&config_folder)?;
    }
    
    let lock_path = config_folder.join(".lock");
    fs2::FileExt::try_lock_exclusive(
        &std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .read(true)
            .write(true)
            .open(&lock_path)
            .context("failed to open lock file")?,
    )
    .map_err(|_| anyhow::anyhow!("Another instance is already running."))?;
    
    let cache_folder = config::get_cache_folder_path()?;
    let cache_audio_folder = cache_folder.join("audio");
    if !cache_audio_folder.exists() {
        std::fs::create_dir_all(&cache_audio_folder)
            .with_context(|| format!("failed to create {}", cache_audio_folder.display()))?;
    }
    let cache_image_folder = cache_folder.join("image");
    if !cache_image_folder.exists() {
        std::fs::create_dir_all(&cache_image_folder)
            .with_context(|| format!("failed to create {}", cache_image_folder.display()))?;
    }
    
    {
        let mut configs = config::Configs::new(&config_folder, &cache_folder)?;
        if configs.app_config.log_folder.is_none() {
            configs.app_config.log_folder = Some(cache_folder.clone());
        }
        config::set_config(configs);
    }
    
    let configs = config::get_config();
    let log_folder = configs
        .app_config
        .log_folder
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("log_folder is not set in config"))?;
    
    let log_buffer: Arc<Mutex<VecDeque<String>>> =
        Arc::new(Mutex::new(VecDeque::with_capacity(1000)));
    
    init_logging(log_folder, log_buffer.clone())
        .context("failed to initialize application's logging")?;
    
    tracing::info!("Starting Spotify Player CLI");
    
    let state = Arc::new(state::State::new(true, log_buffer));
    let (client_pub, client_sub) = flume::bounded::<client::ClientRequest>(1024);
    
    cli::start_cli_headless(command, state, client_pub, client_sub).await?;
    
    Ok(())
}

async fn run_daemon() -> Result<()> {
    let config_folder = config::get_config_folder_path()?;
    if !config_folder.exists() {
        std::fs::create_dir_all(&config_folder)?;
    }
    
    let lock_path = config_folder.join(".lock");
    fs2::FileExt::try_lock_exclusive(
        &std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .read(true)
            .write(true)
            .open(&lock_path)
            .context("failed to open lock file")?,
    )
    .map_err(|_| anyhow::anyhow!("Another instance is already running."))?;
    
    let cache_folder = config::get_cache_folder_path()?;
    let cache_audio_folder = cache_folder.join("audio");
    if !cache_audio_folder.exists() {
        std::fs::create_dir_all(&cache_audio_folder)
            .with_context(|| format!("failed to create {}", cache_audio_folder.display()))?;
    }
    let cache_image_folder = cache_folder.join("image");
    if !cache_image_folder.exists() {
        std::fs::create_dir_all(&cache_image_folder)
            .with_context(|| format!("failed to create {}", cache_image_folder.display()))?;
    }
    
    {
        let mut configs = config::Configs::new(&config_folder, &cache_folder)?;
        if configs.app_config.log_folder.is_none() {
            configs.app_config.log_folder = Some(cache_folder.clone());
        }
        config::set_config(configs);
    }
    
    let configs = config::get_config();
    let log_folder = configs
        .app_config
        .log_folder
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("log_folder is not set in config"))?;
    
    let log_buffer: Arc<Mutex<VecDeque<String>>> =
        Arc::new(Mutex::new(VecDeque::with_capacity(1000)));
    
    init_logging(log_folder, log_buffer.clone())
        .context("failed to initialize application's logging")?;
    
    tracing::info!("Starting Spotify Player Daemon");
    
    let state = Arc::new(state::State::new(true, log_buffer));
    let (client_pub, client_sub) = flume::bounded::<client::ClientRequest>(1024);
    
    cli::start_daemon(state, client_pub, client_sub).await?;
    
    Ok(())
}

fn run_gui() -> Result<()> {
    if let Err(e) = rustls::crypto::ring::default_provider().install_default() {
        eprintln!("Warning: failed to install rustls default crypto provider: {e:?}");
    }

    let config_folder = config::get_config_folder_path()?;
    if !config_folder.exists() {
        std::fs::create_dir_all(&config_folder)?;
    }

    // Multi-instance guard (C3)
    let lock_path = config_folder.join(".lock");
    fs2::FileExt::try_lock_exclusive(
        &std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .read(true)
            .write(true)
            .open(&lock_path)
            .context("failed to open lock file")?,
    )
    .map_err(|_| anyhow::anyhow!("Another instance is already running."))?;
    
    let cache_folder = config::get_cache_folder_path()?;
    let cache_audio_folder = cache_folder.join("audio");
    if !cache_audio_folder.exists() {
        std::fs::create_dir_all(&cache_audio_folder)
            .with_context(|| format!("failed to create {}", cache_audio_folder.display()))?;
    }
    let cache_image_folder = cache_folder.join("image");
    if !cache_image_folder.exists() {
        std::fs::create_dir_all(&cache_image_folder)
            .with_context(|| format!("failed to create {}", cache_image_folder.display()))?;
    }

    {
        let mut configs = config::Configs::new(&config_folder, &cache_folder)?;
        if configs.app_config.log_folder.is_none() {
            configs.app_config.log_folder = Some(cache_folder.clone());
        }
        config::set_config(configs);
    }

    let configs = config::get_config();
    let log_folder = configs
        .app_config
        .log_folder
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("log_folder is not set in config"))?;

    let log_buffer: Arc<Mutex<VecDeque<String>>> =
        Arc::new(Mutex::new(VecDeque::with_capacity(1000)));

    init_logging(log_folder, log_buffer.clone())
        .context("failed to initialize application's logging")?;

    tracing::info!("Starting Spotify Player GUI");

    let state = Arc::new(state::State::new(false, log_buffer));

    // Always create a new dedicated runtime for GUI to avoid conflicts.
    // The try_current approach can cause panics when already on a runtime.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to create tokio runtime")?;
    rt.block_on(start_app(&state))
}
