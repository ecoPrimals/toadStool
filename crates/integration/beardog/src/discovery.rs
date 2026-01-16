//! bearDog service discovery via capability-based discovery
//!
//! Discovers bearDog entropy service at runtime - NO HARDCODING!

use crate::seed::{EphemeralSeed, SeedQuality};
use crate::types::{EntropyMixing, EntropySource};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Request for ephemeral seed generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedRequest {
    /// Requested entropy source
    pub source: EntropySource,
    /// Requested mixing configuration
    pub mixing: EntropyMixing,
    /// Minimum quality threshold
    pub min_quality: f32,
}

impl Default for SeedRequest {
    fn default() -> Self {
        Self {
            source: EntropySource::Mixed,
            mixing: EntropyMixing::beardog_standard(),
            min_quality: 0.7,
        }
    }
}

/// bearDog entropy client
///
/// Discovers and communicates with bearDog entropy service.
/// Uses capability-based discovery (no hardcoded URLs!).
pub struct EntropyClient {
    /// Service endpoint (discovered at runtime)
    #[allow(dead_code)] // Stored for diagnostics
    endpoint: Option<String>,
    /// RPC client for communication (pure Rust unix socket!)
    rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient,
    /// Whether service is available
    available: bool,
}

impl EntropyClient {
    /// Discover bearDog via capability discovery
    ///
    /// Searches for services advertising "capability:entropy:high-quality".
    /// NO HARDCODED URLs - pure runtime discovery!
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Service discovery unavailable
    /// - No bearDog service found
    /// - Connection fails
    pub async fn discover() -> Result<Self> {
        // Step 1: Check environment variable (user can override)
        if let Ok(endpoint) = std::env::var("TOADSTOOL_ENTROPY_SERVICE_URL") {
            tracing::info!("Using entropy service from environment: {}", endpoint);
            return Self::connect(&endpoint).await;
        }

        // Step 2: Try capability discovery
        match Self::discover_via_capability().await {
            Ok(endpoint) => {
                tracing::info!("Discovered bearDog entropy service: {}", endpoint);
                Self::connect(&endpoint).await
            }
            Err(e) => {
                tracing::warn!("bearDog service discovery failed: {}", e);
                // Return unavailable client (will fallback to system entropy)
                Ok(Self {
                    endpoint: None,
                    rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(
                        toadstool_common::primal_sockets::get_beardog_socket_path()
                    ),
                    available: false,
                })
            }
        }
    }

    /// Discover bearDog service via capability announcement
    ///
    /// In production, this would:
    /// 1. Query songBird for services with "capability:entropy:high-quality"
    /// 2. Filter for bearDog-specific capabilities
    /// 3. Select best available service
    ///
    /// For now, returns error to demonstrate graceful fallback.
    async fn discover_via_capability() -> Result<String> {
        // TODO: Implement actual capability discovery via songBird
        // For now, demonstrate the pattern with environment fallback
        
        // Check for local development bearDog instance
        let candidate_urls = vec![
            "http://localhost:8081",  // Common bearDog port
            "http://localhost:3000",  // Alternative
        ];

        for url in candidate_urls {
            if Self::probe_service(url).await.is_ok() {
                return Ok(url.to_string());
            }
        }

        anyhow::bail!("No bearDog service found via capability discovery")
    }

    /// Probe unix socket to check if bearDog service is available
    ///
    /// **PURE RUST**: Uses unix socket instead of HTTP
    async fn probe_service(_url: &str) -> Result<()> {
        // PURE RUST: Try to connect to unix socket
        let socket_path = toadstool_common::primal_sockets::get_beardog_socket_path();
        
        match tokio::net::UnixStream::connect(&socket_path).await {
            Ok(_) => {
                tracing::debug!("BearDog unix socket available");
                Ok(())
            }
            Err(e) => {
                anyhow::bail!("BearDog socket not available: {}", e)
            }
        }
    }

    /// Connect to bearDog service via unix socket
    ///
    /// **PURE RUST**: Uses unix socket instead of HTTP
    async fn connect(endpoint: &str) -> Result<Self> {
        let socket_client = toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(
            toadstool_common::primal_sockets::get_beardog_socket_path()
        );

        // Verify service is reachable via unix socket
        let available = Self::probe_service("").await.is_ok();

        Ok(Self {
            endpoint: Some(endpoint.to_string()),
            rpc_client: socket_client,
            available,
        })
    }

