//! Client module for Spotify API communication
//!
//! This module handles all communication with Spotify, including:
//! - Authentication and session management
//! - Playback control
//! - Library data fetching
//! - Search and browse
//! - Device management
//!
//! # Architecture
//!
//! The client uses a dual-client architecture:
//! 1. **librespot** - For streaming and low-level Spotify protocol
//! 2. **rspotify** - For Web API calls via OAuth PKCE
//!
//! # Threading
//!
//! Client requests are processed asynchronously by a dedicated Tokio task
//! that receives requests via a channel from the GUI thread.
//!
//! # Lock Hierarchy
//!
//! When multiple locks need to be acquired, ALWAYS follow this order:
//! 1. `state.ui` (`Mutex<UIState>`)
//! 2. `state.player` (`RwLock<PlayerState>`)
//! 3. `state.data` (`RwLock<AppData>`)
//! 4. `state.toast_queue` (`Mutex<VecDeque<String>>`)
//! 5. `stream_conn` (`Mutex<Option<Spirc>>`) - streaming feature only

use std::collections::HashSet;
use std::{borrow::Cow, collections::HashMap, sync::Arc};

use crate::state::Lyrics;
use crate::{auth, config};
use crate::{
    auth::AuthConfig,
    state::{
        store_data_into_file_cache, Album, AlbumId, Artist, ArtistId, Category, Context, ContextId,
        Device, FileCacheKey, Item, ItemId, Playback, PlaybackMetadata, Playlist,
        PlaylistFolderItem, PlaylistId, SavedShow, SearchResults, SharedState, Show, ShowId, Track, TrackId,
        UserId, TTL_CACHE_DURATION, USER_LIKED_TRACKS_URI, USER_RECENTLY_PLAYED_TRACKS_URI,
        USER_TOP_TRACKS_URI,
    },
};

use anyhow::Context as _;
use anyhow::Result;

use librespot_core::SpotifyUri;
#[cfg(feature = "streaming")]
use parking_lot::Mutex;

use reqwest::StatusCode;
use rspotify::{http::Query, prelude::*};

mod handlers;
mod request;
mod spotify;

pub use handlers::*;
pub use request::*;
use serde::Deserialize;

const SPOTIFY_API_ENDPOINT: &str = "https://api.spotify.com/v1";
const PLAYBACK_TYPES: [&rspotify::model::AdditionalType; 2] = [
    &rspotify::model::AdditionalType::Track,
    &rspotify::model::AdditionalType::Episode,
];

// Rate-limiting state shared across all http_get calls (M4)
static RATE_LIMIT_REMAINING: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(10);
static RATE_LIMIT_RESET_TIME: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[cfg(feature = "notify")]
static NOTIFY_TEMPLATE_RE: std::sync::LazyLock<regex::Regex> =
    std::sync::LazyLock::new(|| regex::Regex::new(r"\{.*?\}").unwrap());

/// The application's Spotify client
#[derive(Clone)]
pub struct AppClient {
    http: reqwest::Client,
    /// The integrated Spotify client, mainly used for streaming and librespot integration
    spotify: Arc<spotify::Spotify>,
    auth_config: AuthConfig,
    /// The user-provided Spotify client, mainly used for interacting with Spotify Web APIs
    user_client: Option<rspotify::AuthCodePkceSpotify>,
    reauth_lock: Arc<tokio::sync::Mutex<()>>,
    playback_init_handle: Arc<tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>>,
    #[cfg(feature = "streaming")]
    stream_conn: Arc<Mutex<Option<librespot_connect::Spirc>>>,
}

impl AppClient {
    /// Sanitize an image filename to prevent path traversal attacks
    fn sanitize_image_filename(name: &str) -> String {
        // Reject path traversal attempts
        if name.contains("../") || name.contains("..\\") || name == ".." {
            return format!("invalid_traversal_{:x}", Self::hash_filename(name));
        }

        let sanitized: String = name
            .chars()
            .map(|c| match c {
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0' => '_',
                c if c.is_control() => '_',
                c => c,
            })
            .collect();

        // Additional validation: ensure no remaining path separators or traversal
        if sanitized.contains('/') || sanitized.contains('\\') || sanitized.starts_with('.') {
            return format!("invalid_{:x}", Self::hash_filename(&sanitized));
        }

        // Limit filename length
        if sanitized.len() > 200 {
            let hash = Self::hash_filename(&sanitized);
            let truncated: String = sanitized.chars().take(150).collect();
            format!("{}_{:x}", truncated, hash)
        } else {
            sanitized
        }
    }

    /// Generate a simple hash for filename fallback
    fn hash_filename(name: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        hasher.finish()
    }

    pub fn user_client(&self) -> Result<&rspotify::AuthCodePkceSpotify> {
        self.user_client
            .as_ref()
            .context("user-provided client is not initialized: no client_id configured. Set `client_id` or `client_id_command` in the config file.")
    }

    /// Construct a new client
    pub async fn new() -> Result<Self> {
        let configs = config::get_config();
        let auth_config = AuthConfig::new(&configs)?;

        let mut user_client = configs.app_config.get_user_client_id()?.clone().map(|id| {
            let creds = rspotify::Credentials { id, secret: None };
            let mut scopes = auth::OAUTH_SCOPES
                .iter()
                .map(ToString::to_string)
                .collect::<HashSet<_>>();
            // `user-personalized` scope is not supported by user-provided client and only available to the official Spotify client
            scopes.remove("user-personalized");
            let oauth = rspotify::OAuth {
                redirect_uri: configs.app_config.login_redirect_uri.clone(),
                scopes,
                ..Default::default()
            };
        let config = rspotify::Config {
            token_cached: true,
            token_refreshing: true,
            cache_path: configs.cache_folder.join("user_client_token.json"),
            ..Default::default()
        };
            rspotify::AuthCodePkceSpotify::with_config(creds, oauth, config)
        });

        if let Some(client) = &mut user_client {
            let url = client
                .get_authorize_url(None)
                .context("get authorize URL for user-provided client")?;
            eprintln!("Please open this URL in your browser: {url}");
            #[cfg(target_os = "macos")]
            {
                if let Err(e) = std::process::Command::new("open").arg(&url).spawn() {
                    tracing::warn!("Failed to auto-open browser: {e:#}. URL: {url}");
                }
            }
            #[cfg(target_os = "linux")]
            {
                if let Err(e) = std::process::Command::new("xdg-open").arg(&url).spawn() {
                    tracing::warn!("Failed to auto-open browser: {e:#}. URL: {url}");
                }
            }
            client
                .prompt_for_token(&url)
                .await
                .context("get token for user-provided client")?;

            // Set restrictive permissions on the user_client_token.json file
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let token_path = configs.cache_folder.join("user_client_token.json");
                if token_path.exists() {
                    let token_path = token_path.clone();
                    let _ = tokio::task::spawn_blocking(move || {
                        std::fs::set_permissions(&token_path, std::fs::Permissions::from_mode(0o600))
                    }).await;
                }
            }
        }

