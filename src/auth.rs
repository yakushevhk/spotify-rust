//! Authentication module
//!
//! This module handles OAuth authentication with Spotify using the PKCE flow.
//! It supports both cached credentials and interactive browser-based authentication.
//!
//! # Authentication Flow
//!
//! 1. Check for cached credentials in `~/.cache/spotify-player/credentials.json`
//! 2. If found and valid, use them
//! 3. If not found or invalid, open browser for OAuth flow
//! 4. User authenticates with Spotify
//! 5. Access token is cached for future use
//!
//! # Security
//!
//! - Credentials are stored with restrictive permissions (0o600)
//! - Cache directory has 0o700 permissions
//! - Uses PKCE for secure authentication without client secret
//!
//! # OAuth Scopes
//!
//! The application requests the following scopes:
//! - `user-read-playback-state` - Read playback state
//! - `user-modify-playback-state` - Control playback
//! - `streaming` - Stream audio
//! - `playlist-read-private` - Read private playlists
//! - `playlist-modify-private` - Modify private playlists
//! - `playlist-modify-public` - Modify public playlists
//! - `user-follow-read` - Read followed artists
//! - `user-follow-modify` - Follow/unfollow artists
//! - `user-library-read` - Read saved tracks/albums
//! - `user-library-modify` - Save/remove tracks
//! - `user-top-read` - Read top tracks/artists
//! - `user-read-recently-played` - Read recently played
//!
//! # Example
//!
//! ```rust
//! let auth_config = AuthConfig::new(&configs)?;
//! let creds = get_creds(&auth_config, true, true).await?;
//! ```

use crate::config;
use std::path::Path;
use anyhow::{Context, Result};
use librespot_core::{authentication::Credentials, cache::Cache, config::SessionConfig, Session};
use librespot_oauth::OAuthClientBuilder;

/// Default Spotify client ID for standard playback.
/// Users can override this in their config file via `client_id` field.
pub const SPOTIFY_CLIENT_ID: &str = "65b708073fc0480ea92a077233ca87bd";
/// Alternative client ID used by ncspot.
/// Users can override this in their config file via `client_id` field.
pub const NCSPOT_CLIENT_ID: &str = "d420a117a32841c2b3474932e49fb54b";
// based on https://developer.spotify.com/documentation/web-api/concepts/scopes#list-of-scopes
pub const OAUTH_SCOPES: &[&str] = &[
    // Spotify Connect
    "user-read-playback-state",
    "user-modify-playback-state",
    "user-read-currently-playing",
    // Playback
    "app-remote-control",
    "streaming",
    // Playlists
    "playlist-read-private",
    "playlist-read-collaborative",
    "playlist-modify-private",
    "playlist-modify-public",
    // Follow
    "user-follow-modify",
    "user-follow-read",
    // Listening History
    "user-read-playback-position",
    "user-top-read",
    "user-read-recently-played",
    // Library
    "user-library-modify",
    "user-library-read",
    // Users
    "user-personalized",
];

#[derive(Clone)]
pub struct AuthConfig {
    pub cache: Cache,
    pub session_config: SessionConfig,
    pub login_redirect_uri: String,
}

impl Default for AuthConfig {
    fn default() -> Self {
        match AuthConfig::try_default() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("AuthConfig::try_default failed: {e}. Using minimal fallback.");
                AuthConfig {
                    cache: Cache::new(None::<String>, None, None, None)
                        .expect("Cache::new with None paths must succeed"),
                    session_config: SessionConfig::default(),
                    login_redirect_uri: "http://127.0.0.1:8989/login".to_string(),
                }
            }
        }
    }
}

impl AuthConfig {
    pub fn try_default() -> anyhow::Result<AuthConfig> {
        Ok(AuthConfig {
            cache: Cache::new(None::<String>, None, None, None)?,
            session_config: SessionConfig::default(),
            login_redirect_uri: "http://127.0.0.1:8989/login".to_string(),
        })
    }
}

impl AuthConfig {
    /// Create a `librespot::Session` from authentication configs
    pub fn session(&self) -> Session {
        Session::new(self.session_config.clone(), Some(self.cache.clone()))
    }

