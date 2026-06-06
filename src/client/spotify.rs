use librespot_core::session::Session;
use maybe_async::maybe_async;
use rspotify::{
    clients::{BaseClient, OAuthClient},
    http::HttpClient,
    sync::Mutex,
    ClientError, ClientResult, Config, Credentials, OAuth, Token,
};
use std::{fmt, sync::Arc};

use crate::token;

#[derive(Clone, Default)]
/// A custom Spotify client to interact with the official Spotify API server
pub struct Spotify {
    creds: Credentials,
    oauth: OAuth,
    config: Config,
    token: Arc<Mutex<Option<Token>>>,
    http: HttpClient,
    session: Arc<tokio::sync::Mutex<Option<Session>>>,
    refresh_lock: Arc<tokio::sync::Mutex<()>>,
}

#[allow(clippy::missing_fields_in_debug)] // Seems like not all fields are necessary in debug
impl fmt::Debug for Spotify {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Spotify")
            .field("creds", &self.creds)
            .field("oauth", &self.oauth)
            .field("config", &self.config)
            .field("token", &self.token)
            .finish()
    }
}

impl Spotify {
    /// Create a new Spotify client
    pub fn new() -> Spotify {
        Self {
            creds: Credentials::default(),
            oauth: OAuth::default(),
            config: Config {
                token_refreshing: true,
                ..Default::default()
            },
            token: Arc::new(Mutex::new(None)),
            http: HttpClient::default(),
            session: Arc::new(tokio::sync::Mutex::new(None)),
            refresh_lock: Arc::new(tokio::sync::Mutex::new(())),
        }
    }

    pub async fn set_session(&self, session: Session) {
        *self.session.lock().await = Some(session);
    }

    pub async fn session(&self) -> Result<Session, ClientError> {
        let session = self.session.lock().await;
        session.clone().ok_or_else(|| {
            ClientError::Cli("No active Spotify session".to_string())
        })
    }
}

// TODO: remove the below uses of `maybe_async` crate once
// async trait is fully supported in stable Rust.

#[maybe_async]
impl BaseClient for Spotify {
    fn get_http(&self) -> &HttpClient {
        &self.http
    }

    fn get_token(&self) -> Arc<Mutex<Option<Token>>> {
        Arc::clone(&self.token)
    }

    fn get_creds(&self) -> &Credentials {
        &self.creds
    }

    fn get_config(&self) -> &Config {
        &self.config
    }

    async fn refetch_token(&self) -> ClientResult<Option<Token>> {
        let _guard = self.refresh_lock.lock().await;

        // Check if another task already refreshed the token while we were waiting
        {
            let existing = self.token.lock().await
                .map_err(|e| ClientError::Cli(format!("Token mutex poisoned: {:?}", e)))?;
            if let Some(ref tok) = *existing {
                if let Some(expires_at) = tok.expires_at {
                    if chrono::Utc::now() < expires_at {
                        return Ok(Some(tok.clone()));
                    }
                }
            }
        }

        let session = self.session.lock().await.clone();
        let old_token = self.token.lock().await
            .map_err(|e| ClientError::Cli(format!("Token mutex poisoned: {:?}", e)))?
            .clone();

        let Some(session) = session else {
            tracing::error!("No session available for token refresh");
            return Err(ClientError::Cli(
                "No session available for token refresh".to_string(),
            ));
        };

        if session.is_invalid() {
            tracing::error!("Session is invalid, cannot refresh token");
            return Err(ClientError::Cli(
                "Session is invalid, cannot refresh token".to_string(),
            ));
        }

        match token::get_token_rspotify(&session).await {
            Ok(token) => Ok(Some(token)),
            Err(err) => {
                let msg = format!("{err:#}");
                if msg.contains("timeout") {
                    tracing::warn!("Token refresh timed out, keeping existing token");
                    Ok(old_token)
                } else if msg.contains("400") || msg.contains("Bad Request") {
                    let error_msg = "HTTP 400 Bad Request during token refresh. Delete ~/.cache/spotify-player/user_client_token.json and retry.".to_string();
                    tracing::error!("{}", error_msg);
                    Err(ClientError::Cli(error_msg))
                } else if msg.contains("401") || msg.contains("Unauthorized") {
                    let error_msg = "HTTP 401 Unauthorized. Delete ~/.cache/spotify-player/credentials.json and user_client_token.json, then re-authenticate.".to_string();
                    tracing::error!("{}", error_msg);
                    Err(ClientError::Cli(error_msg))
                } else if msg.contains("500") || msg.contains("Internal Server Error") {
                    let error_msg = "HTTP 500 Internal Server Error. Spotify servers may be experiencing issues. Please retry in a few seconds.".to_string();
                    tracing::warn!("{}", error_msg);
                    Err(ClientError::Cli(error_msg))
                } else {
                    tracing::error!("Token refresh failed: {err:#}");
                    Err(ClientError::Cli(msg))
                }
            }
        }
    }
}

/// Implement `OAuthClient` trait for `Spotify` struct
/// to allow calling methods that get/modify user's data such as
/// `current_user_playlists`, `playlist_add_items`, etc.
///
/// Because the `Spotify` client interacts with Spotify APIs
/// using an access token that is manually retrieved by
/// the `librespot::get_token` function, implementing
/// `OAuthClient::get_oauth` and `OAuthClient::request_token` is unnecessary
#[maybe_async]
impl OAuthClient for Spotify {
    fn get_oauth(&self) -> &OAuth {
        &self.oauth
    }

    async fn request_token(&self, _code: &str) -> ClientResult<()> {
        Ok(())
    }
}