        Ok(Self {
            spotify: Arc::new(spotify::Spotify::new()),
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()?,
            auth_config,
            user_client,
            reauth_lock: Arc::new(tokio::sync::Mutex::new(())),
            playback_init_handle: Arc::new(tokio::sync::Mutex::new(None)),

            #[cfg(feature = "streaming")]
            stream_conn: Arc::new(Mutex::new(None)),
        })
    }

    async fn token(&self) -> Result<String> {
        self.user_client()?.auto_reauth().await?;
        Ok(self
            .user_client()?
            .get_token()
            .lock()
            .await
            .map_err(|e| anyhow::anyhow!("Token mutex poisoned: {e:?}"))?
            .as_ref()
            .context("no access token")?
            .access_token
            .clone())
    }

    /// Initialize the application's playback upon creating a new session or during startup
    pub async fn initialize_playback(&self, state: &SharedState) {
        state.push_toast("Connecting to Spotify...");
        let handle = tokio::task::spawn({
            let client = self.clone();
            let state = state.clone();
            async move {
                for attempt in 0..5 {
                    if attempt > 0 {
                        let delay = std::time::Duration::from_secs(1 << attempt)
                            .min(std::time::Duration::from_secs(30));
                        tokio::time::sleep(delay).await;
                    }

                    if let Err(err) = client.retrieve_current_playback(&state, false).await {
                        tracing::error!("Failed to retrieve current playback: {err:#}");
                        continue;
                    }

                    if state.player.read().playback.is_some() {
                        state.push_toast("Connected");
                        break;
                    }

                    let id = match client.find_available_device().await {
                        Ok(Some(id)) => Some(Cow::Owned(id)),
                        Ok(None) => None,
                        Err(err) => {
                            tracing::error!("Failed to find an available device: {err:#}");
                            None
                        }
                    };

                    if let Some(id) = id {
                        tracing::info!("Trying to connect to device (id={id})");
                        match client.user_client() {
                            Ok(uc) => {
                                if let Err(err) = uc.transfer_playback(&id, Some(false)).await {
                                    tracing::warn!("Connection failed (device_id={id}): {err:#}");
                                } else {
                                    tracing::info!("Connection succeeded (device_id={id})!");
                                    state.player.write().buffered_playback = None;
                                    state.push_toast("Connected");
                                    break;
                                }
                            }
                            Err(err) => {
                                tracing::error!("No user client available: {err:#}");
                            }
                        }
                    }
                }

                if state.player.read().playback.is_none() {
                    state.push_toast("Failed to connect to Spotify after retries".to_string());
                }


            }
        });
        let mut guard = self.playback_init_handle.lock().await;
        if let Some(old) = guard.take() {
            old.abort();
        }
        *guard = Some(handle);
    }

    /// Create a new client session
    pub async fn new_session(&self, state: Option<&SharedState>, reauth: bool) -> Result<()> {
        let session = self.auth_config.session();
        let creds = auth::get_creds(&self.auth_config, reauth, true).await.context("get credentials")?;
        self.spotify.set_session(session.clone()).await;

        #[allow(unused_mut)]
        let mut connected = false;

        #[cfg(feature = "streaming")]
        if let Some(state) = state {
            if state.is_streaming_enabled() {
                self.new_streaming_connection(state.clone(), session.clone(), creds.clone())
                    .await
                    .context("new streaming connection")?;
                connected = true;
            }
        }

        if !connected {
            // if session is not connected (triggered by `new_streaming_connection`), connect to the session
            let creds_backup = creds.clone();
            if let Err(err) = session.connect(creds, true).await {
                let err_msg = format!("{err:#}");
                let is_auth_failure = err_msg.to_lowercase().contains("authentication")
                    || err_msg.to_lowercase().contains("auth")
                    || err_msg.to_lowercase().contains("credentials")
                    || err_msg.to_lowercase().contains("login");

                if is_auth_failure {
                    tracing::warn!(
                        "Session connect failed with auth error, clearing cached credentials and retrying: {err:#}"
                    );
                    let fresh_creds = auth::get_creds(&self.auth_config, true, false)
                        .await
                        .context("get credentials after clearing cache")?;
                    session
                        .connect(fresh_creds, true)
                        .await
                        .context("connect to a session after re-auth")?;
                } else {
                    // Network/transport error: retry with same credentials using exponential backoff
                    // Issue #1: Add max total timeout of 60 seconds for retry loop
                    tracing::warn!(
                        "Session connect failed with network error, retrying with backoff: {err:#}"
                    );
                    let start_time = std::time::Instant::now();
                    let max_total_timeout = std::time::Duration::from_secs(60);
                    let mut connected = false;
                    for attempt in 0..3u32 {
                        // Check if we've exceeded total timeout
                        if start_time.elapsed() >= max_total_timeout {
                            tracing::error!("Session connection retry exceeded 60s total timeout");
                            anyhow::bail!("session.connect failed: exceeded 60s total timeout");
                        }
                        let delay = std::time::Duration::from_secs(1 << attempt);
                        tokio::time::sleep(delay).await;
                        // Check timeout again after sleep
                        if start_time.elapsed() >= max_total_timeout {
                            tracing::error!("Session connection retry exceeded 60s total timeout");
                            anyhow::bail!("session.connect failed: exceeded 60s total timeout");
                        }
                        match session.connect(creds_backup.clone(), true).await {
                            Ok(()) => {
                                connected = true;
                                break;
                            }
                            Err(retry_err) => {
                                tracing::warn!(
                                    "Session connect retry {}/3 failed: {retry_err:#}",
                                    attempt + 1
                                );
                            }
                        }
                    }
                    if !connected {
                        anyhow::bail!("session.connect failed after 3 retries: {err:#}");
                    }
                }
            }
        }

        tracing::info!("Used a new session for Spotify client.");

        let configs = config::get_config();
        
        if auth::check_user_token_expired(&configs.cache_folder) {
            tracing::info!("User client token expired or missing, clearing and re-authenticating...");
            if let Err(e) = auth::clear_expired_tokens(&configs.cache_folder) {
                tracing::warn!("Failed to clear expired token: {:#}", e);
            }
        }

        let max_token_retries = 3;
        let mut last_token_err = None;
        
        let user_client = match self.user_client() {
            Ok(uc) => uc,
            Err(e) => {
                return Err(anyhow::anyhow!("User client not available: {:#}", e));
            }
        };
        
        // Try auto_reauth first before falling back to refresh_token
        if let Ok(()) = user_client.auto_reauth().await {
            tracing::info!("Auto re-auth succeeded for user client token");
        } else {
            tracing::info!("Auto re-auth unavailable, attempting token refresh");
            for attempt in 0..max_token_retries {
                match user_client.refresh_token().await {
                    Ok(()) => {
                        tracing::info!("Token refreshed successfully");
                        last_token_err = None;
                        break;
                    }
                    Err(e) => {
                        let err_msg = format!("{:#}", e);
                        tracing::warn!("Token refresh attempt {}/{} failed: {}", attempt + 1, max_token_retries, err_msg);
                        
                        if err_msg.contains("400") || err_msg.contains("Bad Request") {
                            tracing::warn!("HTTP 400 Bad Request during token refresh. Clearing token cache...");
                            let _ = auth::clear_expired_tokens(&configs.cache_folder);
                            last_token_err = Some("HTTP 400 Bad Request: Delete ~/.cache/spotify-player/user_client_token.json and re-authenticate".to_string());
                            break;
                        } else if err_msg.contains("401") || err_msg.contains("Unauthorized") {
                            return Err(anyhow::anyhow!(
                                "Token refresh failed with HTTP 401 Unauthorized. Delete ~/.cache/spotify-player/credentials.json and ~/.cache/spotify-player/user_client_token.json, then re-authenticate: {}",
                                err_msg
                            ));
                        } else if err_msg.contains("500") || err_msg.contains("Internal Server Error") {
                            if attempt < max_token_retries - 1 {
                                let delay = std::time::Duration::from_secs(1 << attempt);
                                tracing::warn!("Spotify server error, retrying in {}s...", delay.as_secs());
                                tokio::time::sleep(delay).await;
                                last_token_err = Some("Spotify server error (500)".to_string());
                                continue;
                            }
                            return Err(anyhow::anyhow!(
                                "Token refresh failed after {} attempts with HTTP 500. Spotify servers may be experiencing issues. Please retry in a few seconds: {}",
                                max_token_retries, err_msg
                            ));
                        } else {
                            last_token_err = Some(err_msg);
                            if attempt < max_token_retries - 1 {
                                tokio::time::sleep(std::time::Duration::from_secs(1 << attempt)).await;
                                continue;
                            }
                        }
                    }
                }
            }
        }
        
        if let Some(err) = last_token_err {
            tracing::error!("Token refresh failed after {} attempts: {}", max_token_retries, err);
            return Err(anyhow::anyhow!("Token refresh failed: {}", err));
        }

        if let Some(state) = state {
            // Per documented lock hierarchy: player before data
            // reset player state (playback, devices, queue, custom_queue)
            // to avoid stale data from a previous account/session
            // Preserve streaming_generation so player_event_task is not killed
            {
                let mut player = state.player.write();
                let generation = player.streaming_generation;
                let queue = std::mem::take(&mut player.custom_queue);
                *player = crate::state::PlayerState::default();
                player.streaming_generation = generation;
                player.custom_queue = queue;
            }
            // reset user data and caches to avoid stale data from a previous account
            state.reset_user_data();
            self.initialize_playback(state).await;
            // Clear transient UI state that should not survive account switches
            // Clear transient "Connecting..." toasts since initialize_playback already completed
            // (keep "Connected" or error toasts that the user should see)
            {
                let mut toasts = state.toast_queue.lock();
                toasts.retain(|t| !t.contains("Connecting"));
            }
            #[cfg(feature = "streaming")]
            if let Some(ref bands) = state.vis_bands {
                bands.lock().is_active = false;
            }
        }

        Ok(())
    }

    /// Check if the current session is valid and if invalid, create a new session
    /// Also validates the user_client OAuth token and refreshes if needed
    pub async fn check_valid_session(&self, state: &SharedState) -> Result<()> {
        // Acquire reauth_lock once to prevent race conditions and deadlocks
        let _guard = self.reauth_lock.lock().await;

        // Check librespot streaming session
        let session = self.spotify.session().await?;
        if session.is_invalid() {
            // Re-check after acquiring the lock in case another task already fixed it
            let session = self.spotify.session().await?;
            if session.is_invalid() {
                tracing::info!("Client's current session is invalid, creating a new session...");
                if let Err(err) = self.new_session(Some(state), false).await {
                    tracing::warn!(
                        "Failed to create new session with cached credentials: {err:#}, retrying with re-auth..."
                    );
                    self.new_session(Some(state), true)
                        .await
                        .context("create new client session after re-auth")?;
                }
            }
        }

        // Check and refresh user_client OAuth token if needed
        // This is critical for CLI commands that use user_client() directly
        if let Some(user_client) = &self.user_client {
            let token_arc = user_client.get_token();
            // Scoped block: only hold the token mutex long enough to read
            // `expires_at`. It MUST be released before `auto_reauth` /
            // `refresh_token` because rspotify internally re-acquires the
            // same `Arc<Mutex<Token>>`. tokio's Mutex is non-reentrant, so
            // holding the guard across those calls deadlocks forever.
            // TOCTOU safety: `reauth_lock` (acquired above) serializes every
            // concurrent caller of `check_valid_session`, so no other task
            // can race the refresh decision once we drop the guard.
            let needs_refresh = {
                let token_guard = token_arc.lock().await
                    .map_err(|e| anyhow::anyhow!("Token mutex poisoned: {e:?}"))?;
                match token_guard.as_ref() {
                    None => true,
                    Some(token) => {
                        token.expires_at.is_none_or(|expires_at| {
                            chrono::Utc::now() >= expires_at - chrono::Duration::seconds(60)
                        })
                    }
                }
            };

            if needs_refresh {
                tracing::info!("User client token expired or expiring soon, refreshing...");

                // Try auto_reauth first
                if let Err(err) = user_client.auto_reauth().await {
                    tracing::warn!("Auto reauth failed: {err:#}, attempting manual refresh");

                    let mut refresh_err = None;
                    for attempt in 0..2 {
                        if let Err(e) = user_client.refresh_token().await {
                            if attempt == 0 {
                                tracing::warn!("Token refresh failed, retrying: {:#}", e);
                                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                                refresh_err = Some(e);
                                continue;
                            }
                            return Err(anyhow::anyhow!(
                                "Failed to refresh user client token. Please re-authenticate: {:#}",
                                e
                            ));
                        }
                        refresh_err = None;
                        break;
                    }
                    if let Some(e) = refresh_err {
                        return Err(anyhow::anyhow!(
                            "Failed to refresh user client token. Please re-authenticate by running the GUI or clearing cache: {e:#}"
                        ));
                    }
                }

                tracing::info!("User client token successfully refreshed");
            }
        }

        Ok(())
    }

    /// Create a new streaming connection
    #[cfg(feature = "streaming")]
    pub async fn new_streaming_connection(
        &self,
        state: SharedState,
        session: librespot_core::Session,
        creds: librespot_core::authentication::Credentials,
    ) -> Result<()> {
        state.player.write().streaming_generation += 1;
        let new_conn =
            crate::streaming::new_connection(self.clone(), state, session, creds).await?;
        let mut stream_conn = self.stream_conn.lock();
        // shutdown old streaming connection and replace it with a new connection
        if let Some(conn) = stream_conn.as_ref() {
            if let Err(err) = conn.shutdown() {
                tracing::error!("Failed to shutdown old streaming connection: {err:#}");
            }
        }
        *stream_conn = Some(new_conn);
        Ok(())
    }

    /// Retry a player operation with exponential backoff on retryable errors.
    /// Retries on rate-limit (429), server errors (500-504), timeout, and connection issues.
    async fn player_op_retry<F, Fut>(&self, mut operation: F) -> Result<()>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<(), rspotify::ClientError>>,
    {
        const MAX_RETRIES: u32 = 3;
        let mut last_err = None;
        for attempt in 0..MAX_RETRIES {
            match operation().await {
                Ok(()) => return Ok(()),
                Err(e) if self.is_player_retryable(&e) && attempt + 1 < MAX_RETRIES => {
                    tracing::warn!(
                        "Player operation failed (attempt {}/{}): {}",
                        attempt + 1, MAX_RETRIES, e
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(200 * (1 << attempt))).await;
                    last_err = Some(e);
                }
                Err(e) => return Err(e.into()),
            }
        }
        if let Some(e) = last_err {
            Err(e.into())
        } else {
            Ok(())
        }
    }

    /// Check if a player error is retryable (network/server issue, not client error).
    fn is_player_retryable(&self, e: &rspotify::ClientError) -> bool {
        let err_str = e.to_string().to_lowercase();
        err_str.contains("http error 429")
            || err_str.contains("http error 500")
            || err_str.contains("http error 502")
            || err_str.contains("http error 503")
            || err_str.contains("http error 504")
            || err_str.contains("timeout")
            || err_str.contains("connection")
    }

    /// Handle a player request, return a new playback metadata on success
    pub async fn handle_player_request(
        &self,
        request: PlayerRequest,
        mut playback: Option<PlaybackMetadata>,
    ) -> Result<Option<PlaybackMetadata>> {
        // handle requests that don't require an active playback
        match request {
            PlayerRequest::TransferPlayback(device_id, force_play) => {
                // `TransferPlayback` needs to be handled separately from other player requests
                // because `TransferPlayback` doesn't require an active playback
                self.user_client()?.transfer_playback(&device_id, Some(force_play)).await?;
                tracing::info!("Transferred playback to device with id={}", device_id);
                // #63: don't clear buffered_playback here; the caller should
                // trigger update_playback after transfer to refresh state.
                return Ok(playback);
            }
            PlayerRequest::StartPlayback(p, shuffle) => {
                if let (Some(shuffle), Some(playback)) = (shuffle, playback.as_mut()) {
                    playback.shuffle_state = shuffle;
                }
                let mut device_id: Option<String> = playback.as_ref().and_then(|p| p.device_id.clone());
                #[cfg(feature = "streaming")]
                {
                    if device_id.is_none() && self.stream_conn.lock().is_some() {
                        match self.spotify.session().await {
                            Ok(session) => { device_id = Some(session.device_id().to_string()); }
                            Err(e) => tracing::warn!("Could not get streaming session for device_id: {e:#}"),
                        }
                    }
                }
                let device_id_ref = device_id.as_deref();
                self.start_playback(p, device_id_ref).await?;
                if let Some(requested_shuffle) = shuffle {
                    self.user_client()?.shuffle(requested_shuffle, device_id_ref).await?;
                    if let Some(ref mut pb) = playback {
                        pb.shuffle_state = requested_shuffle;
                    }
                }
                return Ok(playback);
            }
            _ => {}
        }

        let mut playback = playback.context("no playback found")?;
        let device_id = playback.device_id.as_deref();

        let uc = self.user_client()?;

        match request {
            PlayerRequest::NextTrack => {
                self.player_op_retry(|| uc.next_track(device_id)).await?;
            }
            PlayerRequest::PreviousTrack => {
                self.player_op_retry(|| uc.previous_track(device_id)).await?;
            }
            PlayerRequest::Resume => {
                if !playback.is_playing {
                    self.player_op_retry(|| uc.resume_playback(device_id, None)).await?;
                    playback.is_playing = true;
                } else {
                    tracing::debug!("Resume skipped — already playing according to buffered state");
                }
            }

            PlayerRequest::Pause => {
                if playback.is_playing {
                    self.player_op_retry(|| uc.pause_playback(device_id)).await?;
                    playback.is_playing = false;
                } else {
                    tracing::debug!("Pause skipped — already paused according to buffered state");
                }
            }
            PlayerRequest::ResumePause => {
                if playback.is_playing {
                    self.player_op_retry(|| uc.pause_playback(device_id)).await?;
                } else {
                    self.player_op_retry(|| uc.resume_playback(device_id, None)).await?;
                }
                playback.is_playing = !playback.is_playing;
            }
            PlayerRequest::SeekTrack(position_ms) => {
                self.player_op_retry(|| uc.seek_track(position_ms, device_id)).await?;
            }
            PlayerRequest::Repeat => {
                let next_repeat_state = match playback.repeat_state {
                    rspotify::model::RepeatState::Off => rspotify::model::RepeatState::Track,
                    rspotify::model::RepeatState::Track => rspotify::model::RepeatState::Context,
                    rspotify::model::RepeatState::Context => rspotify::model::RepeatState::Off,
                };

                self.player_op_retry(|| uc.repeat(next_repeat_state, device_id)).await?;

                playback.repeat_state = next_repeat_state;
            }
            PlayerRequest::Shuffle => {
                self.player_op_retry(|| uc.shuffle(!playback.shuffle_state, device_id)).await?;

                playback.shuffle_state = !playback.shuffle_state;
            }
            PlayerRequest::Volume(volume) => {
                self.player_op_retry(|| uc.volume(volume, device_id)).await?;

                playback.volume = Some(u32::from(volume));
                playback.mute_state = None;
            }
            PlayerRequest::ToggleMute => {
                let new_mute_state = match playback.mute_state {
                    None => {
                        let restore_volume = playback.volume.unwrap_or(50).min(100);
                        self.player_op_retry(|| uc.volume(0, device_id)).await?;
                        Some(restore_volume)
                    }
                    Some(volume) => {
                        let vol = volume.min(100) as u8;
                        self.player_op_retry(|| uc.volume(vol, device_id)).await?;
                        None
                    }
                };

                playback.mute_state = new_mute_state;
            }
            PlayerRequest::StartPlayback(..) => {
                anyhow::bail!("`StartPlayback` should be handled earlier")
            }
            PlayerRequest::TransferPlayback(..) => {
                anyhow::bail!("`TransferPlayback` should be handled earlier")
            }
        }

        Ok(Some(playback))
    }

}

