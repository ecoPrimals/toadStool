// SPDX-License-Identifier: AGPL-3.0-or-later
//! Coordination service client - RPC operations over unix socket
//!
//! **Design**: Works with ANY coordination provider via unix sockets (pure Rust!).
//! JSON-RPC over unix socket (no HTTP, no ring!).

use std::time::Duration;

#[cfg(unix)]
use toadstool_common::constants::timeouts;
use toadstool_common::primal_identity::ServiceEndpoint;
use toadstool_common::service_discovery::DiscoveredService;
#[cfg(unix)]
use toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient;
#[cfg(unix)]
use toadstool_common::{NetworkError, ToadStoolError, ToadStoolResult};
#[cfg(not(unix))]
use toadstool_common::{ToadStoolError, ToadStoolResult};

use crate::coordination_integration::types::{
    CoordinationRequest, CoordinationResponse, HealthCheckRequest, LoadBalancingRequest, NodeInfo,
    ServiceRegistration,
};

/// Coordination service client - Makes requests to discovered services
///
/// **Design**: Works with ANY coordination provider via unix sockets (pure Rust!)
#[derive(Debug)]
pub struct CoordinationClient {
    #[cfg(unix)]
    rpc_client: UnixJsonRpcClient,
    /// Service endpoint information (stored for diagnostics and future use)
    _service_endpoint: ServiceEndpoint,
    /// Request timeout for RPC calls
    timeout: Duration,
}

impl CoordinationClient {
    #[cfg(not(unix))]
    fn unix_unavailable<T>() -> ToadStoolResult<T> {
        Err(ToadStoolError::configuration(
            "Unix socket coordination client is unavailable on this platform",
        ))
    }

    /// Create client for a discovered service
    pub async fn new(service: &DiscoveredService) -> ToadStoolResult<Self> {
        #[cfg(unix)]
        {
            let endpoint = service.endpoints.first().ok_or_else(|| {
                ToadStoolError::Network(NetworkError::ConnectionFailed {
                    endpoint: service.name.clone(),
                    reason: "No endpoints available".to_string(),
                })
            })?;

            // CAPABILITY-BASED: Discover ANY coordination service (not hardcoded "coordination")
            let socket_path = toadstool_common::primal_sockets::discover_coordination_socket()
                .await
                .unwrap_or_else(|_| {
                    toadstool_common::primal_sockets::get_biomeos_dir().join("coordination.sock")
                });
            let rpc_client = UnixJsonRpcClient::new(socket_path);

            Ok(Self {
                rpc_client,
                _service_endpoint: endpoint.clone(),
                timeout: timeouts::DEFAULT_REQUEST_TIMEOUT,
            })
        }
        #[cfg(not(unix))]
        {
            let _ = service;
            Err(ToadStoolError::configuration(
                "Unix socket coordination client is unavailable on this platform",
            ))
        }
    }

    /// Create client with custom timeout
    pub async fn with_timeout(
        service: &DiscoveredService,
        timeout: Duration,
    ) -> ToadStoolResult<Self> {
        let mut client = Self::new(service).await?;
        client.timeout = timeout;
        Ok(client)
    }

    /// Create client with custom socket path (for testing error paths when service unavailable)
    #[cfg(all(unix, any(test, feature = "testing")))]
    pub fn new_for_testing(socket_path: std::path::PathBuf, timeout: Duration) -> Self {
        let rpc_client = UnixJsonRpcClient::new(socket_path);
        let endpoint = toadstool_common::primal_identity::ServiceEndpoint {
            protocol: "unix".to_string(),
            address: "/tmp/test-coordination.sock".to_string(),
            port: 0,
            path: None,
            metadata: std::collections::HashMap::new(),
        };
        Self {
            rpc_client,
            _service_endpoint: endpoint,
            timeout,
        }
    }

