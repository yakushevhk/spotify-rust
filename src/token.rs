use std::collections::HashSet;

use anyhow::Result;
use librespot_core::session::Session;

const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
const MAX_RETRIES: u32 = 3;

pub async fn get_token_rspotify(session: &Session) -> Result<rspotify::Token> {
    tracing::info!("Getting a new authentication token...");

    let auth_data = session.auth_data();
    if auth_data.is_empty() {
        anyhow::bail!("Session has no stored credentials for login5 token acquisition");
    }

    let mut last_err = None;
    let token = 'retry: {
        for attempt in 0..MAX_RETRIES {
            let fut = session.login5().auth_token();
            match tokio::time::timeout(TIMEOUT, fut).await {
                Ok(Ok(token)) => break 'retry token,
                Ok(Err(err)) => {
                    anyhow::bail!("failed to get the token: {err:?}");
                }
                Err(_) => {
                    tracing::warn!(
                        "Token acquisition timed out (attempt {}/{})",
                        attempt + 1,
                        MAX_RETRIES
                    );
                    last_err = Some("timeout when getting the token");
                    if attempt + 1 < MAX_RETRIES {
                        continue;
                    }
                }
            }
        }
        anyhow::bail!(
            "{}",
            last_err.unwrap_or("timeout when getting the token")
        );
    };

    // converts the token returned by librespot `get_token` function to a `rspotify::Token`

    let expires_in = chrono::Duration::from_std(token.expires_in)?;
    // let expires_in = Duration::from_std(std::time::Duration::from_secs(5))?;
    let expires_at = chrono::Utc::now() + expires_in;

    let token = rspotify::Token {
        access_token: token.access_token,
        expires_in,
        expires_at: Some(expires_at),
        scopes: HashSet::new(),
        refresh_token: None,
    };

    tracing::info!("Got new token: {token:?}");

    Ok(token)
}