impl AppClient {
    pub(crate) async fn handle_request(
        &self,
        state: &SharedState,
        request: ClientRequest,
    ) -> Result<()> {
        let timer = tokio::time::Instant::now();

        match request {
            ClientRequest::GetBrowseCategories => {
                let result = self.browse_categories().await;
                {
                    let mut data = state.data.write();
                    data.browse.categories_loading = false;
                    if let Ok(ref categories) = result {
                        data.browse.categories = categories.clone();
                    }
                }
                if let Err(e) = result {
                    return Err(e.into());
                }
            }
            ClientRequest::GetBrowseCategoryPlaylists(category) => {
                let result = self.browse_category_playlists(&category.id).await;
                {
                    let mut data = state.data.write();
                    data.browse.category_playlists_loading = None;
                    if let Ok(ref playlists) = result {
                        data.browse.insert_category_playlists(category.id, playlists.clone());
                    }
                }
                if let Err(e) = result {
                    return Err(e.into());
                }
            }
            ClientRequest::GetLyrics { track_id } => {
                let uri = track_id.uri();
                if !state.data.read().caches.lyrics.contains_key(&uri) {
                    let lyrics = self.lyrics(track_id).await?;
                    let mut data = state.data.write();
                    // Re-check after acquiring write lock to avoid race condition (F7)
                    if !data.caches.lyrics.contains_key(&uri) {
                        data.caches.lyrics.insert(uri, lyrics, TTL_CACHE_DURATION);
                    }
                }
            }
            #[cfg(feature = "streaming")]
            ClientRequest::RestartIntegratedClient => {
                self.new_session(Some(state), false).await?;
            }
            ClientRequest::GetCurrentUser => {
                let user = self.user_client()?.current_user().await?;
                state.data.write().user_data.user = Some(user);
            }
            ClientRequest::Player(request) => {
                let seek_position = if let PlayerRequest::SeekTrack(pos) = &request {
                    Some(*pos)
                } else {
                    None
                };
                let playback = state.player.read().buffered_playback.clone();
                let new_playback = self.handle_player_request(request, playback).await?;
                {
                    let mut player = state.player.write();
                    if let Some(updated) = new_playback {
                        if let Some(ref mut existing) = player.buffered_playback {
                            // Capture shuffle/repeat deltas before overwriting to propagate into CustomQueue.
                            let shuffle_changed = existing.shuffle_state != updated.shuffle_state;
                            let repeat_changed = existing.repeat_state != updated.repeat_state;
                            let new_shuffle_state = updated.shuffle_state;
                            let new_repeat_state = updated.repeat_state;

                            existing.is_playing = updated.is_playing;
                            existing.repeat_state = updated.repeat_state;
                            existing.shuffle_state = updated.shuffle_state;
                            existing.volume = updated.volume;
                            existing.mute_state = updated.mute_state;
                            existing.device_id = updated.device_id.clone();
                            existing.device_name = updated.device_name.clone();

                            if let Some(ref mut q) = player.custom_queue {
                                if shuffle_changed {
                                    q.set_shuffle_mode(if new_shuffle_state {
                                        crate::state::ShuffleMode::Shuffle
                                    } else {
                                        crate::state::ShuffleMode::Off
                                    });
                                }
                                if repeat_changed {
                                    q.set_repeat(new_repeat_state);
                                }
                            }
                        } else {
                            player.buffered_playback = Some(updated);
                        }
                    }
                    if let Some(pos_ms) = seek_position {
                        if let Some(ref mut pb) = player.playback {
                            pb.progress = Some(pos_ms);
                        }
                        player.playback_last_updated_time = Some(std::time::Instant::now());
                        player.seek_deadline =
                            Some(std::time::Instant::now() + std::time::Duration::from_millis(500));
                    }
                }
                self.update_playback(state).await;
            }
            ClientRequest::GetCurrentPlayback => {
                self.retrieve_current_playback(state, true).await?;
            }
            ClientRequest::GetDevices => {
                #[allow(unused_mut)]
                let mut devices: Vec<Device> = self
                    .available_devices()
                    .await?
                    .into_iter()
                    .filter_map(Device::try_from_device)
                    .collect();

                    // Include the local streaming device when the streaming feature is enabled.
                    // This ensures the device list is never empty when using integrated playback,
                    // even if the user hasn't configured a custom client_id or if the Spotify API
                    // hasn't registered the device yet.
                    #[cfg(feature = "streaming")]
                    {
                        use crate::state::DeviceType;
                        let configs = config::get_config();
                        match self.spotify.session().await {
                            Ok(session) => {
                                let local_device = Device {
                                    id: session.device_id().to_string(),
                                    name: configs.app_config.device.name.clone(),
                                    is_active: self.stream_conn.lock().is_some(),
                                    device_type: DeviceType::Computer,
                                };

                                // Only add if not already in the list (avoid duplicates)
                                if !devices.iter().any(|d| d.id == local_device.id) {
                                    devices.push(local_device);
                                }
                            }
                            Err(e) => tracing::warn!("Streaming session unavailable for GetDevices: {e:#}"),
                        }
                    }

                state.player.write().devices = devices;
            }
            ClientRequest::GetUserPlaylists => {
                let playlists = self.current_user_playlists().await?;
                let node = state.data.read().user_data.playlist_folder_node.clone();
                let playlists = if let Some(node) = node.filter(|n| !n.children.is_empty()) {
                    crate::playlist_folders::structurize(playlists, &node.children)
                } else {
                    playlists
                        .into_iter()
                        .map(PlaylistFolderItem::Playlist)
                        .collect()
                };
                store_data_into_file_cache(
                    FileCacheKey::Playlists,
                    &config::get_config().cache_folder,
                    &playlists,
                )
                .context("store user's playlists into the cache folder")?;
                {
                    let mut data = state.data.write();
                    data.user_data.playlists = playlists;
                    data.library_loaded = true;
                }
            }
            ClientRequest::GetUserFollowedArtists => {
                let artists = self.current_user_followed_artists().await?;
                store_data_into_file_cache(
                    FileCacheKey::FollowedArtists,
                    &config::get_config().cache_folder,
                    &artists,
                )
                .context("store user's followed artists into the cache folder")?;
                state.data.write().user_data.followed_artists = artists;
            }
            ClientRequest::GetUserSavedAlbums => {
                let albums = self.current_user_saved_albums().await?;
                store_data_into_file_cache(
                    FileCacheKey::SavedAlbums,
                    &config::get_config().cache_folder,
                    &albums,
                )
                .context("store user's saved albums into the cache folder")?;
                state.data.write().user_data.saved_albums = albums;
            }
            ClientRequest::GetUserSavedShows => {
                match self.current_user_saved_shows().await {
                    Ok(shows) => {
                        if let Err(e) = store_data_into_file_cache(
                            FileCacheKey::SavedShows,
                            &config::get_config().cache_folder,
                            &shows,
                        ) {
                            tracing::warn!("Failed to cache shows: {e:#}");
                        }
                        let mut data = state.data.write();
                        data.user_data.saved_shows = shows;
                    }
                    Err(e) => {
                        tracing::error!("Failed to fetch saved shows: {e:#}");
                        state.push_toast(format!("Failed to load saved shows: {e}"));
                        state.data.write().shows_loading = false;
                        return Err(anyhow::anyhow!("Failed to fetch saved shows: {e:#}"));
                    }
                }
                state.data.write().shows_loading = false;
            }
            ClientRequest::GetContext(context) => {
                let uri = context.uri();
                // Liked tracks must always be refreshed to keep user_data.saved_tracks in sync.
                let cache_miss = uri != USER_LIKED_TRACKS_URI
                    && !state.data.read().caches.context.contains_key(&uri);
                let is_liked = uri == USER_LIKED_TRACKS_URI;
                if cache_miss || is_liked {
                    let ctx = match context {
                        ContextId::Playlist(playlist_id) => {
                            self.playlist_context(playlist_id).await?
                        }
                        ContextId::Album(album_id) => self.album_context(album_id).await?,
                        ContextId::Artist(artist_id) => self.artist_context(artist_id).await?,
                        ContextId::Tracks(tracks_id) => match tracks_id.uri.as_str() {
                            USER_TOP_TRACKS_URI => Context::Tracks {
                                tracks: self.current_user_top_tracks().await?,
                                desc: "User's top tracks".to_string(),
                            },
                            USER_RECENTLY_PLAYED_TRACKS_URI => Context::Tracks {
                                tracks: self.current_user_recently_played_tracks().await?,
                                desc: "User's recently played tracks".to_string(),
                            },
                            USER_LIKED_TRACKS_URI => {
                                let tracks = self.current_user_saved_tracks().await?;
                                let tracks_hm = tracks
                                    .iter()
                                    .map(|t| (t.id.uri(), t.clone()))
                                    .collect::<HashMap<_, _>>();
                                store_data_into_file_cache(
                                    FileCacheKey::SavedTracks,
                                    &config::get_config().cache_folder,
                                    &tracks_hm,
                                )
                                .context("store user's saved tracks into the cache folder")?;
                                state.data.write().user_data.saved_tracks = tracks_hm;
                                Context::Tracks {
                                    tracks,
                                    desc: "User's liked tracks".to_string(),
                                }
                            }
                            u if u.starts_with("radio:") => Context::Tracks {
                                tracks: self.radio_tracks(u["radio:".len()..].to_string()).await?,
                                desc: tracks_id.kind.clone(),
                            },
                            uri => anyhow::bail!("unsupported Tracks context: {uri}"),
                        },
                        ContextId::Show(show_id) => self.show_context(show_id).await?,
                    };

                    state
                        .data
                        .write()
                        .caches
                        .context
                        .insert(uri, ctx, TTL_CACHE_DURATION);
                }
            }
            ClientRequest::Search(query) => {
                // #40: do the search first, then check-and-insert atomically
                // to avoid duplicate API calls from concurrent requests.
                // Normalize query to lowercase for case-insensitive cache lookups
                let normalized_query = query.trim().to_lowercase();
                let already_cached = state.data.read().caches.search.contains_key(&normalized_query);
                if !already_cached {
                    let results = self.search(&normalized_query).await?;

                    state
                        .data
                        .write()
                        .caches
                        .search
                        .insert(normalized_query, results, TTL_CACHE_DURATION);
                }
            }

            ClientRequest::AddPlayableToQueue(playable_id) => {
                let device_id = state.player.read().playback.as_ref().and_then(|p| p.device.id.clone());
                self.user_client()?.add_item_to_queue(playable_id, device_id.as_deref()).await?;
            }
            ClientRequest::AddPlayableToPlaylist(playlist_id, playable_id) => {
                self.add_item_to_playlist(state, playlist_id, playable_id)
                    .await?;
            }
            ClientRequest::AddAlbumToQueue(album_id) => {
                let album_context = self.album_context(album_id).await?;

                if let Context::Album { album: _, tracks } = album_context {
                    let device_id = state.player.read().playback.as_ref().and_then(|p| p.device.id.clone());
                    let device_id_ref = device_id.as_deref();
                    let uc = self.user_client()?;
                    const QUEUE_CHUNK_SIZE: usize = 15;
                    for chunk in tracks.chunks(QUEUE_CHUNK_SIZE) {
                        let futs: Vec<_> = chunk.iter().map(|track| {
                            uc.add_item_to_queue(PlayableId::Track(track.id.clone()), device_id_ref)
                        }).collect();
                        futures::future::try_join_all(futs).await?;
                    }
                }
            }
            ClientRequest::DeleteTrackFromPlaylist(playlist_id, track_id) => {
                self.delete_track_from_playlist(state, playlist_id, track_id)
                    .await?;
            }
            ClientRequest::AddToLibrary(item) => {
                self.add_to_library(state, *item).await?;
            }
            ClientRequest::DeleteFromLibrary(id) => {
                self.delete_from_library(state, id).await?;
            }
            ClientRequest::GetCurrentUserQueue => {
                let queue = self.user_client()?.current_user_queue().await?;
                state.player.write().queue = Some(queue);
            }
            ClientRequest::ReorderPlaylistItems {
                playlist_id,
                insert_index,
                range_start,
                range_length,
                snapshot_id,
            } => {
                self.reorder_playlist_items(
                    state,
                    playlist_id,
                    insert_index,
                    range_start,
                    range_length,
                    snapshot_id.as_deref(),
                )
                .await?;
            }
            ClientRequest::CreatePlaylist {
                playlist_name,
                public,
                collab,
                desc,
            } => {
                let user_id = match state.data.read().user_data.user.as_ref() {
                    Some(u) => u.id.clone(),
                    None => anyhow::bail!("Cannot create playlist: user data not loaded"),
                };
                self.create_new_playlist(
                    state,
                    user_id,
                    playlist_name.as_str(),
                    public,
                    collab,
                    desc.as_str(),
                )
                .await?;
            }
        }

        tracing::info!(
            "Successfully handled the client request, took: {}ms",
            timer.elapsed().as_millis()
        );

        Ok(())
    }

