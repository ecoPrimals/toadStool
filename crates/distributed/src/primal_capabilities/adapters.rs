// SPDX-License-Identifier: AGPL-3.0-or-later
//! Primal Adapters
//!
//! Pluggable adapters for different primals in the ecoPrimals ecosystem

#[cfg(unix)]
use toadstool_common::constants::PRIMAL_NAME;
use toadstool_common::interned_strings::capabilities;
use toadstool_common::interned_strings::socket_env;
// No longer using reqwest - using unix sockets (pure Rust!)
#[cfg(unix)]
use serde::{Deserialize, Serialize};
#[cfg(unix)]
use toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient;

use super::registry::Capability;
use crate::error::DistributedError;

/// HTTP path suffix for workload execution, appended to the ToadStool base URL (`TOADSTOOL_ENDPOINT`).
///
/// Override via [`CoordinationAdapterConfig::workload_execute_path`].
pub const WORKLOAD_EXECUTE_PATH: &str = "/api/v1/workload/execute";

/// Configuration for [`CoordinationAdapter`], including optional overrides for federation API paths.
#[derive(Debug, Clone)]
pub struct CoordinationAdapterConfig {
    /// Coordination primal base URL or socket identifier (adapter-specific).
    pub coordination_endpoint: String,
    /// Path suffix for workload execution (leading slash, e.g. [`WORKLOAD_EXECUTE_PATH`]).
    pub workload_execute_path: String,
}

impl CoordinationAdapterConfig {
    /// Creates config with the default [`WORKLOAD_EXECUTE_PATH`].
    #[must_use]
    pub fn new(coordination_endpoint: impl Into<String>) -> Self {
        Self {
            coordination_endpoint: coordination_endpoint.into(),
            workload_execute_path: WORKLOAD_EXECUTE_PATH.to_string(),
        }
    }

    /// Sets a custom workload execution path (for non-standard deployments or tests).
    #[must_use]
    pub fn workload_execute_path(mut self, path: impl Into<String>) -> Self {
        self.workload_execute_path = path.into();
        self
    }
}

/// Trait for primal adapters
///
/// Implement this trait to add support for a new primal.
/// Each primal can have its own communication protocol and registration format.
#[expect(
    async_fn_in_trait,
    reason = "all implementors are Send + Sync; trait is internal, no dyn dispatch"
)]
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

/// Coordination primal adapter
///
/// Implements the Coordination Federation API for capability registration
pub struct CoordinationAdapter {
    endpoint: String,
    #[cfg(unix)]
    rpc_client: UnixJsonRpcClient,
    #[cfg(unix)]
    toadstool_endpoint: String,
    #[cfg(unix)]
    workload_execute_path: String,
}

impl CoordinationAdapter {
    fn build(
        coordination_endpoint: String,
        toadstool_endpoint: String,
        workload_execute_path: String,
    ) -> Result<Self, DistributedError> {
        #[cfg(unix)]
        {
            let socket_path = toadstool_common::primal_sockets::get_socket_path_for_capability(
                capabilities::COORDINATION,
            );

            let rpc_client = UnixJsonRpcClient::new(socket_path);

            Ok(Self {
                endpoint: coordination_endpoint,
                rpc_client,
                toadstool_endpoint,
                workload_execute_path,
            })
        }
        #[cfg(not(unix))]
        {
            let _ = (
                coordination_endpoint,
                toadstool_endpoint,
                workload_execute_path,
            );
            Err(DistributedError::CoordinationRegistration(
                "Unix socket coordination adapter is unavailable on this platform".to_string(),
            ))
        }
    }

    /// Create a new Coordination adapter with runtime discovery
    ///
    /// # Architecture
    ///
    /// Follows primal self-knowledge principle:
    /// - ToadStool knows its own endpoint from configuration/environment
    /// - Coordination endpoint is discovered at runtime (via mDNS, consul, or explicit config)
    /// - No hardcoded fallbacks - fail fast if configuration is missing
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - HTTP client cannot be created
    /// - TOADSTOOL_ENDPOINT environment variable is not set (primal must know itself)
    pub fn new(coordination_endpoint: &str) -> Result<Self, DistributedError> {
        Self::from_config(CoordinationAdapterConfig::new(coordination_endpoint))
    }

    /// Create a coordination adapter from explicit configuration (path override supported).
    pub fn from_config(config: CoordinationAdapterConfig) -> Result<Self, DistributedError> {
        let toadstool_endpoint = std::env::var(socket_env::TOADSTOOL_ENDPOINT)
            .map_err(|_| DistributedError::ToadstoolEndpointNotSet)?;
        Self::build(
            config.coordination_endpoint,
            toadstool_endpoint,
            config.workload_execute_path,
        )
    }

    /// Create adapter with explicit endpoint (for testing/development)
    #[cfg(test)]
    pub fn new_with_endpoint(
        coordination_endpoint: &str,
        toadstool_endpoint: String,
    ) -> Result<Self, DistributedError> {
        Self::build(
            coordination_endpoint.to_string(),
            toadstool_endpoint,
            WORKLOAD_EXECUTE_PATH.to_string(),
        )
    }
}

impl PrimalAdapter for CoordinationAdapter {
    fn primal_name(&self) -> &str {
        capabilities::COORDINATION
    }

    fn endpoint(&self) -> &str {
        &self.endpoint
    }

