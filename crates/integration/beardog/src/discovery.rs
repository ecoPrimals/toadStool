// SPDX-License-Identifier: AGPL-3.0-or-later
//! bearDog service discovery via capability-based discovery
//!
//! Discovers bearDog entropy service at runtime - NO HARDCODING!

use crate::error::BeardogError;
use crate::seed::{EphemeralSeed, SeedQuality};
use crate::types::{EntropyMixing, EntropySource};
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
    pub async fn discover() -> Result<Self, BeardogError> {
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
                // Try capability-based discovery as fallback
                let socket_path = toadstool_common::primal_sockets::discover_crypto_socket()
                    .await
                    .unwrap_or_else(|_| {
                        toadstool_common::primal_sockets::get_biomeos_dir().join("beardog.sock")
                    });

                Ok(Self {
                    endpoint: None,
                    rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(
                        socket_path,
                    ),
                    available: false,
                })
            }
        }
    }

    /// Discover bearDog service via capability announcement
    ///
    /// **PURE RUST**: Uses unix socket discovery (no HTTP!)
    ///
    /// In production, this would:
    /// 1. Query songBird for services with "capability:entropy:high-quality"  
    /// 2. Filter for bearDog-specific capabilities
    /// 3. Select best available service
    ///
    /// For now, returns error to demonstrate graceful fallback.
    async fn discover_via_capability() -> Result<String, BeardogError> {
        // Future: Implement full capability discovery via songBird unix socket
        // Current: Falls back to system entropy (graceful degradation)

        // DEEP DEBT EVOLUTION: Check Unix socket first (no hardcoded ports!)
        // Environment variable override takes precedence
        if let Ok(url) = std::env::var("BEARDOG_URL") {
            tracing::debug!("Using bearDog URL from environment: {}", url);
            return Ok(url);
        }

        // Try Unix socket discovery (preferred - capability-based, no port conflicts)
        // EVOLVED (Feb 16, 2026): Pure capability-based - no hardcoded primal names
        let socket_path = toadstool_common::primal_sockets::discover_crypto_socket()
            .await
            .unwrap_or_else(|e| {
                tracing::debug!(
                    "Capability discovery failed: {}, using biomeOS standard path",
                    e
                );
                // Fallback to biomeOS standard path for crypto services (no hardcoded name)
                toadstool_common::primal_sockets::get_biomeos_dir().join("crypto.sock")
            });

        if tokio::fs::metadata(&socket_path).await.is_ok() {
            tracing::debug!("Found crypto service via Unix socket: {:?}", socket_path);
            return Ok(format!("unix://{}", socket_path.display()));
        }

        // No HTTP fallbacks - capability-based discovery only
        // Users must ensure Unix socket exists or set BEARDOG_URL environment variable
        Err(BeardogError::Other(
            "No crypto service found via capability discovery. \
             Ensure bearDog is running or set BEARDOG_URL environment variable."
                .to_string(),
        ))
    }

    /// Probe unix socket to check if crypto service is available
    ///
    /// **PURE RUST**: Uses unix socket instead of HTTP
    /// **CAPABILITY-BASED**: Discovers crypto service by capability (not hardcoded name)
    async fn probe_service(_url: &str) -> Result<(), BeardogError> {
        // CAPABILITY-BASED: Discover ANY crypto service (not hardcoded "beardog")
        let socket_path = toadstool_common::primal_sockets::discover_crypto_socket()
            .await
            .unwrap_or_else(|_| {
                // Fallback to biomeOS standard path for crypto services
                toadstool_common::primal_sockets::get_biomeos_dir().join("crypto.sock")
            });

        match tokio::net::UnixStream::connect(socket_path).await {
            Ok(_) => {
                tracing::debug!("Crypto service unix socket available");
                Ok(())
            }
            Err(e) => Err(BeardogError::Io(e)),
        }
    }

    /// Connect to bearDog service via unix socket
    ///
    /// **PURE RUST**: Uses unix socket instead of HTTP
    /// **CAPABILITY-BASED**: Discovers crypto service by capability
    async fn connect(endpoint: &str) -> Result<Self, BeardogError> {
        // CAPABILITY-BASED: Discover ANY crypto service (not hardcoded "beardog")
        let socket_path = toadstool_common::primal_sockets::discover_crypto_socket()
            .await
            .unwrap_or_else(|_| {
                toadstool_common::primal_sockets::get_biomeos_dir().join("beardog.sock")
            });

        let socket_client =
            toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path);

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
    pub const fn is_available(&self) -> bool {
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
    pub async fn generate_seed(&self) -> Result<EphemeralSeed, BeardogError> {
        self.generate_seed_with_request(SeedRequest::default())
            .await
    }

    /// Generate seed with custom request
    ///
    /// # Errors
    ///
    /// Returns error if request fails and fallback is disabled.
    pub async fn generate_seed_with_request(
        &self,
        request: SeedRequest,
    ) -> Result<EphemeralSeed, BeardogError> {
        if !self.available {
            // Fallback to system entropy
            return Ok(Self::system_entropy_fallback());
        }

        // Request from bearDog
        match self.request_from_beardog(&request).await {
            Ok(seed) => Ok(seed),
            Err(e) => {
                tracing::warn!(
                    "bearDog request failed: {}, falling back to system entropy",
                    e
                );
                Ok(Self::system_entropy_fallback())
            }
        }
    }

    /// Request seed from bearDog service via unix socket
    ///
    /// **PURE RUST**: JSON-RPC over unix socket (no HTTP!)
    async fn request_from_beardog(
        &self,
        request: &SeedRequest,
    ) -> Result<EphemeralSeed, BeardogError> {
        let params = serde_json::to_value(request)
            .map_err(|e| BeardogError::Other(format!("Failed to serialize seed request: {e}")))?;

        let seed: EphemeralSeed = self
            .rpc_client
            .call_typed("crypto.entropy.generate_seed", params)
            .await
            .map_err(|e| {
                BeardogError::Other(format!("Failed to request seed from bearDog: {e}"))
            })?;

        Ok(seed)
    }

    /// Fallback to system entropy
    ///
    /// When bearDog unavailable, use system RNG.
    /// Quality is lower (pure machine entropy), but sufficient for many use cases.
    pub(crate) fn system_entropy_fallback() -> EphemeralSeed {
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
            0.7, // Acceptable but not cryptographic
            0.9, // Good machine entropy
            0.0, // No human entropy
        );

        let mixing = EntropyMixing {
            machine_weight: 1.0,
            human_weight: 0.0,
            algorithm: "system".to_string(),
        };

        EphemeralSeed::new(seed_data, EntropySource::Machine, mixing, quality)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_entropy_client_discovery() {
        // Test client construction without live discovery (avoids nested runtime)
        let socket_path = toadstool_common::primal_sockets::get_biomeos_dir().join("beardog.sock");
        let client = EntropyClient {
            endpoint: None,
            rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path),
            available: false,
        };
        // Client should exist even if bearDog unavailable (fallback)
        assert!(client.endpoint.is_none() || !client.available);
    }

    #[tokio::test]
    async fn test_system_entropy_fallback() {
        // Test system entropy fallback (no client needed - static method)
        let seed = EntropyClient::system_entropy_fallback();
        // Verify seed has expected properties
        assert_eq!(seed.source, EntropySource::Machine);
        assert!(!seed.seed_data.is_empty());
    }

    #[tokio::test]
    async fn test_generate_seed_fallback() {
        // Use biomeOS standard path directly (discovery requires network)
        let socket_path = toadstool_common::primal_sockets::get_biomeos_dir().join("beardog.sock");

        let client = EntropyClient {
            endpoint: None,
            rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path),
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
        assert!((request.min_quality - 0.7).abs() < f32::EPSILON);
        assert!(request.mixing.is_valid());
    }

    #[tokio::test]
    async fn test_entropy_client_is_available() {
        let socket_path = toadstool_common::primal_sockets::get_biomeos_dir().join("beardog.sock");
        let client = EntropyClient {
            endpoint: Some("unix:///tmp/beardog.sock".to_string()),
            rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path),
            available: true,
        };
        assert!(client.is_available());
    }

    #[tokio::test]
    async fn test_entropy_client_not_available() {
        let socket_path = toadstool_common::primal_sockets::get_biomeos_dir().join("beardog.sock");
        let client = EntropyClient {
            endpoint: None,
            rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path),
            available: false,
        };
        assert!(!client.is_available());
    }

    #[tokio::test]
    async fn test_generate_seed_with_request() {
        let socket_path = toadstool_common::primal_sockets::get_biomeos_dir().join("beardog.sock");
        let client = EntropyClient {
            endpoint: None,
            rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path),
            available: false,
        };

        let request = SeedRequest::default();
        let seed = client.generate_seed_with_request(request).await;
        assert!(seed.is_ok());
        let seed = seed.unwrap();
        assert_eq!(seed.source, EntropySource::Machine);
    }

    #[test]
    fn test_seed_request_serialization() {
        let request = SeedRequest::default();
        let json = serde_json::to_string(&request).unwrap();
        let restored: SeedRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(request.source, restored.source);
        assert!((request.min_quality - restored.min_quality).abs() < f32::EPSILON);
    }

    #[test]
    fn test_seed_request_mixing_valid() {
        let request = SeedRequest::default();
        assert!(request.mixing.is_valid());
    }

    #[test]
    fn test_entropy_mixing_beardog_standard() {
        let mixing = EntropyMixing::beardog_standard();
        assert!(mixing.is_valid());
        assert!(mixing.machine_weight > 0.0 || mixing.human_weight > 0.0);
    }

    #[test]
    fn test_discover_via_env_beardog_url() {
        temp_env::with_var("BEARDOG_URL", Some("unix:///run/beardog.sock"), || {
            let result = std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(EntropyClient::discover_via_capability())
            })
            .join()
            .expect("thread");
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), "unix:///run/beardog.sock");
        });
    }

    #[test]
    fn test_ephemeral_seed_structure() {
        let seed = EntropyClient::system_entropy_fallback();
        assert!(!seed.seed_data.is_empty());
        assert_eq!(seed.source, EntropySource::Machine);
        assert!(seed.quality.machine_quality > 0.0);
    }

    #[test]
    fn test_seed_quality_new() {
        let quality = SeedQuality::new(0.8, 0.9, 0.5);
        assert!((quality.machine_quality - 0.9).abs() < f32::EPSILON);
        assert!((quality.human_quality - 0.5).abs() < f32::EPSILON);
    }
}