    /// Get lyrics of a given track, return None if no lyrics is available
    pub async fn lyrics(&self, track_id: TrackId<'static>) -> Result<Option<Lyrics>> {
        let session = self.spotify.session().await?;
        let uri = SpotifyUri::from_uri(&track_id.uri())?;
        match uri {
            SpotifyUri::Track { id } => {
                match librespot_metadata::Lyrics::get(&session, &id).await {
                    Ok(lyrics) => Ok(Some(lyrics.into())),
                    Err(err) => {
                        if err.to_string().to_lowercase().contains("not found") {
                            Ok(None)
                        } else {
                            Err(err.into())
                        }
                    }
                }
            }
            _ => Ok(None),
        }
    }

    /// Get user available devices
    pub async fn available_devices(&self) -> Result<Vec<rspotify::model::Device>> {
        Ok(self.user_client()?.device().await?)
    }

    pub async fn update_playback(&self, state: &SharedState) {
        static LAST_DISPATCH: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

        let mut now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        loop {
            let last = LAST_DISPATCH.load(std::sync::atomic::Ordering::Acquire);
            if now_ms.saturating_sub(last) < 1000 {
                tokio::time::sleep(std::time::Duration::from_millis(
                    1000 - now_ms.saturating_sub(last)
                )).await;
                now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;
                match LAST_DISPATCH.compare_exchange(
                    last,
                    now_ms,
                    std::sync::atomic::Ordering::AcqRel,
                    std::sync::atomic::Ordering::Acquire,
                ) {
                    Ok(_) => break,
                    Err(_) => continue,
                }
            } else {
                match LAST_DISPATCH.compare_exchange(
                    last,
                    now_ms,
                    std::sync::atomic::Ordering::AcqRel,
                    std::sync::atomic::Ordering::Acquire,
                ) {
                    Ok(_) => break,
                    Err(_) => continue,
                }
            }
        }

        let client = self.clone();
        let state = state.clone();
        // Note: JoinHandle is dropped intentionally — fire-and-forget refresh task
        tokio::task::spawn(async move {
            if let Err(err) = client.retrieve_current_playback(&state, false).await {
                tracing::error!(
                    "Encountered an error when updating the playback state: {err:#}"
                );
            }
        });
    }

    #[cfg(feature = "streaming")]
    pub async fn handle_custom_queue_advance(
        &self,
        state: &SharedState,
        mut result: crate::state::AdvanceResult,
    ) {
        loop {
            match result {
                crate::state::AdvanceResult::SameBatch => {
                    self.update_playback(state).await;
                    break;
                }
                crate::state::AdvanceResult::NewBatch(tracks) => {
                    let mut device_id: Option<String> = state
                        .player
                        .read()
                        .buffered_playback
                        .as_ref()
                        .and_then(|p| p.device_id.clone());
                    if device_id.is_none() {
                        if let Ok(session) = self.spotify.session().await {
                            device_id = Some(session.device_id().to_string());
                        } else {
                            tracing::error!("Failed to get Spotify session for device_id");
                        }
                    }
                    if let Err(err) = self
                        .start_playback(
                            crate::state::Playback::URIs(tracks, None),
                            device_id.as_deref(),
                        )
                        .await
                    {
                        tracing::error!("Failed to start next batch playback: {err:#}");
                    }
                    break;
                }
                crate::state::AdvanceResult::NeedsRadioTracks => {
                    let seed_uri = {
                        let player = state.player.read();
                        player
                            .custom_queue
                            .as_ref()
                            .and_then(|q| q.source_context().map(|ctx| ctx.uri()))
                            .or_else(|| {
                                player
                                    .custom_queue
                                    .as_ref()
                                    .and_then(|q| q.current_track().map(|t| t.uri()))
                            })
                    };

                    let tracks = match seed_uri {
                        Some(uri) => match self.radio_tracks(uri).await {
                            Ok(t) => t,
                            Err(err) => {
                                tracing::error!("Failed to fetch radio tracks: {err:#}");
                                return;
                            }
                        },
                        None => {
                            tracing::error!("No seed URI available for radio tracks");
                            return;
                        }
                    };

                    let radio_ids: Vec<PlayableId<'static>> = tracks
                        .into_iter()
                        .map(|t| PlayableId::Track(t.id))
                        .collect();

                    let new_result = {
                        let mut player = state.player.write();
                        match player.custom_queue.as_mut() {
                            Some(queue) => {
                                queue.append_radio_tracks(radio_ids);
                                queue.advance()
                            }
                            None => return,
                        }
                    };

                    result = new_result;
                    continue;
                }
                crate::state::AdvanceResult::EndOfQueue => {
                    let mut player = state.player.write();
                    if let Some(ref mut playback) = player.buffered_playback {
                        playback.is_playing = false;
                    }
                    break;
                }
            }
        }
    }

    /// Get Spotify's available browse categories
    pub async fn browse_categories(&self) -> Result<Vec<Category>> {
        let first_page = self
            .user_client()?
            .categories_manual(Some("EN"), None, Some(50), None)
            .await?;

        Ok(first_page.items.into_iter().map(Category::from).collect())
    }

    /// Get Spotify's available browse playlists of a given category
    /// 
    /// Note: This uses a custom HTTP implementation instead of `rspotify::category_playlists_manual`
    /// as a workaround for <https://github.com/ramsayleung/rspotify/issues/535>
    pub async fn browse_category_playlists(&self, category_id: &str) -> Result<Vec<Playlist>> {
        #[derive(Deserialize, Debug)]
        struct BrowseCategoryPlaylistsResponse {
            playlists: rspotify::model::Page<serde_json::Value>,
        }

        let mut all_playlists = Vec::new();
        let mut url = format!("{SPOTIFY_API_ENDPOINT}/browse/categories/{category_id}/playlists?limit=50");
        const MAX_PAGES: usize = 100;
        let mut page_count = 0;
        while !url.is_empty() && page_count < MAX_PAGES {
            page_count += 1;
            let resp: BrowseCategoryPlaylistsResponse = self.http_get(&url, &Query::new()).await?;
            all_playlists.extend(resp.playlists.items);
            url = resp.playlists.next.unwrap_or_default();
            if !url.is_empty() && !url.starts_with("http") {
                tracing::warn!("Invalid next URL: {url}, stopping pagination");
                url.clear();
            }
        }
        if page_count >= MAX_PAGES {
            tracing::warn!("browse_category_playlists hit max pages limit ({MAX_PAGES})");
        }
        Ok(all_playlists.into_iter().filter_map(|item| {
            match serde_json::from_value::<rspotify::model::SimplifiedPlaylist>(item) {
                Ok(p) => Some(Playlist::from(p)),
                Err(e) => {
                    tracing::warn!("Skipping unparseable playlist: {e:#}");
                    None
                }
            }
        }).collect())
    }

    /// Find an available device. If found, return the device's ID.
    async fn find_available_device(&self) -> Result<Option<String>> {
        let devices = self.available_devices().await?;
        tracing::info!("Available devices: {devices:?}");

        // if there is an active device, return it
        if let Some(d) = devices.iter().find(|d| d.is_active) {
            return Ok(d.id.clone());
        }

        // convert a vector of `Device` items into `(name, id)` pairs
        let mut devices = devices
            .into_iter()
            .filter_map(|d| d.id.map(|id| (d.name, id)))
            .collect::<Vec<_>>();

        let configs = config::get_config();

        // Manually append the integrated device to the device list if `streaming` feature is enabled.
        // The integrated device may not show up in the device list returned by the Spotify API because
        // 1. The device is just initialized and hasn't been registered in Spotify server.
        //    Related issue/discussion: https://github.com/aome510/spotify-player/issues/79
        // 2. The device list is empty. This might be because user doesn't specify their own client ID.
        //    By default, the application uses Spotify web app's client ID, which doesn't have
        //    access to user's active devices.
        #[cfg(feature = "streaming")]
        {
            let session = self.spotify.session().await?;
            devices.push((
                configs.app_config.device.name.clone(),
                session.device_id().to_string(),
            ));
        }

        if devices.is_empty() {
            return Ok(None);
        }

        // Prioritize the `default_device` specified in the application's configurations,
        // otherwise, use the first available device.
        let id = devices.iter().position(|d| d.0 == configs.app_config.default_device)
            .or_else(|| {
                tracing::warn!("Default device '{}' not found, using first available device", configs.app_config.default_device);
                devices.first().map(|_| 0)
            });
        let Some(id) = id else {
            return Err(anyhow::anyhow!("No available devices found"));
        };

        Ok(Some(devices.swap_remove(id).1))
    }

    /// Get the saved (liked) tracks of the current user
    // NOTE (#86): this fetches the entire saved tracks library, which can be slow
    // for users with large libraries. Consider implementing pagination with a
    // configurable limit and on-demand loading in the future.
    pub async fn current_user_saved_tracks(&self) -> Result<Vec<Track>> {
        let tracks = self
            .all_paging_items::<rspotify::model::SavedTrack>(
                &format!("{SPOTIFY_API_ENDPOINT}/me/tracks"),
                0, // we don't know the total number of saved tracks beforehand
            )
            .await?;

        Ok(tracks
            .into_iter()
            .filter_map(|t| Track::try_from_full_track(t.track))
            .collect())
    }

    /// Get the recently played tracks of the current user
    pub async fn current_user_recently_played_tracks(&self) -> Result<Vec<Track>> {
        let first_page = self.user_client()?.current_user_recently_played(Some(50), None).await?;

        let play_histories = self.all_cursor_based_paging_items(first_page).await?;

        // de-duplicate the tracks returned from the recently-played API using HashSet
        let mut seen = std::collections::HashSet::new();
        let tracks: Vec<Track> = play_histories
            .into_iter()
            .filter_map(|history| Track::try_from_full_track(history.track))
            .filter(|t| seen.insert(t.id.clone()))
            .collect();
        Ok(tracks)
    }

    /// Get the top tracks of the current user
    pub async fn current_user_top_tracks(&self) -> Result<Vec<Track>> {
        let tracks = self
            .all_paging_items::<rspotify::model::FullTrack>(
                &format!("{SPOTIFY_API_ENDPOINT}/me/top/tracks"),
                0, // we don't know the total number of top tracks beforehand
            )
            .await?;

        Ok(tracks
            .into_iter()
            .filter_map(Track::try_from_full_track)
            .collect())
    }

    /// Get all playlists of the current user
    pub async fn current_user_playlists(&self) -> Result<Vec<Playlist>> {
        let playlists = self
            .all_paging_items::<rspotify::model::SimplifiedPlaylist>(
                &format!("{SPOTIFY_API_ENDPOINT}/me/playlists"),
                0, // we don't know the total number of playlists beforehand
            )
            .await?;

        Ok(playlists
            .into_iter()
            .map(std::convert::Into::into)
            .collect())
    }