    async fn register_capabilities(
        &self,
        capabilities: Vec<Capability>,
    ) -> Result<(), DistributedError> {
        #[cfg(not(unix))]
        {
            let _ = capabilities;
            return Err(DistributedError::CoordinationRegistration(
                "Unix socket coordination adapter is unavailable on this platform".to_string(),
            ));
        }
        #[cfg(unix)]
        {
            // Coordination Federation API via JSON-RPC over unix socket
            let registration = CoordinationRegistrationRequest {
                service_id: PRIMAL_NAME.to_string(),
                service_endpoint: self.toadstool_endpoint.clone(),
                capabilities: capabilities
                    .iter()
                    .map(|c| CoordinationCapability {
                        capability_id: c.id.clone(),
                        capability_name: c.name.clone(),
                        description: c.description.clone(),
                        tags: c.tags.clone(),
                        resource_requirements: CoordinationResourceRequirements {
                            min_cpu_cores: c.resource_requirements.min_cpu_cores,
                            min_memory_mb: c.resource_requirements.min_memory_mb,
                            gpu_required: c.resource_requirements.gpu_required,
                            gpu_memory_mb: c.resource_requirements.gpu_memory_mb,
                        },
                        available: c.available,
                        confidence: c.confidence,
                    })
                    .collect(),
                workload_endpoint: format!(
                    "{}{}",
                    self.toadstool_endpoint.trim_end_matches('/'),
                    self.workload_execute_path
                ),
            };

            let params = serde_json::to_value(&registration)?;

            let _: serde_json::Value = self
                .rpc_client
                .call("coordination.register_capabilities", params)
                .await
                .map_err(|e| DistributedError::CoordinationRegistration(e.to_string()))?;

            tracing::info!(
                "Successfully registered {} capabilities with coordination service via unix socket",
                capabilities.len()
            );

            Ok(())
        }
    }

    async fn send_heartbeat(&self) -> Result<(), DistributedError> {
        #[cfg(not(unix))]
        {
            return Ok(());
        }
        #[cfg(unix)]
        {
            // Coordination Federation API via JSON-RPC over unix socket
            let heartbeat = CoordinationHeartbeat {
                service_id: PRIMAL_NAME.to_string(),
                timestamp: std::time::SystemTime::now(),
                status: "healthy".to_string(),
            };

            let params = serde_json::to_value(&heartbeat)?;

            let _: serde_json::Value = self
                .rpc_client
                .call("coordination.heartbeat", params)
                .await
                .unwrap_or_else(|e| {
                    tracing::warn!("Heartbeat to coordination service failed: {e}");
                    serde_json::json!({})
                });

            Ok(())
        }
    }

    async fn notify_capability_change(
        &self,
        capability: &Capability,
        available: bool,
    ) -> Result<(), DistributedError> {
        #[cfg(not(unix))]
        {
            let _ = (capability, available);
            return Ok(());
        }
        #[cfg(unix)]
        {
            // Coordination Federation API via JSON-RPC over unix socket
            let update = CoordinationCapabilityUpdate {
                service_id: PRIMAL_NAME.to_string(),
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
                    tracing::warn!("Capability update to coordination service failed: {e}");
                    serde_json::json!({})
                });

            Ok(())
        }
    }

    async fn deregister(&self) -> Result<(), DistributedError> {
        #[cfg(not(unix))]
        {
            return Ok(());
        }
        #[cfg(unix)]
        {
            // Coordination Federation API via JSON-RPC over unix socket
            let request = CoordinationDeregisterRequest {
                service_id: PRIMAL_NAME.to_string(),
            };

            let params = serde_json::to_value(&request)?;

            let _: serde_json::Value = self
                .rpc_client
                .call("coordination.deregister", params)
                .await
                .unwrap_or_else(|e| {
                    tracing::warn!("Deregistration from coordination service failed: {e}");
                    serde_json::json!({})
                });

            tracing::info!("Successfully deregistered from coordination service via unix socket");

            Ok(())
        }
    }
}

// Coordination-specific types (based on Coordination's Federation API)

#[cfg(unix)]
#[derive(Debug, Serialize, Deserialize)]
struct CoordinationRegistrationRequest {
    service_id: String,
    service_endpoint: String,
    capabilities: Vec<CoordinationCapability>,
    workload_endpoint: String,
}

#[cfg(unix)]
#[derive(Debug, Serialize, Deserialize)]
struct CoordinationCapability {
    capability_id: String,
    capability_name: String,
    description: String,
    tags: Vec<String>,
    resource_requirements: CoordinationResourceRequirements,
    available: bool,
    confidence: f64,
}

#[cfg(unix)]
#[derive(Debug, Serialize, Deserialize)]
struct CoordinationResourceRequirements {
    min_cpu_cores: u32,
    min_memory_mb: u64,
    gpu_required: bool,
    gpu_memory_mb: Option<u64>,
}

#[cfg(unix)]
#[derive(Debug, Serialize, Deserialize)]
struct CoordinationHeartbeat {
    service_id: String,
    #[serde(with = "toadstool_common::system_time_serde")]
    timestamp: std::time::SystemTime,
    status: String,
}

#[cfg(unix)]
#[derive(Debug, Serialize, Deserialize)]
struct CoordinationCapabilityUpdate {
    service_id: String,
    capability_id: String,
    available: bool,
    #[serde(with = "toadstool_common::system_time_serde")]
    timestamp: std::time::SystemTime,
}

#[cfg(unix)]
#[derive(Debug, Serialize, Deserialize)]
struct CoordinationDeregisterRequest {
    service_id: String,
}

// Future adapters can be added here (capability-specific, not name-specific):
// - Security / auth plane
// - Custom JSON-RPC endpoints