    /// Register a service with the coordination provider via unix socket
    pub async fn register_service(
        &self,
        registration: ServiceRegistration,
    ) -> ToadStoolResult<CoordinationResponse> {
        #[cfg(not(unix))]
        {
            let _ = registration;
            return Self::unix_unavailable();
        }
        #[cfg(unix)]
        {
            let params = serde_json::to_value(&registration).map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Failed to serialize registration: {e}"),
                })
            })?;

            tokio::time::timeout(
                self.timeout,
                self.rpc_client
                    .call_typed("coordination.register_service", params),
            )
            .await
            .map_err(|_| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Service registration timed out after {:?}", self.timeout),
                })
            })?
            .map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Service registration failed: {e}"),
                })
            })
        }
    }

    /// Discover services by capability via unix socket
    pub async fn discover_services(&self, capability: &str) -> ToadStoolResult<Vec<NodeInfo>> {
        #[cfg(not(unix))]
        {
            let _ = capability;
            return Self::unix_unavailable();
        }
        #[cfg(unix)]
        {
            let params = serde_json::json!({"capability": capability});

            tokio::time::timeout(
                self.timeout,
                self.rpc_client
                    .call_typed("coordination.discover_services", params),
            )
            .await
            .map_err(|_| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Service discovery timed out after {:?}", self.timeout),
                })
            })?
            .map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Service discovery failed: {e}"),
                })
            })
        }
    }

    /// Report health status via unix socket
    pub async fn report_health(
        &self,
        health: HealthCheckRequest,
    ) -> ToadStoolResult<CoordinationResponse> {
        #[cfg(not(unix))]
        {
            let _ = health;
            return Self::unix_unavailable();
        }
        #[cfg(unix)]
        {
            let params = serde_json::to_value(&health).map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Failed to serialize health: {e}"),
                })
            })?;

            tokio::time::timeout(
                self.timeout,
                self.rpc_client
                    .call_typed("coordination.report_health", params),
            )
            .await
            .map_err(|_| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Health report timed out after {:?}", self.timeout),
                })
            })?
            .map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Health report failed: {e}"),
                })
            })
        }
    }

    /// Get load balancing advice via unix socket
    pub async fn get_load_balancing(
        &self,
        request: LoadBalancingRequest,
    ) -> ToadStoolResult<Vec<NodeInfo>> {
        #[cfg(not(unix))]
        {
            let _ = request;
            return Self::unix_unavailable();
        }
        #[cfg(unix)]
        {
            let params = serde_json::to_value(&request).map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Failed to serialize request: {e}"),
                })
            })?;

            tokio::time::timeout(
                self.timeout,
                self.rpc_client
                    .call_typed("coordination.get_load_balancing", params),
            )
            .await
            .map_err(|_| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Load balancing timed out after {:?}", self.timeout),
                })
            })?
            .map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Load balancing request failed: {e}"),
                })
            })
        }
    }

    /// Health check via unix socket
    pub async fn health_check(&self) -> ToadStoolResult<bool> {
        #[cfg(not(unix))]
        {
            return Self::unix_unavailable();
        }
        #[cfg(unix)]
        {
            let result: serde_json::Value = tokio::time::timeout(
                self.timeout,
                self.rpc_client
                    .call("coordination.health", serde_json::json!({})),
            )
            .await
            .map_err(|_| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Health check timed out after {:?}", self.timeout),
                })
            })?
            .map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Health check failed: {e}"),
                })
            })?;

            Ok(result
                .get("healthy")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or_default())
        }
    }

    /// Execute generic coordination request via unix socket
    pub async fn execute(
        &self,
        request: CoordinationRequest,
    ) -> ToadStoolResult<CoordinationResponse> {
        #[cfg(not(unix))]
        {
            let _ = request;
            return Self::unix_unavailable();
        }
        #[cfg(unix)]
        {
            let params = serde_json::to_value(&request).map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Failed to serialize request: {e}"),
                })
            })?;

            tokio::time::timeout(
                self.timeout,
                self.rpc_client.call_typed("coordination.execute", params),
            )
            .await
            .map_err(|_| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Coordination request timed out after {:?}", self.timeout),
                })
            })?
            .map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Coordination request failed: {e}"),
                })
            })
        }
    }
}

#[cfg(test)]
#[path = "rpc_tests.rs"]
mod tests;