    pub fn new(configs: &config::Configs) -> Result<AuthConfig> {
        let audio_cache_folder = if configs.app_config.device.audio_cache {
            Some(configs.cache_folder.join("audio"))
        } else {
            None
        };

        let cache = Cache::new(
            Some(configs.cache_folder.clone()),
            None,
            audio_cache_folder,
            None,
        )?;

        // Set restrictive permissions on Unix to prevent other users from reading
        // cached credentials and access tokens.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&configs.cache_folder, std::fs::Permissions::from_mode(0o700));
            let credentials_file = configs.cache_folder.join("credentials.json");
            if credentials_file.exists() {
                let _ = std::fs::set_permissions(&credentials_file, std::fs::Permissions::from_mode(0o600));
            }
        }

        Ok(AuthConfig {
            cache,
            session_config: configs.app_config.session_config(),
            login_redirect_uri: configs.app_config.login_redirect_uri.clone(),
        })
    }
}

/// Get Spotify credentials to authenticate the application
///
/// # Args
/// - `auth_config`: authentication configuration
/// - `reauth`: whether to re-authenticate the application if no cached credentials are found
// - `use_cached`: whether to use cached credentials if available
pub async fn get_creds(auth_config: &AuthConfig, reauth: bool, use_cached: bool) -> Result<Credentials> {
    let creds = if use_cached {
        auth_config.cache.credentials()
    } else {
        None
    };

    match creds {
        None => {
            let msg = "No cached credentials found, please authenticate the application first.";
            if reauth {
                eprintln!("{msg}");

                let login_redirect_uri = auth_config.login_redirect_uri.clone();
                let mut handle = tokio::task::spawn_blocking(move || {
                    // NOTE: The underlying HTTP call (get_access_token) cannot be cancelled.
                    // On timeout, we abort the tokio task handle but the blocking HTTP thread
                    // will stay alive until the HTTP layer times out internally. This is a known
                    // limitation of spawn_blocking with non-cancellable blocking I/O.
                    let client_builder = OAuthClientBuilder::new(
                        SPOTIFY_CLIENT_ID,
                        &login_redirect_uri,
                        OAUTH_SCOPES.to_vec(),
                    )
                    .open_in_browser();
                    let oauth_client = client_builder.build()?;
                    oauth_client
                        .get_access_token()
                        .map(|t| Credentials::with_access_token(t.access_token))
                });
                let creds = tokio::select! {
                    result = &mut handle => {
                        result.context("blocking task panicked")??
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_secs(300)) => {
                        handle.abort();
                        return Err(anyhow::anyhow!("OAuth login timed out. Please try again."));
                    }
                };
                Ok(creds)
            } else {
                anyhow::bail!(msg);
            }
        }
        Some(creds) => {
            tracing::info!("Using cached credentials");
            Ok(creds)
        }
    }
}

pub fn check_user_token_expired(cache_folder: &Path) -> bool {
    let token_file = cache_folder.join("user_client_token.json");
    if !token_file.exists() {
        tracing::debug!("No user_client_token.json found");
        return true;
    }

    match std::fs::read_to_string(&token_file) {
        Ok(content) => {
            if let Ok(token) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(expires_at) = token.get("expires_at").and_then(|v| v.as_str()) {
                    if let Ok(expires_at) = chrono::DateTime::parse_from_rfc3339(expires_at) {
                        let now = chrono::Utc::now();
                        let expires_at_utc = expires_at.with_timezone(&chrono::Utc);
                        let is_expired = now >= expires_at_utc - chrono::Duration::seconds(60);
                        tracing::debug!(
                            "Token expires at {}, now: {}, expired: {}",
                            expires_at_utc, now, is_expired
                        );
                        return is_expired;
                    }
                }
            }
            tracing::warn!("Could not parse token expiration, treating as expired");
            true
        }
        Err(e) => {
            tracing::warn!("Failed to read token file: {:#}, treating as expired", e);
            true
        }
    }
}

pub fn clear_expired_tokens(cache_folder: &Path) -> Result<()> {
    let token_file = cache_folder.join("user_client_token.json");
    if token_file.exists() {
        tracing::info!("Clearing expired user_client_token.json");
        std::fs::remove_file(&token_file)
            .context("failed to delete expired user_client_token.json")?;
    }
    Ok(())
}

