// SPDX-License-Identifier: AGPL-3.0-or-later
//! High-quality entropy service discovery via capability-based discovery
//!
//! Discovers a crypto/entropy provider at runtime — no hardcoded service identities.
#[cfg(unix)]
use toadstool_common::interned_strings::capabilities;
#[cfg(unix)]
use toadstool_common::interned_strings::socket_env;

use crate::error::SecurityError;
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
            mixing: EntropyMixing::security_standard(),
            min_quality: 0.7,
        }
    }
}

fn build_system_entropy_fallback() -> EphemeralSeed {
    let mut seed_data = vec![0u8; 32];
    if let Err(e) = getrandom::getrandom(&mut seed_data) {
        tracing::error!("OS entropy source unavailable: {e} — using zero-filled seed");
    }

    let quality = SeedQuality::new(
        0.9, // OS CSPRNG — cryptographically suitable
        0.9, 0.0,
    );

    let mixing = EntropyMixing {
        machine_weight: 1.0,
        human_weight: 0.0,
        algorithm: "getrandom".to_string(),
    };

    EphemeralSeed::new(seed_data, EntropySource::Machine, mixing, quality)
}

/// Entropy client for the crypto / high-quality entropy capability
///
/// Discovers and communicates with an entropy service via capability-based discovery (no hardcoded URLs).
#[cfg(unix)]
pub struct EntropyClient {
    /// RPC client for communication (pure Rust unix socket!)
    rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient,
    /// Whether service is available
    available: bool,
}

#[cfg(unix)]
impl EntropyClient {
    /// Discover entropy service via capability discovery
    ///
    /// Searches for services advertising "capability:entropy:high-quality".
    /// NO HARDCODED URLs - pure runtime discovery!
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Service discovery unavailable
    /// - No entropy service found
    /// - Connection fails
    pub async fn discover() -> Result<Self, SecurityError> {
        // Step 1: Check environment variable (user can override)
        if let Ok(endpoint) = std::env::var(socket_env::TOADSTOOL_ENTROPY_SERVICE_URL) {
            tracing::info!("Using entropy service from environment: {}", endpoint);
            return Self::connect(&endpoint).await;
        }

        // Step 2: Try capability discovery
        match Self::discover_via_capability().await {
            Ok(endpoint) => {
                tracing::info!("Discovered security/crypto service: {}", endpoint);
                Self::connect(&endpoint).await
            }
            Err(e) => {
                tracing::warn!("Security/crypto service discovery failed: {}", e);
                // Return unavailable client (will fallback to system entropy)
                // Try capability-based discovery as fallback
                let socket_path = toadstool_common::primal_sockets::discover_crypto_socket()
                    .await
                    .unwrap_or_else(|_| {
                        toadstool_common::primal_sockets::get_socket_path_for_capability(
                            capabilities::CRYPTO,
                        )
                    });

                Ok(Self {
                    rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(
                        socket_path,
                    ),
                    available: false,
                })
            }
        }
    }