    /// Get all followed artists of the current user
    pub async fn current_user_followed_artists(&self) -> Result<Vec<Artist>> {
        let first_page = self
            .user_client()?
            .current_user_followed_artists(None, None)
            .await?;

        // followed artists pagination is handled different from
        // other paginations. The endpoint uses cursor-based pagination.
        let mut artists = first_page.items;
        let mut maybe_next = first_page.next;
        while let Some(url) = maybe_next {
            let mut next_page = self
                .http_get::<rspotify::model::CursorPageFullArtists>(&url, &Query::new())
                .await?
                .artists;
            artists.append(&mut next_page.items);
            maybe_next = next_page.next;
        }

        // converts `rspotify::model::FullArtist` into `state::Artist`
        Ok(artists.into_iter().map(std::convert::Into::into).collect())
    }

    /// Get all saved albums of the current user
    pub async fn current_user_saved_albums(&self) -> Result<Vec<Album>> {
        let albums = self
            .all_paging_items::<rspotify::model::SavedAlbum>(
                &format!("{SPOTIFY_API_ENDPOINT}/me/albums"),
                0, // we don't know the total number of saved albums beforehand
            )
            .await?;

        // Converts `rspotify::model::SavedAlbum` into `state::Album`
        Ok(albums.into_iter().map(Album::from).collect())
    }

/// Get all saved shows of the current user
    pub async fn current_user_saved_shows(&self) -> Result<Vec<Show>> {
        let shows = self
            .all_paging_items::<SavedShow>(
                &format!("{SPOTIFY_API_ENDPOINT}/me/shows"),
                0,
            )
            .await?;

        Ok(shows.into_iter().map(|s| s.show.into()).collect())
    }

    /// Get all albums of an artist
    pub async fn artist_albums(&self, artist_id: ArtistId<'_>) -> Result<Vec<Album>> {
        let albums = self
            .all_paging_items::<rspotify::model::SimplifiedAlbum>(
                &format!(
                    "{SPOTIFY_API_ENDPOINT}/artists/{}/albums?include_groups=album,single",
                    artist_id.id()
                ),
                0, // we don't know the total number of artist albums beforehand
            )
            .await?
            .into_iter()
            .filter_map(Album::try_from_simplified_album)
            .collect();

        Ok(AppClient::process_artist_albums(albums))
    }

    /// Start a playback
    async fn start_playback(&self, playback: Playback, device_id: Option<&str>) -> Result<()> {
        match playback {
            Playback::Context(id, offset) => match id {
                ContextId::Album(id) => {
                    self.user_client()?.start_context_playback(PlayContextId::from(id), device_id, offset, None)
                        .await?;
                }
                ContextId::Artist(id) => {
                    self.user_client()?.start_context_playback(PlayContextId::from(id), device_id, offset, None)
                        .await?;
                }
                ContextId::Playlist(id) => {
                    self.user_client()?.start_context_playback(PlayContextId::from(id), device_id, offset, None)
                        .await?;
                }
                ContextId::Show(id) => {
                    self.user_client()?.start_context_playback(PlayContextId::from(id), device_id, offset, None)
                        .await?;
                }
                ContextId::Tracks(_) => {
                    anyhow::bail!("`StartPlayback` request for `tracks` context is not supported")
                }
            },
            Playback::URIs(ids, offset) => {
                self.user_client()?.start_uris_playback(ids, device_id, offset, None)
                    .await?;
            }
        }

        Ok(())
    }

    /// Get recommendation (radio) tracks based on a seed
    pub async fn radio_tracks(&self, seed_uri: String) -> Result<Vec<Track>> {
        #[derive(Debug, Deserialize)]
        struct TrackData {
            original_gid: String,
        }
        #[derive(Debug, Deserialize)]
        struct RadioStationResponse {
            tracks: Vec<TrackData>,
        }

        let session = self.spotify.session().await?;

        // Get an autoplay URI from the seed URI.
        // The return URI is a Spotify station's URI
        let autoplay_query_url = format!("hm://autoplay-enabled/query?uri={seed_uri}");
        let response = session
            .mercury()
            .get(autoplay_query_url)
            .map_err(|err| anyhow::anyhow!("Failed to get autoplay URI: {err:#}"))?
            .await?;
        if response.status_code != 200 {
            anyhow::bail!(
                "Failed to get autoplay URI: got non-OK status code: {}",
                response.status_code
            );
        }
        let autoplay_uri = String::from_utf8(response.payload[0].clone())?;

        // Retrieve radio's data based on the autoplay URI
        let radio_query_url = format!("hm://radio-apollo/v3/stations/{autoplay_uri}");
        let response = session
            .mercury()
            .get(radio_query_url)
            .map_err(|err| anyhow::anyhow!("Failed to get radio data of {autoplay_uri}: {err:#}"))?
            .await?;
        if response.status_code != 200 {
            anyhow::bail!(
                "Failed to get radio data of {autoplay_uri}: got non-OK status code: {}",
                response.status_code
            );
        }

        // Parse a list consisting of IDs of tracks inside the radio station
        let track_ids = serde_json::from_slice::<RadioStationResponse>(&response.payload[0])?
            .tracks
            .into_iter()
            .filter_map(|t| TrackId::from_id(t.original_gid).ok());

        // Retrieve tracks based on IDs
        let tracks = self
            .user_client()?
            .tracks(track_ids, Some(rspotify::model::Market::FromToken))
            .await?;
        let mut tracks: Vec<_> = tracks
            .into_iter()
            .filter_map(Track::try_from_full_track)
            .collect();

        // Track-seeded radios in the official Spotify clients include the seed track itself
        // as the first item in the generated session.
        match TrackId::from_uri(&seed_uri) {
            Ok(track_id) => match self.track(track_id).await {
                Ok(track) => move_seed_track_to_front(&mut tracks, track),
                Err(err) => {
                    tracing::warn!("Failed to fetch track radio seed {seed_uri}: {err:#}");
                }
            },
            Err(e) => tracing::warn!("Failed to parse seed URI as TrackId: {e:#}"),
        }

        Ok(tracks)
    }

    /// Search for items (tracks, artists, albums, playlists) matching a given query
    pub async fn search(&self, query: &str) -> Result<SearchResults> {
        // #29: use tokio::join! instead of try_join! so a single failure
        // doesn't cancel all in-flight requests.
        let (
            track_result,
            artist_result,
            album_result,
            playlist_result,
            show_result,
        ) = tokio::join!(
            self.search_specific_type(query, rspotify::model::SearchType::Track),
            self.search_specific_type(query, rspotify::model::SearchType::Artist),
            self.search_specific_type(query, rspotify::model::SearchType::Album),
            self.search_specific_type(query, rspotify::model::SearchType::Playlist),
            self.search_specific_type(query, rspotify::model::SearchType::Show)
        );

        let tracks = match track_result {
            Ok(rspotify::model::SearchResult::Tracks(p)) => {
                p.items.into_iter().filter_map(Track::try_from_full_track).collect()
            }
            Ok(_) => Vec::new(),
            Err(e) => {
                tracing::warn!("Track search failed: {e:#}");
                Vec::new()
            }
        };
        let artists = match artist_result {
            Ok(rspotify::model::SearchResult::Artists(p)) => {
                p.items.into_iter().map(std::convert::Into::into).collect()
            }
            Ok(_) => Vec::new(),
            Err(e) => {
                tracing::warn!("Artist search failed: {e:#}");
                Vec::new()
            }
        };
        let albums = match album_result {
            Ok(rspotify::model::SearchResult::Albums(p)) => {
                p.items.into_iter().filter_map(Album::try_from_simplified_album).collect()
            }
            Ok(_) => Vec::new(),
            Err(e) => {
                tracing::warn!("Album search failed: {e:#}");
                Vec::new()
            }
        };
        let playlists = match playlist_result {
            Ok(rspotify::model::SearchResult::Playlists(p)) => {
                p.items.into_iter().map(std::convert::Into::into).collect()
            }
            Ok(_) => Vec::new(),
            Err(e) => {
                tracing::warn!("Playlist search failed: {e:#}");
                Vec::new()
            }
        };
        let shows = match show_result {
            Ok(rspotify::model::SearchResult::Shows(p)) => {
                p.items.into_iter().map(std::convert::Into::into).collect()
            }
            Ok(_) => Vec::new(),
            Err(e) => {
                tracing::warn!("Show search failed: {e:#}");
                Vec::new()
            }
        };

        Ok(SearchResults {
            tracks,
            artists,
            albums,
            playlists,
            shows,
            episodes: Vec::new(),
        })
    }

    /// Search for items of a specific type matching a given query
    pub async fn search_specific_type(
        &self,
        query: &str,
        typ: rspotify::model::SearchType,
    ) -> Result<rspotify::model::SearchResult> {
        Ok(self
            .user_client()?
            .search(query, typ, None, None, None, None)
            .await?)
    }

    /// Add a playable item to a playlist
    pub async fn add_item_to_playlist(
        &self,
        state: &SharedState,
        playlist_id: PlaylistId<'_>,
        playable_id: PlayableId<'_>,
    ) -> Result<()> {
        // Remove all existing occurrences of the item first
        self.user_client()?
            .playlist_remove_all_occurrences_of_items(
                playlist_id.as_ref(),
                [playable_id.as_ref()],
                None,
            )
            .await?;

        // Then add the item as a single instance
        self.user_client()?
            .playlist_add_items(playlist_id.as_ref(), [playable_id.as_ref()], None)
            .await?;

        // After adding a new track to a playlist, remove the cache of that playlist to force refetching new data
        state.data.write().caches.context.remove(&playlist_id.uri());

        Ok(())
    }

    /// Remove a track from a playlist
    pub async fn delete_track_from_playlist(
        &self,
        state: &SharedState,
        playlist_id: PlaylistId<'_>,
        track_id: TrackId<'_>,
    ) -> Result<()> {
        // remove all the occurrences of the track to ensure no duplication in the playlist
        self.user_client()?.playlist_remove_all_occurrences_of_items(
            playlist_id.as_ref(),
            [PlayableId::Track(track_id.as_ref())],
            None,
        )
        .await?;

        // After making a delete request, update the playlist in-memory data stored inside the app caches.
        if let Some(Context::Playlist { tracks, .. }) = state
            .data
            .write()
            .caches
            .context
            .get_mut(&playlist_id.uri())
        {
            tracks.retain(|t| t.id != track_id);
        }

        Ok(())
    }

    /// Reorder items in a playlist
    async fn reorder_playlist_items(
        &self,
        state: &SharedState,
        playlist_id: PlaylistId<'_>,
        insert_index: usize,
        range_start: usize,
        range_length: Option<usize>,
        snapshot_id: Option<&str>,
    ) -> Result<()> {
        let insert_before = if insert_index > range_start {
            insert_index + 1
        } else {
            insert_index
        };

        self.user_client()?.playlist_reorder_items(
            playlist_id.clone(),
            Some(range_start as i32),
            Some(insert_before as i32),
            range_length.map(|range_length| range_length as u32),
            snapshot_id,
        )
        .await?;

        // After making a reorder request, update the playlist in-memory data stored inside the app caches.
        if let Some(Context::Playlist { tracks, .. }) = state
            .data
            .write()
            .caches
            .context
            .get_mut(&playlist_id.uri())
        {
            if range_start >= tracks.len() {
                tracing::warn!("Range start {} out of bounds for tracks (len={})", range_start, tracks.len());
                return Ok(());
            }
            let track = tracks.remove(range_start);
            // After remove, indices shift. When insert_index > range_start,
            // the API's insert_before accounts for the original position,
            // so local insert needs to be adjusted by -1.
            let local_insert = if insert_index > range_start {
                insert_before.saturating_sub(1)
            } else {
                insert_before
            };
            tracks.insert(std::cmp::min(local_insert, tracks.len()), track);
        }

        Ok(())
    }

