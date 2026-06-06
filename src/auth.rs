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
use anyhow::{Context, Result};
use librespot_core::{authentication::Credentials, cache::Cache, config::SessionConfig, Session};
use librespot_oauth::OAuthClientBuilder;

pub const SPOTIFY_CLIENT_ID: &str = "65b708073fc0480ea92a077233ca87bd";
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
            Ok(config) => config,
            Err(e) => panic!("failed to create default AuthConfig: {:#}", e),
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
                let creds = tokio::task::spawn_blocking(move || {
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
                })
                .await
                .context("blocking task panicked")??;
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
