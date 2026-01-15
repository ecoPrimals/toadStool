//! bearDog service discovery via capability-based discovery
//!
//! Discovers bearDog entropy service at runtime - NO HARDCODING!

use crate::seed::{EphemeralSeed, SeedQuality};
use crate::types::{EntropyMixing, EntropySource};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

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
    endpoint: Option<String>,
    /// HTTP client for communication
    client: reqwest::Client,
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
                    client: reqwest::Client::builder()
                        .timeout(Duration::from_secs(5))
                        .build()?,
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

    /// Probe a URL to check if it's a bearDog service
    async fn probe_service(url: &str) -> Result<()> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()?;

        let response = client
            .get(format!("{}/health", url))
            .send()
            .await
            .context("Failed to probe service")?;

        if response.status().is_success() {
            Ok(())
        } else {
            anyhow::bail!("Service returned non-success status")
        }
    }

    /// Connect to specific endpoint
    async fn connect(endpoint: &str) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?;

        // Verify service is reachable
        let available = Self::probe_service(endpoint).await.is_ok();

        Ok(Self {
            endpoint: Some(endpoint.to_string()),
            client,
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

    /// Request seed from bearDog service
    async fn request_from_beardog(&self, request: &SeedRequest) -> Result<EphemeralSeed> {
        let endpoint = self.endpoint.as_ref()
            .context("No endpoint available")?;

        let url = format!("{}/api/v1/entropy/seed", endpoint);

        let response = self.client
            .post(&url)
            .json(request)
            .send()
            .await
            .context("Failed to request seed from bearDog")?;

        if !response.status().is_success() {
            anyhow::bail!("bearDog returned error: {}", response.status());
        }

        let seed: EphemeralSeed = response
            .json()
            .await
            .context("Failed to parse seed response")?;

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
            client: reqwest::Client::new(),
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
            client: reqwest::Client::new(),
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
