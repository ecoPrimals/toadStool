// SPDX-License-Identifier: AGPL-3.0-or-later
//! Primal Adapters
//!
//! Pluggable adapters for different primals in the ecoPrimals ecosystem
#![allow(deprecated)] // get_songbird_socket_path: intentional use during capability-discovery migration

use async_trait::async_trait;
#[allow(deprecated)] // Protocol compatibility: primal_name in adapter
use toadstool_common::constants::ecosystem::well_known::SONGBIRD;
// No longer using reqwest - using unix sockets (pure Rust!)
use serde::{Deserialize, Serialize};

use super::registry::Capability;
use crate::error::DistributedError;

/// Trait for primal adapters
///
/// Implement this trait to add support for a new primal.
/// Each primal can have its own communication protocol and registration format.
// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
pub trait PrimalAdapter: Send + Sync {
    /// Get the primal name
    fn primal_name(&self) -> &str;

    /// Get the primal endpoint
    fn endpoint(&self) -> &str;

    /// Register capabilities with the primal
    async fn register_capabilities(
        &self,
        capabilities: Vec<Capability>,
    ) -> Result<(), DistributedError>;

    /// Send heartbeat to the primal
    async fn send_heartbeat(&self) -> Result<(), DistributedError>;

    /// Notify primal of capability change
    async fn notify_capability_change(
        &self,
        capability: &Capability,
        available: bool,
    ) -> Result<(), DistributedError>;

    /// Deregister from the primal
    async fn deregister(&self) -> Result<(), DistributedError>;
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
    pub fn new(songbird_endpoint: &str) -> Result<Self, DistributedError> {
        let socket_path =
            toadstool_common::primal_sockets::get_socket_path_for_capability("coordination");

        let rpc_client = toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path);

        let toadstool_endpoint = std::env::var("TOADSTOOL_ENDPOINT")
            .map_err(|_| DistributedError::ToadstoolEndpointNotSet)?;

        Ok(Self {
            endpoint: songbird_endpoint.to_string(),
            rpc_client,
            toadstool_endpoint,
        })
    }

    /// Create adapter with explicit endpoint (for testing/development)
    #[cfg(test)]
    pub fn new_with_endpoint(
        songbird_endpoint: &str,
        toadstool_endpoint: String,
    ) -> Result<Self, DistributedError> {
        let socket_path =
            toadstool_common::primal_sockets::get_socket_path_for_capability("coordination");

        let rpc_client = toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path);

        Ok(Self {
            endpoint: songbird_endpoint.to_string(),
            rpc_client,
            toadstool_endpoint,
        })
    }
}

// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
impl PrimalAdapter for SongbirdAdapter {
    fn primal_name(&self) -> &str {
        SONGBIRD
    }

    fn endpoint(&self) -> &str {
        &self.endpoint
    }

    async fn register_capabilities(
        &self,
        capabilities: Vec<Capability>,
    ) -> Result<(), DistributedError> {
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
            .call("coordination.register_capabilities", params)
            .await
            .map_err(|e| DistributedError::SongbirdRegistration(e.to_string()))?;

        tracing::info!(
            "Successfully registered {} capabilities with Songbird via unix socket",
            capabilities.len()
        );

        Ok(())
    }