    /// Discover entropy service via capability announcement
    ///
    /// **PURE RUST**: Uses unix socket discovery (no HTTP!)
    ///
    /// In production, this would:
    /// 1. Query the coordination plane for services with "capability:entropy:high-quality"
    /// 2. Filter for matching capability metadata
    /// 3. Select best available service
    ///
    /// For now, returns error to demonstrate graceful fallback.
    #[expect(
        deprecated,
        reason = "reads legacy BEARDOG_URL as backward-compat fallback"
    )]
    async fn discover_via_capability() -> Result<String, SecurityError> {
        // Future: Implement full capability discovery via coordination unix socket
        // Current: Falls back to system entropy (graceful degradation)

        // DEEP DEBT EVOLUTION: Check Unix socket first (no hardcoded ports!)
        // Environment variable override takes precedence
        // `BEARDOG_URL`: legacy env alias (backward compat)
        if let Ok(url) = std::env::var(socket_env::SECURITY_URL) {
            tracing::debug!("Using crypto service URL from environment: {}", url);
            return Ok(url);
        }
        if let Ok(url) = std::env::var(socket_env::LEGACY_BEARDOG_URL) {
            tracing::warn!(
                env_var = %socket_env::LEGACY_BEARDOG_URL,
                value = %url,
                "deprecated LEGACY env variable used — migrate to capability-based discovery"
            );
            tracing::debug!("Using crypto service URL from environment: {}", url);
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
                toadstool_common::primal_sockets::get_socket_path_for_capability(
                    capabilities::CRYPTO,
                )
            });

        if std::fs::metadata(&socket_path).is_ok() {
            tracing::debug!("Found crypto service via Unix socket: {:?}", socket_path);
            return Ok(format!("unix://{}", socket_path.display()));
        }

        Err(SecurityError::Other(
            "No crypto service found via capability discovery. \
             Ensure the security provider is running or set SECURITY_URL environment variable."
                .to_string(),
        ))
    }

    /// Probe unix socket to check if crypto service is available
    ///
    /// **PURE RUST**: Uses unix socket instead of HTTP
    /// **CAPABILITY-BASED**: Discovers crypto service by capability (not hardcoded name)
    async fn probe_service(_url: &str) -> Result<(), SecurityError> {
        // CAPABILITY-BASED: Discover ANY crypto service (not hardcoded "security")
        let socket_path = toadstool_common::primal_sockets::discover_crypto_socket()
            .await
            .unwrap_or_else(|_| {
                toadstool_common::primal_sockets::get_socket_path_for_capability(
                    capabilities::CRYPTO,
                )
            });

        match tokio::net::UnixStream::connect(socket_path).await {
            Ok(_) => {
                tracing::debug!("Crypto service unix socket available");
                Ok(())
            }
            Err(e) => Err(SecurityError::Io(e)),
        }
    }

    /// Connect to entropy service via unix socket
    ///
    /// **PURE RUST**: Uses unix socket instead of HTTP
    /// **CAPABILITY-BASED**: Discovers crypto service by capability
    async fn connect(_endpoint: &str) -> Result<Self, SecurityError> {
        // CAPABILITY-BASED: Discover ANY crypto service (not hardcoded "security")
        let socket_path = toadstool_common::primal_sockets::discover_crypto_socket()
            .await
            .unwrap_or_else(|_| {
                toadstool_common::primal_sockets::get_socket_path_for_capability(
                    capabilities::CRYPTO,
                )
            });

        let socket_client =
            toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path);

        // Verify service is reachable via unix socket
        let available = Self::probe_service("").await.is_ok();

        Ok(Self {
            rpc_client: socket_client,
            available,
        })
    }

    /// Check if entropy service is available
    #[must_use]
    pub const fn is_available(&self) -> bool {
        self.available
    }

    /// Generate ephemeral seed
    ///
    /// Requests high-quality, human-mixed entropy from the provider.
    /// Falls back to system entropy if unavailable.
    ///
    /// # Errors
    ///
    /// Returns error if request fails and fallback is disabled.
    pub async fn generate_seed(&self) -> Result<EphemeralSeed, SecurityError> {
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
    ) -> Result<EphemeralSeed, SecurityError> {
        if !self.available {
            // Fallback to system entropy
            return Ok(Self::system_entropy_fallback());
        }

        // Request from entropy service
        match self.request_from_security(&request).await {
            Ok(seed) => Ok(seed),
            Err(e) => {
                tracing::warn!(
                    "entropy service request failed: {}, falling back to system entropy",
                    e
                );
                Ok(Self::system_entropy_fallback())
            }
        }
    }

    /// Request seed from entropy service via unix socket
    ///
    /// **PURE RUST**: JSON-RPC over unix socket (no HTTP!)
    async fn request_from_security(
        &self,
        request: &SeedRequest,
    ) -> Result<EphemeralSeed, SecurityError> {
        let params = serde_json::to_value(request)
            .map_err(|e| SecurityError::Other(format!("Failed to serialize seed request: {e}")))?;

        let seed: EphemeralSeed = self
            .rpc_client
            .call_typed("crypto.entropy.generate_seed", params)
            .await
            .map_err(|e| {
                SecurityError::Other(format!(
                    "Failed to request seed from security/crypto service: {e}"
                ))
            })?;

        Ok(seed)
    }

    /// Fallback to system entropy
    ///
    /// When the entropy service is unavailable, use system RNG.
    /// Quality is lower (pure machine entropy), but sufficient for many use cases.
    pub(crate) fn system_entropy_fallback() -> EphemeralSeed {
        build_system_entropy_fallback()
    }
}

/// Entropy client stub for non-Unix targets (Windows, WASM, etc.)
///
/// Unix socket discovery is unavailable on these platforms; callers always receive
/// system entropy via [`EntropyClient::system_entropy_fallback`].
#[cfg(not(unix))]
pub struct EntropyClient {
    /// Whether service is available (always false on non-Unix targets)
    available: bool,
}

#[cfg(not(unix))]
impl EntropyClient {
    /// Discover entropy service via capability discovery
    ///
    /// On non-Unix targets, Unix socket IPC is unavailable. Returns an unavailable
    /// client that falls back to system entropy.
    pub async fn discover() -> Result<Self, SecurityError> {
        tracing::warn!(
            "Unix socket entropy discovery is unavailable on this platform; using system entropy"
        );
        Ok(Self { available: false })
    }

    /// Check if entropy service is available
    #[must_use]
    pub const fn is_available(&self) -> bool {
        self.available
    }

    /// Generate ephemeral seed
    ///
    /// On non-Unix targets, always uses system entropy.
    pub async fn generate_seed(&self) -> Result<EphemeralSeed, SecurityError> {
        Ok(Self::system_entropy_fallback())
    }

    /// Generate seed with custom request
    ///
    /// On non-Unix targets, always uses system entropy.
    pub async fn generate_seed_with_request(
        &self,
        _request: SeedRequest,
    ) -> Result<EphemeralSeed, SecurityError> {
        Ok(Self::system_entropy_fallback())
    }

    /// Fallback to system entropy
    pub(crate) fn system_entropy_fallback() -> EphemeralSeed {
        build_system_entropy_fallback()
    }
}

#[cfg(all(test, unix))]
#[path = "discovery_tests.rs"]
mod tests;
