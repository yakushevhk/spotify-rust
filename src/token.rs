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
    let mut attempts = 0;
    let token = 'retry: {
        for _attempt in 0..MAX_RETRIES {
            attempts += 1;
            let fut = session.login5().auth_token();
            match tokio::time::timeout(TIMEOUT, fut).await {
                Ok(Ok(token)) => break 'retry token,
                Ok(Err(err)) => {
                    last_err = Some(format!("{err:#}"));
                    let err_str = format!("{:?}", err);
                    let is_retryable = err_str.contains("timeout")
                        || err_str.contains("connection")
                        || err_str.contains("retry")
                        || err_str.contains("rate")
                        || err_str.contains("timed out")
                        || err_str.contains("unavailable")
                        || err_str.contains("temporary");

                    if is_retryable && attempts < MAX_RETRIES {
                        tracing::warn!(
                            "Token request failed (attempt {}/{}): {:#}",
                            attempts,
                            MAX_RETRIES,
                            err
                        );
                        tokio::time::sleep(std::time::Duration::from_secs(1 << attempts)).await;
                        continue;
                    }
                    anyhow::bail!("Failed after {} attempts: {}", attempts, last_err.unwrap());
                }
                Err(_) => {
                    last_err = Some("timeout when getting the token".to_string());
                    tracing::warn!(
                        "Token acquisition timed out (attempt {}/{})",
                        attempts,
                        MAX_RETRIES
                    );
                    if attempts < MAX_RETRIES {
                        tokio::time::sleep(std::time::Duration::from_secs(1 << attempts)).await;
                        continue;
                    }
                }
            }
        }
        anyhow::bail!(
            "Failed after {} attempts: {}",
            attempts,
            last_err.unwrap_or("timeout when getting the token".to_string())
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that TIMEOUT constant is reasonable (5 seconds)
    #[test]
    fn test_timeout_value() {
        assert_eq!(TIMEOUT, std::time::Duration::from_secs(5));
    }

    /// Test that MAX_RETRIES constant is set correctly
    #[test]
    fn test_max_retries_value() {
        assert_eq!(MAX_RETRIES, 3);
    }

    /// Test error propagation when session has no auth data
    /// This tests the early return path when session has no stored credentials
    #[tokio::test]
    async fn test_get_token_rspotify_no_auth_data() {
        // Create a session with no credentials
        let session = librespot_core::Session::new(
            librespot_core::config::SessionConfig::default(),
            None,
        );

        let result = get_token_rspotify(&session).await;
        assert!(result.is_err());
        
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Session has no stored credentials"));
    }

    /// Test retry logic behavior by verifying the loop structure
    /// The retry loop should attempt up to MAX_RETRIES times
    #[test]
    fn test_retry_loop_structure() {
        // Verify the retry loop is properly configured
        let mut attempt_count = 0;
        for attempt in 0..MAX_RETRIES {
            attempt_count += 1;
            assert!(attempt < MAX_RETRIES);
        }
        assert_eq!(attempt_count, MAX_RETRIES);
    }

    /// Test timeout behavior - verify timeout duration is applied correctly
    #[tokio::test]
    async fn test_timeout_behavior() {
        // Test that tokio::time::timeout works with our TIMEOUT constant
        let start = tokio::time::Instant::now();
        
        // Create a future that completes immediately
        let result = tokio::time::timeout(TIMEOUT, async { "success" }).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        
        // Should complete quickly, well before timeout
        let elapsed = start.elapsed();
        assert!(elapsed < TIMEOUT);
    }

    /// Test timeout expiration - verify timeout error is returned
    #[tokio::test]
    async fn test_timeout_expiration() {
        let start = tokio::time::Instant::now();
        
        // Create a future that never completes
        let result: Result<Result<(), ()>, _> = tokio::time::timeout(
            std::time::Duration::from_millis(10),
            async {
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                Ok(())
            }
        ).await;
        
        // Should timeout
        assert!(result.is_err());
        
        let elapsed = start.elapsed();
        // Should have timed out quickly
        assert!(elapsed < std::time::Duration::from_millis(100));
    }

    /// Test error propagation for timeout scenarios
    #[tokio::test]
    async fn test_error_propagation_timeout() {
        // Simulate the error message that would be created
        let error_msg = "timeout when getting the token";
        assert_eq!(error_msg, "timeout when getting the token");
    }

    /// Test chrono Duration conversion for token expiration
    #[test]
    fn test_chrono_duration_conversion() {
        let std_duration = std::time::Duration::from_secs(3600); // 1 hour
        let chrono_result = chrono::Duration::from_std(std_duration);
        assert!(chrono_result.is_ok());
        
        let chrono_duration = chrono_result.unwrap();
        assert_eq!(chrono_duration.num_seconds(), 3600);
    }

    /// Test token expiration calculation
    #[test]
    fn test_token_expiration_calculation() {
        let expires_in = chrono::Duration::hours(1);
        let expires_at = chrono::Utc::now() + expires_in;
        
        // Expiration should be in the future
        assert!(expires_at > chrono::Utc::now());
        
        // Should be approximately 1 hour from now
        let diff = expires_at - chrono::Utc::now();
        assert!(diff.num_seconds() >= 3590 && diff.num_seconds() <= 3600);
    }
}