#[allow(dead_code)]
pub fn check_credentials_expired(cache_folder: &Path) -> bool {
    let creds_file = cache_folder.join("credentials.json");
    if !creds_file.exists() {
        tracing::debug!("No credentials.json found");
        return true;
    }

    match std::fs::read_to_string(&creds_file) {
        Ok(content) => {
            if let Ok(creds) = serde_json::from_str::<serde_json::Value>(&content) {
                // Check if credentials have an expires_at or timestamp field
                if let Some(expires_at) = creds.get("expires_at").and_then(|v| v.as_i64()) {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);
                    let expired = now >= expires_at - 60;
                    tracing::debug!(
                        "Credentials expire at {}, now: {}, expired: {}",
                        expires_at, now, expired
                    );
                    return expired;
                }
                // Credentials file is valid JSON with data but no expiration field
                // OAuth access tokens are long-lived and refreshed on use
                return false;
            }
            tracing::debug!("Could not parse credentials file, treating as expired");
            true
        }
        Err(e) => {
            tracing::warn!("Failed to read credentials file: {:#}, treating as expired", e);
            true
        }
    }
}

#[allow(dead_code)]
pub fn clear_credentials(cache_folder: &Path) -> Result<()> {
    let creds_file = cache_folder.join("credentials.json");
    if creds_file.exists() {
        tracing::info!("Clearing credentials.json");
        std::fs::remove_file(&creds_file)
            .context("failed to delete credentials.json")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that AuthConfig::default() creates a valid configuration
    #[test]
    fn test_auth_config_default() {
        let config = AuthConfig::default();
        assert_eq!(config.login_redirect_uri, "http://127.0.0.1:8989/login");
        // Cache should be initialized
        assert!(config.cache.credentials().is_none());
    }

    /// Test that session() returns a valid Session
    /// Note: This test requires a tokio runtime for Session::new
    #[tokio::test]
    async fn test_auth_config_session() {
        let config = AuthConfig::default();
        let session = config.session();
        // Session should be created without panicking
        assert!(!session.is_invalid());
    }

    /// Test OAUTH_SCOPES contains expected scopes
    #[test]
    fn test_oauth_scopes() {
        assert!(OAUTH_SCOPES.contains(&"user-read-playback-state"));
        assert!(OAUTH_SCOPES.contains(&"user-modify-playback-state"));
        assert!(OAUTH_SCOPES.contains(&"streaming"));
        assert!(OAUTH_SCOPES.contains(&"playlist-read-private"));
        assert!(!OAUTH_SCOPES.is_empty());
    }

    /// Test client IDs are defined
    #[test]
    fn test_client_ids() {
        assert!(!SPOTIFY_CLIENT_ID.is_empty());
        assert!(!NCSPOT_CLIENT_ID.is_empty());
        // They should be different
        assert_ne!(SPOTIFY_CLIENT_ID, NCSPOT_CLIENT_ID);
    }

    /// Test credential caching behavior - no credentials initially
    #[test]
    fn test_credential_caching_no_credentials() {
        let config = AuthConfig::default();
        // Should return None when no credentials are cached
        let creds = config.cache.credentials();
        assert!(creds.is_none());
    }

    /// Test error handling for reauth=false with no cached credentials
    /// This test verifies the error path when credentials are not available
    #[tokio::test]
    async fn test_get_creds_error_no_reauth() {
        let config = AuthConfig::default();
        
        // With use_cached=false and reauth=false, should fail with no credentials
        let result = get_creds(&config, false, false).await;
        assert!(result.is_err());
        
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("No cached credentials found"));
    }

    /// Test that get_creds respects the use_cached parameter
    #[tokio::test]
    async fn test_get_creds_use_cached_parameter() {
        let config = AuthConfig::default();
        
        // With use_cached=false, should behave as if no credentials exist
        let result = get_creds(&config, false, false).await;
        assert!(result.is_err());
    }

    /// Test AuthConfig implements Clone
    #[test]
    fn test_auth_config_clone() {
        let config = AuthConfig::default();
        let cloned = config.clone();
        assert_eq!(config.login_redirect_uri, cloned.login_redirect_uri);
    }
}
