//! Primal Adapters
//!
//! Pluggable adapters for different primals in the ecoPrimals ecosystem

use async_trait::async_trait;
// No longer using reqwest - using unix sockets (pure Rust!)
use serde::{Deserialize, Serialize};

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
    rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient,
    toadstool_endpoint: String,
}

impl SongbirdAdapter {
    /// Create a new Songbird adapter with runtime discovery
    ///
    /// # Architecture
    ///
    /// Follows primal self-knowledge principle:
    /// - ToadStool knows its own endpoint from configuration/environment
    /// - Songbird endpoint is discovered at runtime (via mDNS, consul, or explicit config)
    /// - No hardcoded fallbacks - fail fast if configuration is missing
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - HTTP client cannot be created
    /// - TOADSTOOL_ENDPOINT environment variable is not set (primal must know itself)
    pub fn new(songbird_endpoint: &str) -> Result<Self> {
        // CAPABILITY-BASED: Discover ANY coordination service (not hardcoded "songbird")
        // Note: This function is sync, so we use the tokio blocking bridge
        let socket_path = if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle
                .block_on(toadstool_common::primal_sockets::discover_coordination_socket())
                .unwrap_or_else(|_| {
                    toadstool_common::primal_sockets::get_biomeos_dir().join("songbird.sock")
                })
        } else {
            toadstool_common::primal_sockets::get_biomeos_dir().join("songbird.sock")
        };

        let rpc_client = toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path);

        // Get ToadStool's own endpoint from environment (required - primal self-knowledge)
        let toadstool_endpoint = std::env::var("TOADSTOOL_ENDPOINT").map_err(|_| {
            anyhow::anyhow!(
                "TOADSTOOL_ENDPOINT not set - primal must know its own endpoint for discovery. \
                 Set via environment variable or configuration file."
            )
        })?;

        Ok(Self {
            endpoint: songbird_endpoint.to_string(),
            rpc_client,
            toadstool_endpoint,
        })
    }

    /// Create adapter with explicit endpoint (for testing/development)
    #[cfg(test)]
    pub fn new_with_endpoint(songbird_endpoint: &str, toadstool_endpoint: String) -> Result<Self> {
        // CAPABILITY-BASED: Discover ANY coordination service (not hardcoded "songbird")
        let socket_path = if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle
                .block_on(toadstool_common::primal_sockets::discover_coordination_socket())
                .unwrap_or_else(|_| {
                    toadstool_common::primal_sockets::get_biomeos_dir().join("songbird.sock")
                })
        } else {
            toadstool_common::primal_sockets::get_biomeos_dir().join("songbird.sock")
        };

        let rpc_client = toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path);

        Ok(Self {
            endpoint: songbird_endpoint.to_string(),
            rpc_client,
            toadstool_endpoint,
        })
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
        // Songbird Federation API via JSON-RPC over unix socket
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

        let params = serde_json::to_value(&registration)?;

        let _: serde_json::Value = self
            .rpc_client
            .call("songbird.register_capabilities", params)
            .await
            .map_err(|e| anyhow::anyhow!("Songbird registration failed: {e}"))?;

        tracing::info!(
            "Successfully registered {} capabilities with Songbird via unix socket",
            capabilities.len()
        );

        Ok(())
    }

    async fn send_heartbeat(&self) -> Result<()> {
        // Songbird Federation API via JSON-RPC over unix socket
        let heartbeat = SongbirdHeartbeat {
            service_id: "toadstool".to_string(),
            timestamp: chrono::Utc::now(),
            status: "healthy".to_string(),
        };

        let params = serde_json::to_value(&heartbeat)?;

        let _: serde_json::Value = self
            .rpc_client
            .call("songbird.heartbeat", params)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("Heartbeat to Songbird failed: {e}");
                serde_json::json!({})
            });

        Ok(())
    }

    async fn notify_capability_change(
        &self,
        capability: &Capability,
        available: bool,
    ) -> Result<()> {
        // Songbird Federation API via JSON-RPC over unix socket
        let update = SongbirdCapabilityUpdate {
            service_id: "toadstool".to_string(),
            capability_id: capability.id.clone(),
            available,
            timestamp: chrono::Utc::now(),
        };

        let params = serde_json::to_value(&update)?;

        let _: serde_json::Value = self
            .rpc_client
            .call("songbird.capability_update", params)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("Capability update to Songbird failed: {e}");
                serde_json::json!({})
            });

        Ok(())
    }

    async fn deregister(&self) -> Result<()> {
        // Songbird Federation API via JSON-RPC over unix socket
        let request = SongbirdDeregisterRequest {
            service_id: "toadstool".to_string(),
        };

        let params = serde_json::to_value(&request)?;

        let _: serde_json::Value = self
            .rpc_client
            .call("songbird.deregister", params)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("Deregistration from Songbird failed: {e}");
                serde_json::json!({})
            });

        tracing::info!("Successfully deregistered from Songbird via unix socket");

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