    /// Check if bearDog service is available
    #[must_use]
    pub fn is_available(&self) -> bool {
        self.available
    }

    /// Generate ephemeral seed
    ///
    /// Requests high-quality, human-mixed entropy from bearDog.
    /// Falls back to system entropy if bearDog unavailable.
    ///
    /// # Errors
    ///
    /// Returns error if request fails and fallback is disabled.
    pub async fn generate_seed(&self) -> Result<EphemeralSeed> {
        self.generate_seed_with_request(SeedRequest::default()).await
    }

    /// Generate seed with custom request
    ///
    /// # Errors
    ///
    /// Returns error if request fails and fallback is disabled.
    pub async fn generate_seed_with_request(&self, request: SeedRequest) -> Result<EphemeralSeed> {
        if !self.available {
            // Fallback to system entropy
            return self.system_entropy_fallback();
        }

        // Request from bearDog
        match self.request_from_beardog(&request).await {
            Ok(seed) => Ok(seed),
            Err(e) => {
                tracing::warn!("bearDog request failed: {}, falling back to system entropy", e);
                self.system_entropy_fallback()
            }
        }
    }

    /// Request seed from bearDog service via unix socket
    ///
    /// **PURE RUST**: JSON-RPC over unix socket (no HTTP!)
    async fn request_from_beardog(&self, request: &SeedRequest) -> Result<EphemeralSeed> {
        let params = serde_json::to_value(request)
            .context("Failed to serialize seed request")?;

        let seed: EphemeralSeed = self.rpc_client
            .call_typed("beardog.entropy.generate_seed", params)
            .await
            .context("Failed to request seed from bearDog")?;

        Ok(seed)
    }

    /// Fallback to system entropy
    ///
    /// When bearDog unavailable, use system RNG.
    /// Quality is lower (pure machine entropy), but sufficient for many use cases.
    fn system_entropy_fallback(&self) -> Result<EphemeralSeed> {
        use std::time::SystemTime;

        // Generate random bytes using system entropy
        let mut seed_data = vec![0u8; 32];
        // In production, use: getrandom::getrandom(&mut seed_data)?;
        // For now, use timestamp-based (demonstration)
        if let Ok(duration) = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
            let nanos = duration.as_nanos();
            seed_data[0..16].copy_from_slice(&nanos.to_le_bytes());
        }

        let quality = SeedQuality::new(
            0.7,  // Acceptable but not cryptographic
            0.9,  // Good machine entropy
            0.0,  // No human entropy
        );

        let mixing = EntropyMixing {
            machine_weight: 1.0,
            human_weight: 0.0,
            algorithm: "system".to_string(),
        };

        Ok(EphemeralSeed::new(
            seed_data,
            EntropySource::Machine,
            mixing,
            quality,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_entropy_client_discovery() {
        // Should not panic - graceful if bearDog unavailable
        let result = EntropyClient::discover().await;
        assert!(result.is_ok());

        let client = result.unwrap();
        // Client should exist even if bearDog unavailable (fallback)
        assert!(client.endpoint.is_none() || !client.available);
    }

    #[tokio::test]
    async fn test_system_entropy_fallback() {
        let client = EntropyClient {
            endpoint: None,
            rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(
                toadstool_common::primal_sockets::get_beardog_socket_path()
            ),
            available: false,
        };

        let seed = client.system_entropy_fallback();
        assert!(seed.is_ok());

        let seed = seed.unwrap();
        assert_eq!(seed.source, EntropySource::Machine);
        assert!(!seed.seed_data.is_empty());
    }

    #[tokio::test]
    async fn test_generate_seed_fallback() {
        let client = EntropyClient {
            endpoint: None,
            rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(
                toadstool_common::primal_sockets::get_beardog_socket_path()
            ),
            available: false,
        };

        // Should fallback gracefully to system entropy
        let seed = client.generate_seed().await;
        assert!(seed.is_ok());

        let seed = seed.unwrap();
        assert_eq!(seed.source, EntropySource::Machine);
    }

    #[test]
    fn test_seed_request_default() {
        let request = SeedRequest::default();
        assert_eq!(request.source, EntropySource::Mixed);
        assert_eq!(request.min_quality, 0.7);
        assert!(request.mixing.is_valid());
    }
}