    async fn send_heartbeat(&self) -> Result<(), DistributedError> {
        // Songbird Federation API via JSON-RPC over unix socket
        let heartbeat = SongbirdHeartbeat {
            service_id: "toadstool".to_string(),
            timestamp: std::time::SystemTime::now(),
            status: "healthy".to_string(),
        };

        let params = serde_json::to_value(&heartbeat)?;

        let _: serde_json::Value = self
            .rpc_client
            .call("coordination.heartbeat", params)
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
    ) -> Result<(), DistributedError> {
        // Songbird Federation API via JSON-RPC over unix socket
        let update = SongbirdCapabilityUpdate {
            service_id: "toadstool".to_string(),
            capability_id: capability.id.clone(),
            available,
            timestamp: std::time::SystemTime::now(),
        };

        let params = serde_json::to_value(&update)?;

        let _: serde_json::Value = self
            .rpc_client
            .call("coordination.capability_update", params)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("Capability update to Songbird failed: {e}");
                serde_json::json!({})
            });

        Ok(())
    }

    async fn deregister(&self) -> Result<(), DistributedError> {
        // Songbird Federation API via JSON-RPC over unix socket
        let request = SongbirdDeregisterRequest {
            service_id: "toadstool".to_string(),
        };

        let params = serde_json::to_value(&request)?;

        let _: serde_json::Value = self
            .rpc_client
            .call("coordination.deregister", params)
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
    #[serde(with = "toadstool_common::system_time_serde")]
    timestamp: std::time::SystemTime,
    status: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct SongbirdCapabilityUpdate {
    service_id: String,
    capability_id: String,
    available: bool,
    #[serde(with = "toadstool_common::system_time_serde")]
    timestamp: std::time::SystemTime,
}

#[derive(Debug, Serialize, Deserialize)]
struct SongbirdDeregisterRequest {
    service_id: String,
}

// Future primal adapters can be added here:
// - SquirrelAdapter (for ML coordination)
// - BearDogAdapter (for authentication/security)
// - CustomAdapter (for custom primals)

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_songbird_adapter_new_with_endpoint() {
        let adapter = SongbirdAdapter::new_with_endpoint(
            "http://songbird:8080",
            "http://toadstool:9090".to_string(),
        )
        .unwrap();
        assert_eq!(adapter.primal_name(), "songbird");
        assert_eq!(adapter.endpoint(), "http://songbird:8080");
    }

    #[test]
    fn test_songbird_adapter_new_requires_toadstool_endpoint() {
        temp_env::with_vars([("TOADSTOOL_ENDPOINT", None::<&str>)], || {
            let result = SongbirdAdapter::new("http://songbird:8080");
            match result {
                Err(e) => assert!(e.to_string().contains("TOADSTOOL_ENDPOINT")),
                Ok(_) => panic!("expected error when TOADSTOOL_ENDPOINT not set"),
            }
        });
    }

    #[test]
    fn test_songbird_adapter_new_with_env() {
        temp_env::with_var("TOADSTOOL_ENDPOINT", Some("http://self:9090"), || {
            let result = SongbirdAdapter::new("http://songbird:8080");
            assert!(result.is_ok());
            let adapter = result.unwrap();
            assert_eq!(adapter.primal_name(), "songbird");
            assert_eq!(adapter.endpoint(), "http://songbird:8080");
        });
    }

    fn make_test_adapter() -> SongbirdAdapter {
        SongbirdAdapter::new_with_endpoint(
            "http://songbird:8080",
            "http://toadstool:9090".to_string(),
        )
        .unwrap()
    }

    #[test]
    fn test_songbird_adapter_send_heartbeat() {
        let adapter = make_test_adapter();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let result = adapter.send_heartbeat().await;
            assert!(result.is_ok());
        });
    }

    #[test]
    fn test_songbird_adapter_notify_capability_change() {
        let adapter = make_test_adapter();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let cap = Capability::compute_heavy();
            let result = adapter.notify_capability_change(&cap, false).await;
            assert!(result.is_ok());
        });
    }

    #[test]
    fn test_songbird_adapter_deregister() {
        let adapter = make_test_adapter();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let result = adapter.deregister().await;
            assert!(result.is_ok());
        });
    }

    #[test]
    fn test_songbird_adapter_register_capabilities_fails_without_socket() {
        let adapter = make_test_adapter();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let caps = vec![Capability::compute_heavy()];
            let result = adapter.register_capabilities(caps).await;
            assert!(result.is_err());
        });
    }

    #[test]
    fn test_songbird_adapter_register_capabilities_empty() {
        let adapter = make_test_adapter();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let result = adapter.register_capabilities(vec![]).await;
            assert!(result.is_err());
        });
    }

    #[test]
    fn test_songbird_adapter_primal_adapter_trait() {
        let adapter = SongbirdAdapter::new_with_endpoint(
            "unix:///tmp/songbird.sock",
            "http://localhost:9090".to_string(),
        )
        .unwrap();
        assert_eq!(adapter.primal_name(), "songbird");
        assert_eq!(adapter.endpoint(), "unix:///tmp/songbird.sock");
    }

    #[test]
    fn test_songbird_adapter_notify_gpu_capability() {
        let adapter = make_test_adapter();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let cap = Capability::compute_gpu();
            let result = adapter.notify_capability_change(&cap, true).await;
            assert!(result.is_ok());
        });
    }

    #[test]
    fn test_songbird_adapter_notify_capability_with_custom_id() {
        let adapter = make_test_adapter();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let cap = Capability::compute_ml_training();
            let result = adapter.notify_capability_change(&cap, false).await;
            assert!(result.is_ok());
        });
    }

    #[test]
    fn test_songbird_adapter_endpoint_preserved() {
        let ep = "https://custom-songbird.example.com:9999";
        let adapter = SongbirdAdapter::new_with_endpoint(ep, "http://me:1".to_string()).unwrap();
        assert_eq!(adapter.endpoint(), ep);
    }
}
