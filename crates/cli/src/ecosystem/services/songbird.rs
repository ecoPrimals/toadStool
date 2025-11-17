//! Songbird integration - Service discovery and coordination
//!
//! This module handles all Songbird-specific operations including:
//! - Service registration with Songbird orchestrator
//! - Capability advertisement
//! - Health checks and heartbeats

use std::net::SocketAddr;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::time::timeout;

use super::super::types::*;

/// Send registration request to Songbird orchestrator
pub async fn send_registration(
    addr: &SocketAddr,
    registration: &SongbirdRegistration,
) -> Result<SongbirdResponse> {
    // Send HTTP POST request to Songbird registration endpoint
    match timeout(
        Duration::from_secs(10),
        reqwest::Client::new()
            .post(format!("http://{addr}/api/v1/register"))
            .json(registration)
            .send(),
    )
    .await
    {
        Ok(Ok(response)) => {
            if response.status().is_success() {
                let songbird_response: SongbirdResponse = response
                    .json()
                    .await
                    .with_context(|| "Failed to parse Songbird registration response")?;
                Ok(songbird_response)
            } else {
                anyhow::bail!(
                    "Songbird registration failed with status: {}",
                    response.status()
                )
            }
        }
        Ok(Err(e)) => anyhow::bail!("Failed to send registration request: {e}"),
        Err(_) => anyhow::bail!("Registration request timeout"),
    }
}

// Removed verify_songbird_service() and verify_songbird_capabilities() - unused.
// These were complete implementations but never called. If service verification
// becomes needed, the logic exists in git history and can be restored.