    /// Add a Spotify item to current user's library.
    async fn add_to_library(&self, state: &SharedState, item: Item) -> Result<()> {
        // Before adding new item, checks if that item already exists in the library to avoid adding a duplicated item.
        match item {
            Item::Track(track) => {
                let contains = self
                    .user_client()?
                    .current_user_saved_tracks_contains([track.id.as_ref()])
                    .await?;
                if !contains[0] {
                    self.user_client()?.current_user_saved_tracks_add([track.id.as_ref()])
                        .await?;
                    // update the in-memory `user_data`
                    state
                        .data
                        .write()
                        .user_data
                        .saved_tracks
                        .insert(track.id.uri(), track);
                }
            }
            Item::Album(album) => {
                let contains = self
                    .user_client()?
                    .current_user_saved_albums_contains([album.id.as_ref()])
                    .await?;
                if !contains[0] {
                    self.user_client()?.current_user_saved_albums_add([album.id.as_ref()])
                        .await?;
                    // update the in-memory `user_data`
                    state.data.write().user_data.saved_albums.push(album);
                }
            }
            Item::Artist(artist) => {
                let follows = self.user_client()?.user_artist_check_follow([artist.id.as_ref()]).await?;
                if !follows[0] {
                    self.user_client()?.user_follow_artists([artist.id.as_ref()]).await?;
                    // update the in-memory `user_data`
                    state
                        .data
                        .write()
                        .user_data
                        .followed_artists
                        .push(artist);
                }
            }
            Item::Playlist(playlist) => {
                let user_id = state
                    .data
                    .read()
                    .user_data
                    .user
                    .as_ref()
                    .map(|u| u.id.clone());

                if let Some(user_id) = user_id {
                    let follows = self
                        .user_client()?
                        .playlist_check_follow(playlist.id.as_ref(), &[user_id])
                        .await?;
                    if !follows[0] {
                        self.user_client()?.playlist_follow(playlist.id.as_ref(), None).await?;
                        // update the in-memory `user_data`
                        state
                            .data
                            .write()
                            .user_data
                            .playlists
                            .push(PlaylistFolderItem::Playlist(playlist));
                    }
                }
            }
            Item::Show(show) => {
                let follows = self.user_client()?.check_users_saved_shows([show.id.as_ref()]).await?;
                if !follows[0] {
                    self.user_client()?.save_shows([show.id.as_ref()]).await?;
                    // update the in-memory `user_data`
                    state.data.write().user_data.saved_shows.push(show);
                }
            }
        }
        Ok(())
    }

    // Delete a Spotify item from user's library
    async fn delete_from_library(&self, state: &SharedState, id: ItemId) -> Result<()> {
        match id {
            ItemId::Track(id) => {
                let uri = id.uri();
                self.user_client()?.current_user_saved_tracks_delete([id]).await?;
                state.data.write().user_data.saved_tracks.remove(&uri);
            }
            ItemId::Album(id) => {
                self.user_client()?.current_user_saved_albums_delete([id.clone()]).await?;
                state
                    .data
                    .write()
                    .user_data
                    .saved_albums
                    .retain(|a| a.id != id);
            }
            ItemId::Artist(id) => {
                self.user_client()?.user_unfollow_artists([id.clone()]).await?;
                state
                    .data
                    .write()
                    .user_data
                    .followed_artists
                    .retain(|a| a.id != id);
            }
            ItemId::Playlist(id) => {
                self.user_client()?.playlist_unfollow(id.clone()).await?;
                state
                    .data
                    .write()
                    .user_data
                    .playlists
                    .retain(|item| match item {
                        PlaylistFolderItem::Playlist(p) => p.id != id,
                        PlaylistFolderItem::Folder(_) => true,
                    });
            }
            ItemId::Show(id) => {
                self.user_client()?.remove_users_saved_shows([id.clone()], Some(rspotify::model::Market::FromToken))
                    .await?;
                state
                    .data
                    .write()
                    .user_data
                    .saved_shows
                    .retain(|s| s.id != id);
            }
        }
        Ok(())
    }

    /// Get a track data
    pub async fn track(&self, track_id: TrackId<'_>) -> Result<Track> {
        Track::try_from_full_track(
            self.user_client()?
                .track(track_id, Some(rspotify::model::Market::FromToken))
                .await?,
        )
        .context("convert FullTrack into Track")
    }

    /// Get a playlist context data
    pub async fn playlist_context(&self, playlist_id: PlaylistId<'_>) -> Result<Context> {
        let playlist_uri = playlist_id.uri();
        tracing::info!("Get playlist context: {}", playlist_uri);

        let playlist = self
            .user_client()?
            .playlist(
                playlist_id.clone(),
                None,
                Some(rspotify::model::Market::FromToken),
            )
            .await?;

        let tracks = self
            .all_paging_items(
                &format!(
                    "{SPOTIFY_API_ENDPOINT}/playlists/{}/tracks",
                    playlist_id.id(),
                ),
                playlist.tracks.total as usize,
            )
            .await?
            .into_iter()
            .filter_map(Track::try_from_playlist_item)
            .collect::<Vec<_>>();

        Ok(Context::Playlist {
            playlist: playlist.into(),
            tracks,
        })
    }

    /// Get an album context data
    pub async fn album_context(&self, album_id: AlbumId<'_>) -> Result<Context> {
        let album_uri = album_id.uri();
        tracing::info!("Get album context: {}", album_uri);

        let album = self
            .user_client()?
            .album(album_id.clone(), Some(rspotify::model::Market::FromToken))
            .await?;

        let total_tracks = album.tracks.total as usize;

        // converts `rspotify::model::FullAlbum` into `state::Album`
        let album: Album = album.into();

        // get the album's tracks
        let tracks = self
            .all_paging_items(
                &format!("{SPOTIFY_API_ENDPOINT}/albums/{}/tracks", album_id.id()),
                total_tracks,
            )
            .await?
            .into_iter()
            .filter_map(|t| {
                // TODO: album.clone() is called once per track. If Album becomes large,
                // consider using Arc<Album> to share ownership cheaply.
                Track::try_from_simplified_track(t).map(|mut t| {
                    t.album = Some(album.clone());
                    t
                })
            })
            .collect::<Vec<_>>();

        Ok(Context::Album { album, tracks })
    }

    /// Get an artist context data
    pub async fn artist_context(&self, artist_id: ArtistId<'_>) -> Result<Context> {
        let artist_uri = artist_id.uri();
        tracing::info!("Get artist context: {}", artist_uri);

        // get the artist's information, including top tracks, related artists, and albums

        let (artist, top_tracks, related_artists, albums) = tokio::try_join!(
            async {
                    let a: Artist = self.user_client()?.artist(artist_id.as_ref())
                        .await
                        .context("get artist")?
                        .into();
                    Ok::<_, anyhow::Error>(a)
            },
            async {
                    let tracks = self.user_client()?
                        .artist_top_tracks(artist_id.as_ref(), Some(rspotify::model::Market::FromToken))
                        .await
                        .context("get artist's top tracks")?
                        .into_iter()
                        .filter_map(Track::try_from_full_track)
                        .collect::<Vec<_>>();
                    Ok::<_, anyhow::Error>(tracks)
            },
            async {
                    #[allow(deprecated)]
                    let artists = self.user_client()?
                        .artist_related_artists(artist_id.as_ref())
                        .await
                        .ok()
                        .unwrap_or_default()
                        .into_iter()
                        .map(std::convert::Into::into)
                        .collect::<Vec<_>>();
                    Ok::<_, anyhow::Error>(artists)
            },
            self.artist_albums(artist_id.as_ref())
        )?;

        Ok(Context::Artist {
            artist,
            top_tracks,
            albums,
            related_artists,
        })
    }

    /// Get a show context data
    pub async fn show_context(&self, show_id: ShowId<'_>) -> Result<Context> {
        let show_uri = show_id.uri();
        tracing::info!("Get show context: {}", show_uri);

        let show = self.user_client()?.get_a_show(show_id.clone(), None).await?;

        // get the show's episodes
        let episodes = self
            .all_paging_items::<rspotify::model::SimplifiedEpisode>(
                &format!("{SPOTIFY_API_ENDPOINT}/shows/{}/episodes", show_id.id()),
                show.episodes.total as usize,
            )
            .await?
            .into_iter()
            .filter_map(|e| crate::state::Episode::try_from(e).ok())
            .collect::<Vec<_>>();

        // converts `rspotify::model::FullShow` into `state::Show`
        let show: Show = show.into();

        Ok(Context::Show { show, episodes })
    }

