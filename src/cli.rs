//! CLI module for command-line interface
//!
//! This module provides CLI commands similar to spotify-player terminal client,
//! allowing users to control playback without launching the GUI.
//!
//! # Commands
//!
//! - `play` - start/resume playback
//! - `pause` - pause playback
//! - `next` - skip to next track
//! - `prev` - skip to previous track
//! - `search <query>` - search and play first result
//! - `status` - show current playback status
//! - `volume <level>` - set volume (0-100)
//! - `shuffle` - toggle shuffle
//! - `repeat` - toggle repeat mode
//!
//! # Examples
//!
//! ```bash
//! spotify-player-gui play
//! spotify-player-gui pause
//! spotify-player-gui search "radiohead creep"
//! spotify-player-gui status
//! ```

use crate::client::{AppClient, ClientRequest, PlayerRequest};
use crate::config;
use crate::state::{Playback, SharedState};
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Poll until a condition is met or timeout is reached
async fn poll_until<F, Fut>(
    condition: F,
    timeout_secs: u64,
    interval_ms: u64,
    timeout_msg: &str,
) where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    loop {
        if condition().await { return; }
        if std::time::Instant::now() > deadline {
            println!("{timeout_msg}");
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(interval_ms)).await;
    }
}

#[derive(Parser, Debug)]
#[command(name = "spotify-player-gui")]
#[command(author = "Spotify Player GUI")]
#[command(version = "0.1.0")]
#[command(about = "A native macOS Spotify player with a dark GUI, built in Rust")]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: Option<CliCommand>,
    
    #[arg(long, global = true)]
    pub daemon: bool,
}

#[derive(Subcommand, Debug)]
pub enum CliCommand {
    #[command(about = "Start/resume playback")]
    Play,
    
    #[command(about = "Pause playback")]
    Pause,
    
    #[command(about = "Skip to next track")]
    Next,
    
    #[command(about = "Skip to previous track")]
    Prev,
    
    #[command(about = "Search for tracks and play the first result")]
    Search {
        #[arg(required = true)]
        query: String,
    },
    
    #[command(about = "Show current playback status")]
    Status,
    
    #[command(about = "Set volume (0-100)")]
    Volume {
        #[arg(required = true)]
        level: u8,
    },
    
    #[command(about = "Toggle shuffle mode")]
    Shuffle,
    
    #[command(about = "Toggle repeat mode")]
    Repeat,
}

