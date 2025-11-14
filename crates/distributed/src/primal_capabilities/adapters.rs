//! Primal Adapters
//!
//! Pluggable adapters for different primals in the ecoPrimals ecosystem

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::registry::Capability;
use anyhow::Result;

/// Trait for primal adapters
///
/// Implement this trait to add support for a new primal.
/// Each primal can have its own communication protocol and registration format.
#[async_trait]
pub trait PrimalAdapter: Send + Sync {
    /// Get the primal name
    fn primal_name(&self) -> &str;

    /// Get the primal endpoint
    fn endpoint(&self) -> &str;

    /// Register capabilities with the primal
    async fn register_capabilities(&self, capabilities: Vec<Capability>) -> Result<()>;

    /// Send heartbeat to the primal
    async fn send_heartbeat(&self) -> Result<()>;

    /// Notify primal of capability change
    async fn notify_capability_change(
        &self,
        capability: &Capability,
        available: bool,
    ) -> Result<()>;

    /// Deregister from the primal
    async fn deregister(&self) -> Result<()>;
}

/// Songbird primal adapter
///
/// Implements the Songbird Federation API for capability registration
pub struct SongbirdAdapter {
    endpoint: String,
    client: Client,
    toadstool_endpoint: String,
}

impl SongbirdAdapter {
    /// Create a new Songbird adapter
    pub fn new(songbird_endpoint: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        // Get ToadStool's own endpoint from environment or use default
        let toadstool_endpoint = std::env::var("TOADSTOOL_ENDPOINT")
            .unwrap_or_else(|_| "http://localhost:8084".to_string());

        Self {
            endpoint: songbird_endpoint.to_string(),
            client,
            toadstool_endpoint,
        }
    }
}

#[async_trait]
impl PrimalAdapter for SongbirdAdapter {
    fn primal_name(&self) -> &str {
        "songbird"
    }

    fn endpoint(&self) -> &str {
        &self.endpoint
    }

    async fn register_capabilities(&self, capabilities: Vec<Capability>) -> Result<()> {
        // Songbird Federation API: POST /api/v1/federation/register
        let url = format!("{}/api/v1/federation/register", self.endpoint);

        let registration = SongbirdRegistrationRequest {
            service_id: "toadstool".to_string(),
            service_endpoint: self.toadstool_endpoint.clone(),
            capabilities: capabilities
                .iter()
                .map(|c| SongbirdCapability {
                    capability_id: c.id.clone(),
                    capability_name: c.name.clone(),
                    description: c.description.clone(),
                    tags: c.tags.clone(),
                    resource_requirements: SongbirdResourceRequirements {
                        min_cpu_cores: c.resource_requirements.min_cpu_cores,
                        min_memory_mb: c.resource_requirements.min_memory_mb,
                        gpu_required: c.resource_requirements.gpu_required,
                        gpu_memory_mb: c.resource_requirements.gpu_memory_mb,
                    },
                    available: c.available,
                    confidence: c.confidence,
                })
                .collect(),
            workload_endpoint: format!("{}/api/v1/workload/execute", self.toadstool_endpoint),
        };

        let response = self.client.post(&url).json(&registration).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!(
                "Songbird registration failed ({}): {}",
                status,
                body
            ));
        }

        tracing::info!(
            "Successfully registered {} capabilities with Songbird at {}",
            capabilities.len(),
            self.endpoint
        );

        Ok(())
    }

    async fn send_heartbeat(&self) -> Result<()> {
        // Songbird Federation API: POST /api/v1/federation/heartbeat
        let url = format!("{}/api/v1/federation/heartbeat", self.endpoint);

        let heartbeat = SongbirdHeartbeat {
            service_id: "toadstool".to_string(),
            timestamp: chrono::Utc::now(),
            status: "healthy".to_string(),
        };

        let response = self.client.post(&url).json(&heartbeat).send().await?;

        if !response.status().is_success() {
            tracing::warn!("Heartbeat to Songbird failed: {}", response.status());
        }

        Ok(())
    }

    async fn notify_capability_change(
        &self,
        capability: &Capability,
        available: bool,
    ) -> Result<()> {
        // Songbird Federation API: POST /api/v1/federation/capability/update
        let url = format!("{}/api/v1/federation/capability/update", self.endpoint);

        let update = SongbirdCapabilityUpdate {
            service_id: "toadstool".to_string(),
            capability_id: capability.id.clone(),
            available,
            timestamp: chrono::Utc::now(),
        };

        let response = self.client.post(&url).json(&update).send().await?;

        if !response.status().is_success() {
            tracing::warn!(
                "Capability update to Songbird failed: {}",
                response.status()
            );
        }

        Ok(())
    }

    async fn deregister(&self) -> Result<()> {
        // Songbird Federation API: DELETE /api/v1/federation/deregister
        let url = format!("{}/api/v1/federation/deregister", self.endpoint);

        let request = SongbirdDeregisterRequest {
            service_id: "toadstool".to_string(),
        };

        let response = self.client.delete(&url).json(&request).send().await?;

        if !response.status().is_success() {
            tracing::warn!("Deregistration from Songbird failed: {}", response.status());
        } else {
            tracing::info!(
                "Successfully deregistered from Songbird at {}",
                self.endpoint
            );
        }

        Ok(())
    }
}

// Songbird-specific types (based on Songbird's Federation API)

#[derive(Debug, Serialize, Deserialize)]
struct SongbirdRegistrationRequest {
    service_id: String,
    service_endpoint: String,
    capabilities: Vec<SongbirdCapability>,
    workload_endpoint: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct SongbirdCapability {
    capability_id: String,
    capability_name: String,
    description: String,
    tags: Vec<String>,
    resource_requirements: SongbirdResourceRequirements,
    available: bool,
    confidence: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct SongbirdResourceRequirements {
    min_cpu_cores: u32,
    min_memory_mb: u64,
    gpu_required: bool,
    gpu_memory_mb: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SongbirdHeartbeat {
    service_id: String,
    timestamp: chrono::DateTime<chrono::Utc>,
    status: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct SongbirdCapabilityUpdate {
    service_id: String,
    capability_id: String,
    available: bool,
    timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SongbirdDeregisterRequest {
    service_id: String,
}

// Future primal adapters can be added here:
// - SquirrelAdapter (for ML coordination)
// - BearDogAdapter (for authentication/security)
// - CustomAdapter (for custom primals)