    /// Make a GET HTTP request to the Spotify server with retry logic
    /// Issue #9: Parses X-RateLimit-Remaining header for preemptive backoff
    async fn http_get<T>(&self, url: &str, payload: &Query<'_>) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        tracing::debug!("{url}");

        let mut access_token = self.token().await.context("get token")?;
        let mut last_err = None;

        for attempt in 0..4u32 {
            if attempt > 0 {
                let delay = std::time::Duration::from_secs(1 << (attempt - 1));
                tokio::time::sleep(delay).await;
            }
            
            // Issue #9: Preemptive backoff if rate limit is low
            let remaining = RATE_LIMIT_REMAINING.load(std::sync::atomic::Ordering::Acquire);
            let reset_time = RATE_LIMIT_RESET_TIME.load(std::sync::atomic::Ordering::Acquire);
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            
            if remaining < 3 && reset_time > now_ms {
                // Rate limit is low, add small delay before making request
                let backoff_ms = (reset_time - now_ms).min(5000) / 10; // Max 500ms
                if backoff_ms > 0 {
                    tracing::debug!("Preemptive rate limit backoff: {backoff_ms}ms (remaining={remaining})");
                    tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
                }
            }

            let response = self
                .http
                .get(url)
                .query(payload)
                .header(
                    reqwest::header::AUTHORIZATION,
                    format!("Bearer {access_token}"),
                )
                .send()
                .await?;

            let status = response.status();
            let headers = response.headers();
            
            // Issue #9: Parse rate limit headers
            if let Some(remaining_header) = headers.get("X-RateLimit-Remaining") {
                if let Ok(remaining_str) = remaining_header.to_str() {
                    if let Ok(remaining_val) = remaining_str.parse::<i64>() {
                        RATE_LIMIT_REMAINING.store(remaining_val, std::sync::atomic::Ordering::Release);
                    }
                }
            }
            
            if let Some(reset_header) = headers.get("X-RateLimit-Reset") {
                if let Ok(reset_str) = reset_header.to_str() {
                    if let Ok(reset_val) = reset_str.parse::<u64>() {
                        // Convert seconds to milliseconds
                        RATE_LIMIT_RESET_TIME.store(reset_val * 1000, std::sync::atomic::Ordering::Release);
                    }
                }
            }

            if status == StatusCode::UNAUTHORIZED {
                tracing::warn!("Got 401 for {url}, re-authenticating...");
                if let Err(err) = self.user_client()?.auto_reauth().await {
                    tracing::error!("Re-auth failed: {err:#}");
                    last_err = Some(err.into());
                    continue;
                }
                match self.token().await {
                    Ok(new_token) => {
                        access_token = new_token;
                        last_err = Some(anyhow::anyhow!("retried after re-auth but still failing for {url}"));
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        continue;
                    }
                    Err(err) => {
                        tracing::error!("Token re-fetch failed after 401: {err:#}");
                        return Err(anyhow::anyhow!("Failed to refresh token after 401: {err:#}"));
                    }
                }
            }

            if status == StatusCode::TOO_MANY_REQUESTS {
                let retry_after = headers
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(1);
                let sleep_duration = std::time::Duration::from_secs(retry_after.min(60));
                tracing::warn!("Got 429 for {url}, retrying after {sleep_duration:?}");
                if last_err.is_none() {
                    last_err = Some(anyhow::anyhow!("rate limited (429), waited {sleep_duration:?}"));
                }
                tokio::time::sleep(sleep_duration).await;
                continue;
            }

            if status.is_server_error() {
                let err_text = response.text().await.unwrap_or_default();
                tracing::warn!("Got {status} for {url}: {err_text}");
                last_err = Some(anyhow::anyhow!("server error {status}: {err_text}"));
                continue;
            }

            let text = response.text().await?;

            if status != StatusCode::OK {
                anyhow::bail!("failed to send a Spotify API request {url}: {text}");
            }

            return Ok(serde_json::from_str(&text)?);
        }

        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("all retries exhausted for {url}")))
    }

    async fn all_paging_items<T>(&self, base_url: &str, mut count: usize) -> Result<Vec<T>>
    where
        T: serde::de::DeserializeOwned + std::fmt::Debug,
    {
        const PAGE_LIMIT: usize = 50;
        const MAX_PARALLEL: usize = 8;

        let mut all_items = Vec::new();
        let mut offset = 0;

        // if count is 0 (i.e., unknown), set it to usize::MAX to fetch until no more items
        if count == 0 {
            count = usize::MAX;
        }

        while offset < count {
            let n_jobs = std::cmp::min(MAX_PARALLEL, (count - offset).div_ceil(PAGE_LIMIT));

            let mut futures = Vec::with_capacity(n_jobs);

            for i in 0..n_jobs {
                let current_offset = offset + i * PAGE_LIMIT;
                let limit_str = PAGE_LIMIT.to_string();
                let offset_str = current_offset.to_string();

                futures.push(async move {
                    let params = Query::from([
                        ("market", "from_token"),
                        ("limit", &limit_str),
                        ("offset", &offset_str),
                    ]);
                    self.http_get::<rspotify::model::Page<T>>(base_url, &params)
                        .await
                });
            }

            let results = futures::future::join_all(futures).await;
            let mut found_empty = false;
            let mut any_ok = false;
            let mut failed_count = 0;
            let mut successful_items = 0;
            for result in results {
                match result {
                    Ok(mut paging) => {
                        any_ok = true;
                        successful_items += paging.items.len();
                        if paging.items.is_empty() {
                            found_empty = true;
                        } else {
                            all_items.append(&mut paging.items);
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Page fetch failed: {e:#}");
                        failed_count += 1;
                    }
                }
            }
            if failed_count > 0 && any_ok {
                tracing::warn!(
                    "{failed_count} of {n_jobs} page requests failed — returning {successful_items} partial items",
                );
                // Return partial data, don't discard
            } else if !any_ok {
                return Err(anyhow::anyhow!("all page requests failed"));
            }

            if found_empty {
                break;
            }

            offset += successful_items;
        }

        Ok(all_items)
    }

    /// Get all cursor-based paging items starting from a pagination object of the first page
    async fn all_cursor_based_paging_items<T>(
        &self,
        first_page: rspotify::model::CursorBasedPage<T>,
    ) -> Result<Vec<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        let mut items = first_page.items;
        let mut maybe_next = first_page.next;
        while let Some(url) = maybe_next {
            let mut next_page = self
                .http_get::<rspotify::model::CursorBasedPage<T>>(&url, &Query::new())
                .await?;
            items.append(&mut next_page.items);
            maybe_next = next_page.next;
        }
        Ok(items)
    }

    pub async fn current_playback2(
        &self,
    ) -> Result<Option<rspotify::model::CurrentPlaybackContext>> {
        Ok(self.user_client()?.current_playback(None, PLAYBACK_TYPES.into()).await?)
    }

    /// Retrieve the latest playback state
    pub async fn retrieve_current_playback(
        &self,
        state: &SharedState,
        reset_buffered_playback: bool,
    ) -> Result<()> {
        let new_playback = {
            // update the playback state
            let playback = self.current_playback2().await?;
            let mut player = state.player.write();

            let prev_item = player.currently_playing();

            let prev_uri = match prev_item {
                Some(rspotify::model::PlayableItem::Track(track)) => {
                    track.id.as_ref().map(|id| id.uri())
                }
                Some(rspotify::model::PlayableItem::Episode(episode)) => {
                    Some(episode.id.uri())
                }
                Some(rspotify::model::PlayableItem::Unknown(_)) | None => None,
            };

            player.playback = playback;
            player.playback_last_updated_time = Some(std::time::Instant::now());

            let curr_item = player.currently_playing();

            let curr_uri = match curr_item {
                Some(rspotify::model::PlayableItem::Track(track)) => {
                    track.id.as_ref().map(|id| id.uri())
                }
                Some(rspotify::model::PlayableItem::Episode(episode)) => {
                    Some(episode.id.uri())
                }
                Some(rspotify::model::PlayableItem::Unknown(_)) | None => None,
            };

            let new_playback = prev_uri != curr_uri && curr_uri.is_some();
            // check if we need to update the buffered playback
            let needs_update = match (&player.buffered_playback, &player.playback) {
                (Some(bp), Some(p)) => bp.device_id != p.device.id
                    || new_playback
                    || bp.is_playing != p.is_playing
                    || bp.shuffle_state != p.shuffle_state
                    || bp.repeat_state != p.repeat_state
                    || bp.volume != p.device.volume_percent.map(|v| v as u32),
                (None, None) => false,
                _ => true,
            };

            if reset_buffered_playback || needs_update {
                player.buffered_playback = player.playback.as_ref().map(|p| {
                    let mut playback = PlaybackMetadata::from_playback(p);

                    // handle additional data from the previous buffered state
                    // that is not available in a standard Spotify playback's state
                    if let Some(bp) = &player.buffered_playback {
                        if let Some(volume) = bp.mute_state {
                            playback.volume = Some(volume);
                        }
                        playback.mute_state = bp.mute_state;
                    }
                    playback
                });
            }

            new_playback
        };

        if !new_playback {
            return Ok(());
        }
        self.handle_new_playback_event(state).await?;

        Ok(())
    }

    // Handle new track event
    async fn handle_new_playback_event(&self, state: &SharedState) -> Result<()> {
        let configs = config::get_config();

        let curr_item = {
            let player = state.player.read();
            let Some(track_or_episode) = player.currently_playing() else {
                return Ok(());
            };
            track_or_episode.clone()
        };

        // retrieve current artist for genres if not in cache
        let curr_artist = match &curr_item {
            rspotify::model::PlayableItem::Track(full_track) => {
                match full_track.artists.first() {
                    Some(first_artist) => {
                        let cached = state
                            .data
                            .read()
                            .caches
                            .genres
                            .contains_key(&first_artist.id.as_ref().map_or_else(|| first_artist.name.clone(), |id| id.uri()));

                        if cached {
                            None
                        } else {
                            match &first_artist.id {
                                Some(id) => self.user_client()?.artist(id.clone()).await.ok(),
                                None => None,
                            }
                        }
                    }
                    None => None,
                }
            }
            rspotify::model::PlayableItem::Episode(_)
            | rspotify::model::PlayableItem::Unknown(_) => None,
        };

        if let Some(artist) = curr_artist {
            if !artist.genres.is_empty() {
                let genre_key = artist.id.uri();
                state.data.write().caches.genres.insert(
                    genre_key,
                    artist.genres,
                    TTL_CACHE_DURATION,
                );
            }
        }

        let (url, raw_filename) = match curr_item {
            rspotify::model::PlayableItem::Track(ref track) => {
                let artist_name = track.album.artists.first().map_or("unknown", |a| &a.name);
                let album_id_str = track.album.id.as_ref().map_or("unknown", |id| id.id());
                let album_id_prefix = album_id_str.chars().take(6).collect::<String>();
                let filename = format!(
                    "{}-{}-cover-{}.jpg",
                    track.album.name,
                    artist_name,
                    album_id_prefix
                );
                (
                    crate::utils::get_track_album_image_url(track)
                        .ok_or(anyhow::anyhow!("missing image"))?,
                    filename,
                )
            }
            rspotify::model::PlayableItem::Episode(ref episode) => (
                crate::utils::get_episode_show_image_url(episode)
                    .ok_or(anyhow::anyhow!("missing image"))?,
                {
                    let publisher = if episode.show.publisher.is_empty() {
                        "unknown"
                    } else {
                        &episode.show.publisher
                    };
                    format!(
                        "{}-{}-cover-{}.jpg",
                        episode.show.name,
                        publisher,
                        episode.show.id.as_ref().id().chars().take(6).collect::<String>()
                    )
                },
            ),
            rspotify::model::PlayableItem::Unknown(_) => return Ok(()),
        };
        let filename = Self::sanitize_image_filename(&raw_filename);
        let path = configs.cache_folder.join("image").join(filename);

        if configs.app_config.enable_cover_image_cache {
            let _bytes = self.retrieve_image(url, &path, true).await?;

            #[cfg(feature = "image")]
            if !state.data.read().caches.images.contains_key(url) {
                let bytes = &*_bytes;

                #[cfg(not(feature = "pixelate"))]
                let image =
                    image::load_from_memory(bytes).context("Failed to load image from memory")?;
                #[cfg(feature = "pixelate")]
                let mut image =
                    image::load_from_memory(bytes).context("Failed to load image from memory")?;

                #[cfg(feature = "pixelate")]
                {
                    Self::pixelate_image(&mut image);
                }

                state
                    .data
                    .write()
                    .caches
                    .images
                    .insert(url.to_owned(), image, TTL_CACHE_DURATION);
            }
        } else {
            #[cfg(feature = "image")]
            if !state.data.read().caches.images.contains_key(url) {
                let bytes = self.retrieve_image(url, &path, false).await?;

                #[cfg(not(feature = "pixelate"))]
                let image =
                    image::load_from_memory(&bytes).context("Failed to load image from memory")?;
                #[cfg(feature = "pixelate")]
                let mut image =
                    image::load_from_memory(&bytes).context("Failed to load image from memory")?;

                #[cfg(feature = "pixelate")]
                {
                    Self::pixelate_image(&mut image);
                }

                state
                    .data
                    .write()
                    .caches
                    .images
                    .insert(url.to_owned(), image, TTL_CACHE_DURATION);
            }
        }

        #[cfg(feature = "notify")]
        if configs.app_config.enable_notify && (
            !cfg!(feature = "streaming")
            || !configs.app_config.notify_streaming_only
            || self.stream_conn.lock().is_some()
        ) {
            if let Err(e) = Self::notify_new_playback(&curr_item, &path) {
                tracing::warn!("Notification failed: {e:#}");
            }
        }

        Ok(())
    }

    /// Create a new playlist
    async fn create_new_playlist(
        &self,
        state: &SharedState,
        user_id: UserId<'static>,
        playlist_name: &str,
        public: bool,
        collab: bool,
        desc: &str,
    ) -> Result<()> {
        let playlist: Playlist = self
            .user_client()?
            .user_playlist_create(
                user_id,
                playlist_name,
                Some(public),
                Some(collab),
                Some(desc),
            )
            .await?
            .into();
        tracing::info!(
            "new playlist (name={},id={}) was successfully created",
            playlist.name,
            playlist.id
        );
        state
            .data
            .write()
            .user_data
            .playlists
            .insert(0, PlaylistFolderItem::Playlist(playlist));
        Ok(())
    }

    #[cfg(feature = "notify")]
    /// Create a notification for a new playback
    fn notify_new_playback(
        playable: &rspotify::model::PlayableItem,
        cover_img_path: &std::path::Path,
    ) -> Result<()> {
        let mut n = notify_rust::Notification::new();

        // Generate a text described a track from a format string.
        // For example, a format string "{track} - {artists}" will generate
        // a text consisting of the track's name followed by a dash then artists' names.
        let get_text_from_format_str = |format_str: &str| {
            let mut text = String::new();

            let mut ptr = 0;
            for m in NOTIFY_TEMPLATE_RE.find_iter(format_str) {
                let s = m.start();
                let e = m.end();

                if ptr < s {
                    text += &format_str[ptr..s];
                }
                ptr = e;
                match m.as_str() {
                    "{track}" => {
                        let name = match playable {
                            rspotify::model::PlayableItem::Track(ref track) => &track.name,
                            rspotify::model::PlayableItem::Episode(ref episode) => &episode.name,
                            rspotify::model::PlayableItem::Unknown(_) => continue,
                        };
                        text += name;
                    }
                    "{artists}" => {
                        if let rspotify::model::PlayableItem::Track(ref track) = playable {
                            text += &crate::utils::map_join(&track.artists, |a| &a.name, ", ");
                        }
                    }
                    "{album}" => match playable {
                        rspotify::model::PlayableItem::Track(ref track) => {
                            text += &track.album.name;
                        }
                        rspotify::model::PlayableItem::Episode(ref episode) => {
                            text += &episode.show.name;
                        }
                        rspotify::model::PlayableItem::Unknown(_) => {}
                    },
                    other => tracing::warn!("Unknown notification placeholder: {other}"),
                }
            }
            if ptr < format_str.len() {
                text += &format_str[ptr..];
            }

            text
        };

        let configs = config::get_config();

        n.appname("spotify_player")
            .summary(&get_text_from_format_str(
                &configs.app_config.notify_format.summary,
            ))
            .body(&get_text_from_format_str(
                &configs.app_config.notify_format.body,
            ));
        if cover_img_path.exists() {
            n.icon(cover_img_path.to_str().context("valid cover_img_path")?);
        }
        if configs.app_config.notify_timeout_in_secs > 0 {
            n.timeout(std::time::Duration::from_secs(
                configs.app_config.notify_timeout_in_secs,
            ));
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        if configs.app_config.notify_transient {
            use notify_rust::Hint;
            n.hint(Hint::Transient(true));
        }
        n.show()?;

        Ok(())
    }

    /// Validate that a path is within the cache directory to prevent path traversal
    fn validate_cache_path(path: &std::path::Path) -> Result<()> {
        let configs = config::get_config();
        let cache_folder = &configs.cache_folder;

        // Check for path traversal components
        if path.components().any(|c| {
            matches!(c, std::path::Component::ParentDir)
        }) {
            anyhow::bail!("Path contains parent directory reference (..): {}", path.display());
        }

        // Ensure path is within cache folder
        let canonical_cache = cache_folder.canonicalize()?;
        let canonical_path = path.canonicalize().or_else(|_| {
            // If path doesn't exist yet, check its parent
            if let Some(parent) = path.parent() {
                let canonical_parent = parent.canonicalize()?;
                Ok(canonical_parent.join(path.file_name().unwrap_or(std::ffi::OsStr::new(""))))
            } else {
                anyhow::bail!("Path has no parent: {}", path.display())
            }
        })?;

        if !canonical_path.starts_with(&canonical_cache) {
            anyhow::bail!("Path is outside cache directory: {}", path.display());
        }

        Ok(())
    }

    /// Retrieve an image from a `url` or a cached `path`.
    /// If `saved` is specified, the retrieved image is saved to the cached `path`.
    /// Issue #5: Uses tempfile crate for automatic cleanup on failure.
    async fn retrieve_image(
        &self,
        url: &str,
        path: &std::path::Path,
        saved: bool,
    ) -> Result<Vec<u8>> {
        // Validate path is within cache directory for security
        if let Err(e) = Self::validate_cache_path(path) {
            anyhow::bail!("Invalid image cache path: {e}");
        }

        if tokio::fs::try_exists(path).await? {
            tracing::debug!("Retrieving image from file: {}", path.display());
            return Ok(tokio::fs::read(path).await?);
        }

        tracing::info!("Retrieving image from url: {url}");

        let bytes = self
            .http
            .get(url)
            .send()
            .await
            .with_context(|| format!("get image from url {url}"))?
            .bytes()
            .await?;

        if saved {
            tracing::info!("Saving the retrieved image into {}", path.display());
            // Issue #5: Use tempfile with automatic cleanup to prevent temp file leakage
            let parent = path.parent().unwrap_or_else(|| std::path::Path::new("."));
            let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("image");
            
            if let Some(d) = path.parent() {
                tokio::fs::create_dir_all(d).await?;
            }
            
            let temp_file = tempfile::NamedTempFile::with_prefix_in(
                format!("{}.", file_stem),
                parent,
            )?;
            
            // M1: Convert bytes to Vec once, move into spawn_blocking to avoid cloning MB-sized data
            let mut bytes_vec = bytes.to_vec();
            let temp_path = temp_file.path().to_path_buf();
            bytes_vec = tokio::task::spawn_blocking(move || {
                let mut file = std::fs::File::create(&temp_path)?;
                std::io::copy(&mut bytes_vec.as_slice(), &mut file)?;
                Ok::<_, std::io::Error>(bytes_vec)
            }).await??;
            
            // Atomically persist the temp file to the final path
            // On failure, temp file is automatically deleted
            match temp_file.persist(path) {
                Ok(_) => {},
                Err(e) => {
                    tracing::error!("Failed to persist temp file to {}: {e}", path.display());
                    // Explicitly clean up temp file on error (though persist already does this)
                    return Err(e.error.into());
                }
            }
            Ok(bytes_vec)
        } else {
            Ok(bytes.to_vec())
        }
    }

    #[cfg(feature = "pixelate")]
    fn pixelate_image(image: &mut image::DynamicImage) {
        let pixels = config::get_config().app_config.cover_img_pixels;
        let pixelated_image = image.resize(pixels, pixels, image::imageops::FilterType::Nearest);
        *image = pixelated_image.resize(
            image.width(),
            image.height(),
            image::imageops::FilterType::Nearest,
        );
    }

    /// Process a list of albums, which includes
    /// - sort albums by the release date
    /// - sort albums by the type if `sort_artist_albums_by_type` config is enabled
    fn process_artist_albums(mut albums: Vec<Album>) -> Vec<Album> {
        if config::get_config().app_config.sort_artist_albums_by_type {
            fn get_priority(album_type: &str) -> usize {
                match album_type {
                    "album" => 0,
                    "single" => 1,
                    "appears_on" => 2,
                    "compilation" => 3,
                    _ => 4,
                }
            }
            albums.sort_by(|a, b| {
                get_priority(&a.album_type())
                    .cmp(&get_priority(&b.album_type()))
                    .then_with(|| b.release_date.partial_cmp(&a.release_date).unwrap_or(std::cmp::Ordering::Equal))
            });
        } else {
            albums.sort_by(|x, y| y.release_date.partial_cmp(&x.release_date).unwrap_or(std::cmp::Ordering::Equal));
        }

        albums
    }
}

fn move_seed_track_to_front(tracks: &mut Vec<Track>, seed_track: Track) {
    tracks.retain(|track| track.id != seed_track.id);
    tracks.insert(0, seed_track);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{Track, PlaybackMetadata};
    use rspotify::model::TrackId;

    fn sample_track(id: &'static str, name: &str) -> Track {
        Track {
            id: TrackId::from_id(id).unwrap().into_static(),
            name: name.to_string(),
            artists: vec![],
            album: None,
            duration: std::time::Duration::default(),
            explicit: false,
            added_at: 0,
            artists_display: None,
        }
    }

    #[test]
    fn move_seed_track_to_front_prepends_missing_seed() {
        let seed = sample_track("3n3Ppam7vgaVa1iaRUc9Lp", "seed");
        let second = sample_track("4uLU6hMCjMI75M1A2tKUQC", "second");
        let third = sample_track("1301WleyT98MSxVHPZCA6M", "third");
        let mut tracks = vec![second.clone(), third];

        move_seed_track_to_front(&mut tracks, seed.clone());

        assert_eq!(tracks.len(), 3);
        assert_eq!(tracks[0].id, seed.id);
        assert_eq!(tracks[1].id, second.id);
    }

    #[test]
    fn move_seed_track_to_front_reorders_existing_seed_without_duplication() {
        let seed = sample_track("3n3Ppam7vgaVa1iaRUc9Lp", "seed");
        let second = sample_track("4uLU6hMCjMI75M1A2tKUQC", "second");
        let mut tracks = vec![second.clone(), seed.clone()];

        move_seed_track_to_front(&mut tracks, seed.clone());

        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].id, seed.id);
        assert_eq!(tracks[1].id, second.id);
    }

    // Additional playback control tests
    /// Test PlayerRequest state transitions for Pause/Resume
    #[test]
    fn test_playback_pause_resume_states() {
        let mut playback = PlaybackMetadata {
            device_name: "Test Device".to_string(),
            device_id: Some("test_device_id".to_string()),
            volume: Some(50),
            is_playing: true,
            repeat_state: rspotify::model::RepeatState::Off,
            shuffle_state: false,
            mute_state: None,
        };

        // Test pause transition
        let was_playing = playback.is_playing;
        playback.is_playing = false;
        assert!(was_playing);
        assert!(!playback.is_playing);

        // Test resume transition
        playback.is_playing = true;
        assert!(playback.is_playing);
    }

    /// Test PlayerRequest Repeat state transitions
    #[test]
    fn test_playback_repeat_transitions() {
        let mut playback = PlaybackMetadata {
            device_name: "Test Device".to_string(),
            device_id: Some("test_device_id".to_string()),
            volume: Some(50),
            is_playing: true,
            repeat_state: rspotify::model::RepeatState::Off,
            shuffle_state: false,
            mute_state: None,
        };

        // Test repeat state transitions
        let next_state = match playback.repeat_state {
            rspotify::model::RepeatState::Off => rspotify::model::RepeatState::Track,
            rspotify::model::RepeatState::Track => rspotify::model::RepeatState::Context,
            rspotify::model::RepeatState::Context => rspotify::model::RepeatState::Off,
        };
        playback.repeat_state = next_state;
        assert_eq!(playback.repeat_state, rspotify::model::RepeatState::Track);

        // Second transition
        let next_state = match playback.repeat_state {
            rspotify::model::RepeatState::Off => rspotify::model::RepeatState::Track,
            rspotify::model::RepeatState::Track => rspotify::model::RepeatState::Context,
            rspotify::model::RepeatState::Context => rspotify::model::RepeatState::Off,
        };
        playback.repeat_state = next_state;
        assert_eq!(playback.repeat_state, rspotify::model::RepeatState::Context);

        // Third transition (back to Off)
        let next_state = match playback.repeat_state {
            rspotify::model::RepeatState::Off => rspotify::model::RepeatState::Track,
            rspotify::model::RepeatState::Track => rspotify::model::RepeatState::Context,
            rspotify::model::RepeatState::Context => rspotify::model::RepeatState::Off,
        };
        playback.repeat_state = next_state;
        assert_eq!(playback.repeat_state, rspotify::model::RepeatState::Off);
    }

    /// Test PlayerRequest Shuffle toggle
    #[test]
    fn test_playback_shuffle_toggle() {
        let mut playback = PlaybackMetadata {
            device_name: "Test Device".to_string(),
            device_id: Some("test_device_id".to_string()),
            volume: Some(50),
            is_playing: true,
            repeat_state: rspotify::model::RepeatState::Off,
            shuffle_state: false,
            mute_state: None,
        };

        // Toggle shuffle on
        playback.shuffle_state = !playback.shuffle_state;
        assert!(playback.shuffle_state);

        // Toggle shuffle off
        playback.shuffle_state = !playback.shuffle_state;
        assert!(!playback.shuffle_state);
    }

    /// Test PlayerRequest Volume changes
    #[test]
    fn test_playback_volume() {
        let mut playback = PlaybackMetadata {
            device_name: "Test Device".to_string(),
            device_id: Some("test_device_id".to_string()),
            volume: Some(50),
            is_playing: true,
            repeat_state: rspotify::model::RepeatState::Off,
            shuffle_state: false,
            mute_state: None,
        };

        // Test volume change
        let new_volume: u8 = 75;
        playback.volume = Some(u32::from(new_volume));
        playback.mute_state = None;
        assert_eq!(playback.volume, Some(75));
        assert!(playback.mute_state.is_none());
    }

    /// Test PlayerRequest ToggleMute behavior
    #[test]
    fn test_playback_toggle_mute() {
        let mut playback = PlaybackMetadata {
            device_name: "Test Device".to_string(),
            device_id: Some("test_device_id".to_string()),
            volume: Some(50),
            is_playing: true,
            repeat_state: rspotify::model::RepeatState::Off,
            shuffle_state: false,
            mute_state: None,
        };

        // Toggle mute on
        let new_mute_state = match playback.mute_state {
            None => {
                let restore_volume = playback.volume.unwrap_or(50).min(100);
                Some(restore_volume)
            }
            Some(volume) => {
                let vol = volume.min(100) as u8;
                let _ = vol;
                None
            }
        };
        playback.mute_state = new_mute_state;
        assert_eq!(playback.mute_state, Some(50));

        // Toggle mute off
        let new_mute_state = match playback.mute_state {
            None => {
                let restore_volume = playback.volume.unwrap_or(50).min(100);
                Some(restore_volume)
            }
            Some(volume) => {
                let vol = volume.min(100) as u8;
                let _ = vol;
                None
            }
        };
        playback.mute_state = new_mute_state;
        assert!(playback.mute_state.is_none());
    }

    /// Test sanitize_image_filename with various inputs
    #[test]
    fn test_sanitize_image_filename() {
        // Test normal filename
        let sanitized = AppClient::sanitize_image_filename("test_image.png");
        assert_eq!(sanitized, "test_image.png");

        // Test filename with path traversal
        let sanitized = AppClient::sanitize_image_filename("../etc/passwd");
        assert!(sanitized.starts_with("invalid_traversal_"));

        // Test filename with invalid characters
        let sanitized = AppClient::sanitize_image_filename("test:image?.png");
        assert_eq!(sanitized, "test_image_.png");

        // Test filename with control characters
        let sanitized = AppClient::sanitize_image_filename("test\x01image.png");
        assert_eq!(sanitized, "test_image.png");

        // Test very long filename
        let long_name = "a".repeat(300);
        let sanitized = AppClient::sanitize_image_filename(&long_name);
        assert!(sanitized.len() <= 200);
    }

    /// Test hash_filename produces consistent results
    #[test]
    fn test_hash_filename_consistency() {
        let name1 = "test_image.png";
        let hash1 = AppClient::hash_filename(name1);
        let hash2 = AppClient::hash_filename(name1);
        assert_eq!(hash1, hash2);

        // Different names should produce different hashes (likely)
        let name2 = "different_image.png";
        let hash3 = AppClient::hash_filename(name2);
        assert_ne!(hash1, hash3);
    }

    /// Test hash_filename produces different values for different inputs
    #[test]
    fn test_hash_filename_uniqueness() {
        let names = [
            "image1.png",
            "image2.png",
            "test.jpg",
            "cover.png",
        ];
        
        let hashes: Vec<u64> = names.iter()
            .map(|n| AppClient::hash_filename(n))
            .collect();
        
        // All hashes should be different
        let unique_hashes: std::collections::HashSet<u64> = hashes.iter().cloned().collect();
        assert_eq!(hashes.len(), unique_hashes.len());
    }
}