pub async fn run_cli_command(
    command: CliCommand,
    _client: &AppClient,
    client_pub: &flume::Sender<ClientRequest>,
    state: &SharedState,
) -> Result<()> {
    match command {
        CliCommand::Play => {
            client_pub.send(ClientRequest::Player(PlayerRequest::Resume))?;
            println!("Resume command sent to Spotify");
        }
        
        CliCommand::Pause => {
            client_pub.send(ClientRequest::Player(PlayerRequest::Pause))?;
            println!("Pause command sent to Spotify");
        }
        
        CliCommand::Next => {
            client_pub.send(ClientRequest::Player(PlayerRequest::NextTrack))?;
            println!("Next track command sent to Spotify");
        }
        
        CliCommand::Prev => {
            client_pub.send(ClientRequest::Player(PlayerRequest::PreviousTrack))?;
            println!("Previous track command sent to Spotify");
        }
        
        CliCommand::Search { query } => {
            println!("Searching for: {}", query);
            let q = query.clone();
            state.data.write().caches.search.remove(&q);
            client_pub.send(ClientRequest::Search(query.clone()))?;
            
            poll_until(
                || async { state.data.read().caches.search.contains_key(&q) },
                5, 200, "Search timed out after 5 seconds",
            ).await;
            
            let search_results = state.data.read().caches.search.get(&query).cloned();
            
            if let Some(results) = search_results {
                if let Some(track) = results.tracks.first() {
                    println!("Playing: {} - {}", track.name, 
                        track.artists.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", "));
                    
                    let playback = Playback::URIs(
                        vec![rspotify::prelude::PlayableId::Track(track.id.clone())],
                        None,
                    );
                    client_pub.send(ClientRequest::Player(PlayerRequest::StartPlayback(playback, None)))?;
                } else {
                    println!("No tracks found for query: {}", query);
                }
            } else {
                println!("Search still in progress or no results found");
            }
        }
        
        CliCommand::Status => {
            client_pub.send(ClientRequest::GetCurrentPlayback)?;
            
            poll_until(
                || async { state.player.read().playback.is_some() },
                5, 200, "Status timed out after 5 seconds",
            ).await;
            
            let player = state.player.read();
            let playback = player.playback.clone();
            let progress = player.playback_progress();
            let buffered = player.buffered_playback.clone();
            drop(player);
            
            if let Some(pb) = playback {
                if let Some(item) = pb.item {
                    let (name, artists, duration) = match item {
                        rspotify::model::PlayableItem::Track(track) => (
                            track.name,
                            track.artists.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", "),
                            track.duration,
                        ),
                        rspotify::model::PlayableItem::Episode(episode) => (
                            episode.name,
                            episode.show.name,
                            episode.duration,
                        ),
                        rspotify::model::PlayableItem::Unknown(_) => (
                            "Unknown".to_string(),
                            "".to_string(),
                            chrono::Duration::zero(),
                        ),
                    };
                    
                    let progress_str = progress
                        .map(|p| format!("{:02}:{:02}", p.num_minutes(), p.num_seconds() % 60))
                        .unwrap_or_else(|| "--:--".to_string());
                    
                    let duration_str = format!("{:02}:{:02}", duration.num_minutes(), duration.num_seconds() % 60);
                    
                    let status = if pb.is_playing { "Playing" } else { "Paused" };
                    let device = pb.device.name;
                    let volume = pb.device.volume_percent.unwrap_or(0);
                    
                    println!("Status: {}", status);
                    println!("Track:  {} - {}", name, artists);
                    println!("Progress: {} / {}", progress_str, duration_str);
                    println!("Device: {}", device);
                    println!("Volume: {}%", volume);
                    
                    if let Some(ref buffered) = buffered {
                        let shuffle = if buffered.shuffle_state { "On" } else { "Off" };
                        let repeat = match buffered.repeat_state {
                            rspotify::model::RepeatState::Off => "Off",
                            rspotify::model::RepeatState::Track => "Track",
                            rspotify::model::RepeatState::Context => "Context",
                        };
                        println!("Shuffle: {}", shuffle);
                        println!("Repeat: {}", repeat);
                    }
                } else {
                    println!("No track currently playing");
                }
            } else {
                println!("No active playback session");
            }
        }
        
        CliCommand::Volume { level } => {
            if level > 100 {
                anyhow::bail!("Volume must be between 0 and 100");
            }
            client_pub.send(ClientRequest::Player(PlayerRequest::Volume(level)))?;
            println!("Volume set to {}%", level);
        }
        
        CliCommand::Shuffle => {
            client_pub.send(ClientRequest::Player(PlayerRequest::Shuffle))?;
            
            poll_until(
                || async { state.player.read().buffered_playback.is_some() },
                5, 200, "Shuffle timed out after 5 seconds",
            ).await;
            
            if let Some(ref buffered) = state.player.read().buffered_playback {
                let status = if buffered.shuffle_state { "enabled" } else { "disabled" };
                println!("Shuffle {}", status);
            } else {
                println!("Shuffle toggled (state unknown)");
            }
        }
        
        CliCommand::Repeat => {
            client_pub.send(ClientRequest::Player(PlayerRequest::Repeat))?;
            
            poll_until(
                || async { state.player.read().buffered_playback.is_some() },
                5, 200, "Repeat timed out after 5 seconds",
            ).await;
            
            if let Some(ref buffered) = state.player.read().buffered_playback {
                let status = match buffered.repeat_state {
                    rspotify::model::RepeatState::Off => "Off",
                    rspotify::model::RepeatState::Track => "Track",
                    rspotify::model::RepeatState::Context => "Context",
                };
                println!("Repeat mode: {}", status);
            } else {
                println!("Repeat toggled (state unknown)");
            }
        }
    }
    
    Ok(())
}

pub async fn start_cli_headless(
    command: CliCommand,
    state: SharedState,
    client_pub: flume::Sender<ClientRequest>,
    client_sub: flume::Receiver<ClientRequest>,
) -> Result<()> {
    if config::get_config().app_config.get_user_client_id().ok().flatten().is_some() {
        eprintln!("Opening browser for Spotify login...");
    }
    let client = AppClient::new().await.context("construct app client")?;
    eprintln!("Authenticating with Spotify...");
    
    let auth_result = client.new_session(Some(&state), true).await;
    
    if let Err(ref e) = auth_result {
        let err_msg = format!("{:#}", e);
        if err_msg.contains("400") || err_msg.contains("Bad Request") {
            eprintln!("\nERROR: Authentication failed with HTTP 400 Bad Request");
            eprintln!("This usually means the cached token is invalid or expired.");
            if let Ok(cache_path) = config::get_cache_folder_path() {
                let token_path = cache_path.join("user_client_token.json");
                eprintln!("Try running: rm {}", token_path.display());
            }
            eprintln!("Then run your command again to re-authenticate.\n");
        } else if err_msg.contains("401") || err_msg.contains("Unauthorized") {
            eprintln!("\nERROR: Authentication failed with HTTP 401 Unauthorized");
            eprintln!("The credentials are no longer valid.");
            if let Ok(cache_path) = config::get_cache_folder_path() {
                eprintln!("Try running: rm {} {}", cache_path.join("credentials.json").display(), cache_path.join("user_client_token.json").display());
            }
            eprintln!("Then run your command again to re-authenticate.\n");
        }
    }
    
    auth_result.context("initialize new Spotify session")?;
    
    client_pub.send(ClientRequest::GetCurrentUser)?;
    client_pub.send(ClientRequest::GetCurrentPlayback)?;
    
    let client_handler = tokio::task::spawn({
        let state = state.clone();
        let client = client.clone();
        async move {
            crate::client::start_client_handler(&state, &client, &client_sub).await;
        }
    });
    
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        if state.data.read().user_data.user.is_some() { break; }
        if std::time::Instant::now() > deadline {
            eprintln!("Warning: timed out waiting for user data");
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    
    if let Err(err) = run_cli_command(command, &client, &client_pub, &state).await {
        eprintln!("Command failed: {err:#}");
        client_handler.abort();
        return Err(err);
    }
    
    client_handler.abort();
    
    Ok(())
}

pub async fn start_daemon(
    state: SharedState,
    client_pub: flume::Sender<ClientRequest>,
    client_sub: flume::Receiver<ClientRequest>,
) -> Result<()> {
    eprintln!("Starting Spotify Player daemon...");
    let client = AppClient::new().await.context("construct app client")?;
    eprintln!("Authenticating with Spotify...");
    client
        .new_session(Some(&state), true)
        .await
        .context("initialize new Spotify session")?;
    
    client_pub.send(ClientRequest::GetCurrentUser)?;
    client_pub.send(ClientRequest::GetCurrentPlayback)?;
    client_pub.send(ClientRequest::GetUserPlaylists)?;
    
    let client_handler = tokio::task::spawn({
        let state = state.clone();
        async move {
            crate::client::start_client_handler(&state, &client, &client_sub).await;
        }
    });
    
    let player_watcher_running = Arc::new(AtomicBool::new(true));
    let player_watcher_running_clone = player_watcher_running.clone();
    let player_watcher = std::thread::Builder::new()
        .name("player-event-watcher".to_string())
        .spawn({
            let state = state.clone();
            let client_pub = client_pub.clone();
            move || {
                loop {
                    if !player_watcher_running_clone.load(Ordering::Acquire) {
                        break;
                    }
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        crate::client::start_player_event_watcher(&state, &client_pub);
                    }));
                    if let Err(panic_info) = result {
                        tracing::error!("Player event watcher panicked: {:?}", panic_info);
                        tracing::warn!("Player event watcher crashed, restarting...");
                    } else {
                        // Normal exit — likely shutdown, don't restart
                        break;
                    }
                    if !player_watcher_running_clone.load(Ordering::Acquire) {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            }
        })?;

    #[cfg(feature = "media-control")]
    let (media_control_handle, media_control_running) = {
        let media_control_running = Arc::new(AtomicBool::new(true));
        let media_control_running_clone = media_control_running.clone();
        let configs = config::get_config();
        if configs.app_config.enable_media_control {
            let media_client_pub = client_pub.clone();
            let handle = Some(
                std::thread::Builder::new()
                    .name("media-control".to_string())
                    .spawn({
                        let state = state.clone();
                        move || {
                        loop {
                            if !media_control_running_clone.load(Ordering::Acquire) {
                                break;
                            }
                            if let Err(err) = crate::media_control::start_event_watcher(&state, media_client_pub.clone()) {
                                tracing::error!("Media control event watcher failed: {err:#}");
                                std::thread::sleep(std::time::Duration::from_secs(5));
                            }
                        }
                        }
                    })?,
            );
            (handle, Some(media_control_running))
        } else {
            (None, Some(media_control_running))
        }
    };
    #[cfg(not(feature = "media-control"))]
    let media_control_handle: Option<std::thread::JoinHandle<()>> = None;
    
    let signal_state = state.clone();
    tokio::spawn(async move {
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
                    tracing::info!("Received Ctrl+C (SIGINT), shutting down daemon...");
                }
                _ = sigterm.recv() => {
                    tracing::info!("Received SIGTERM, shutting down daemon...");
                }
            }
        }
        
        #[cfg(not(unix))]
        {
            if let Err(e) = tokio::signal::ctrl_c().await {
                tracing::error!("Failed to listen for Ctrl+C: {e}");
                return;
            }
            tracing::info!("Received Ctrl+C, shutting down daemon...");
        }
        
        signal_state.running.store(false, std::sync::atomic::Ordering::Release);
    });
    
    while state.running.load(std::sync::atomic::Ordering::Acquire) {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    
    tracing::info!("Daemon shutting down...");

    player_watcher_running.store(false, Ordering::Release);
    #[cfg(feature = "media-control")]
    if let Some(ref mc_running) = media_control_running {
        mc_running.store(false, Ordering::Release);
    }

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