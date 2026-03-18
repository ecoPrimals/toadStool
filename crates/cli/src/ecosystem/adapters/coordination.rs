// SPDX-License-Identifier: AGPL-3.0-or-later
//! Coordination adapter - capability-based coordination and service discovery
//!
//! This adapter replaces the hardcoded Songbird integration with a generic
//! coordination adapter that works with ANY service providing coordination capabilities.
//!
//! # Migration from Songbird
//! ```rust,ignore
//! // ❌ OLD: Hardcoded Songbird (services/songbird.rs)
//! use crate::ecosystem::services::songbird;
//! let response = songbird::send_registration(&addr, &registration).await?;
//!
//! // ✅ NEW: Capability-based (adapters/coordination.rs)
//! use crate::ecosystem::adapters::CoordinationAdapter;
//! let token = coordination.register_service(service_info).await?;
//! ```

use crate::{CliContextExt, Result};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

use super::universal::{Request, UniversalServiceAdapter};
use crate::ecosystem::capabilities::StandardCapability;

/// Coordination adapter - provides coordination operations via capability discovery
///
/// This adapter discovers and invokes coordination services without knowing their identity.
/// Services could be Songbird, Consul, etcd, Kubernetes service discovery, or custom implementations.
pub struct CoordinationAdapter {
    /// Universal service adapter for invoking capabilities
    universal: Arc<UniversalServiceAdapter>,
}

impl CoordinationAdapter {
    /// Create a new coordination adapter
    pub const fn new(universal: Arc<UniversalServiceAdapter>) -> Self {
        Self { universal }
    }

    /// Register this service with the coordination service
    ///
    /// Discovers a service registry and registers this service, making it
    /// discoverable by other services in the ecosystem.
    ///
    /// # Example
    /// ```ignore
    /// // Forward-looking example - API under development
    /// # use toadstool_cli::ecosystem::adapters::CoordinationAdapter;
    /// # async fn example(coordination: CoordinationAdapter) -> anyhow::Result<()> {
    /// let token = coordination.register_service(ServiceInfo {
    ///     name: "toadstool".to_string(),
    ///     capabilities: vec!["compute.wasm.component-model".to_string()],
    ///     endpoint: format!("http://{}:{}", toadstool_common::constants::DEFAULT_HOSTNAME, toadstool_common::constants::DEFAULT_HTTP_PORT),
    ///     metadata: Default::default(),
    /// }).await?;
    ///
    /// println!("Registered with token: {:?}", token);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    /// Returns an error if:
    /// - The coordination service cannot be reached
    /// - Service registration is rejected or fails
    /// - The response is malformed or missing the registration token
    #[must_use = "Service registration result should be checked"]
    pub async fn register_service(&self, service_info: ServiceInfo) -> Result<RegistrationToken> {
        let capability = StandardCapability::CoordinationServiceRegistry.id();

        let request = Request::new(
            "register",
            json!({
                "name": service_info.name,
                "capabilities": service_info.capabilities,
                "endpoint": service_info.endpoint,
                "metadata": service_info.metadata,
            }),
        );

        let response = self
            .universal
            .invoke(capability, request)
            .await
            .context("Failed to register service")?;

        let data = response.data()?;
        let token = data
            .get("token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| crate::CliError::Other("Missing token in response".to_string()))?;

        Ok(RegistrationToken {
            token: token.to_string(),
        })
    }

    /// Discover peer services
    ///
    /// Queries the coordination service to find other services providing
    /// specific capabilities.
    ///
    /// # Example
    /// ```ignore
    /// // Forward-looking example - API under development
    /// # use toadstool_cli::ecosystem::adapters::CoordinationAdapter;
    /// # async fn example(coordination: CoordinationAdapter) -> anyhow::Result<()> {
    /// // Find all services providing crypto capabilities
    /// let peers = coordination.discover_peers(Some("crypto.*")).await?;
    ///
    /// for peer in peers {
    ///     println!("Found: {} at {}", peer.name, peer.endpoint);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    /// Returns an error if:
    /// - The coordination service cannot be reached
    /// - Peer discovery fails or times out
    /// - The response is malformed or missing peer information
    #[must_use = "Peer discovery result should be checked"]
    pub async fn discover_peers(&self, capability_filter: Option<&str>) -> Result<Vec<PeerInfo>> {
        let capability = StandardCapability::CoordinationPeerDiscovery.id();

        let request = Request::new(
            "discover",
            json!({
                "capability": capability_filter,
            }),
        );

        let response = self
            .universal
            .invoke(capability, request)
            .await
            .context("Failed to discover peers")?;

        let data = response.data()?;
        let peers = data
            .get("peers")
            .and_then(|v| v.as_array())
            .ok_or_else(|| crate::CliError::Other("Missing peers in response".to_string()))?;

        let mut result = Vec::new();
        for peer in peers {
            if let Some(obj) = peer.as_object() {
                result.push(PeerInfo {
                    name: obj
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    endpoint: obj
                        .get("endpoint")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    capabilities: obj
                        .get("capabilities")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str())
                                .map(String::from)
                                .collect()
                        })
                        .unwrap_or_default(),
                    health: obj
                        .get("health")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                });
            }
        }

        Ok(result)
    }

    /// Send heartbeat to maintain service registration
    ///
    /// # Errors
    /// Returns an error if:
    /// - The coordination service cannot be reached
    /// - The heartbeat is rejected (invalid or expired token)
    /// - Network communication fails
    #[must_use = "Heartbeat send result should be checked"]
    pub async fn send_heartbeat(&self, token: &RegistrationToken) -> Result<()> {
        let capability = StandardCapability::CoordinationHealthCheck.id();

        let request = Request::new(
            "heartbeat",
            json!({
                "token": token.token,
            }),
        );

        self.universal
            .invoke(capability, request)
            .await
            .context("Failed to send heartbeat")?;

        Ok(())
    }

    /// Unregister service
    ///
    /// # Errors
    /// Returns an error if:
    /// - The coordination service cannot be reached
    /// - The unregistration fails (invalid token, service not found)
    /// - Network communication fails
    #[must_use = "Service unregistration result should be checked"]
    pub async fn unregister_service(&self, token: &RegistrationToken) -> Result<()> {
        let capability = StandardCapability::CoordinationServiceRegistry.id();

        let request = Request::new(
            "unregister",
            json!({
                "token": token.token,
            }),
        );

        self.universal
            .invoke(capability, request)
            .await
            .context("Failed to unregister service")?;

        Ok(())
    }

    /// Get configuration value
    ///
    /// # Errors
    /// Returns an error if:
    /// - The coordination service cannot be reached
    /// - The configuration key is not found
    /// - The response is malformed or missing the value
    #[must_use = "Configuration get result should be checked"]
    pub async fn get_config(&self, key: &str) -> Result<serde_json::Value> {
        let capability = StandardCapability::CoordinationConfigManagement.id();

        let request = Request::new(
            "get",
            json!({
                "key": key,
            }),
        );

        let response = self
            .universal
            .invoke(capability, request)
            .await
            .context("Failed to get configuration")?;

        let data = response.data()?;
        let value = data
            .get("value")
            .cloned()
            .ok_or_else(|| crate::CliError::Other("Missing value in response".to_string()))?;

        Ok(value)
    }

    /// Set configuration value
    ///
    /// # Errors
    /// Returns an error if:
    /// - The coordination service cannot be reached
    /// - The configuration cannot be set (permissions, validation)
    /// - Network communication fails
    #[must_use = "Configuration set result should be checked"]
    pub async fn set_config(&self, key: &str, value: serde_json::Value) -> Result<()> {
        let capability = StandardCapability::CoordinationConfigManagement.id();

        let request = Request::new(
            "set",
            json!({
                "key": key,
                "value": value,
            }),
        );

        self.universal
            .invoke(capability, request)
            .await
            .context("Failed to set configuration")?;

        Ok(())
    }

    /// Acquire distributed lock
    ///
    /// # Errors
    /// Returns an error if:
    /// - The coordination service cannot be reached
    /// - The lock is already held by another process
    /// - The response is malformed or missing the lock ID
    #[must_use = "Lock acquisition result should be checked"]
    pub async fn acquire_lock(&self, lock_name: &str, ttl_seconds: u64) -> Result<LockHandle> {
        let capability = StandardCapability::CoordinationDistributedLock.id();

        let request = Request::new(
            "acquire",
            json!({
                "lock_name": lock_name,
                "ttl_seconds": ttl_seconds,
            }),
        );

        let response = self
            .universal
            .invoke(capability, request)
            .await
            .context("Failed to acquire lock")?;

        let data = response.data()?;
        let lock_id = data
            .get("lock_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| crate::CliError::Other("Missing lock_id in response".to_string()))?;

        Ok(LockHandle {
            lock_name: lock_name.to_string(),
            lock_id: lock_id.to_string(),
        })
    }

    /// Release distributed lock
    ///
    /// # Errors
    /// Returns an error if:
    /// - The coordination service cannot be reached
    /// - The lock cannot be released (already released, expired)
    /// - Network communication fails
    #[must_use = "Lock release result should be checked"]
    pub async fn release_lock(&self, handle: LockHandle) -> Result<()> {
        let capability = StandardCapability::CoordinationDistributedLock.id();

        let request = Request::new(
            "release",
            json!({
                "lock_id": handle.lock_id,
            }),
        );

        self.universal
            .invoke(capability, request)
            .await
            .context("Failed to release lock")?;

        Ok(())
    }
}

/// Service information for registration
#[derive(Debug, Clone)]
pub struct ServiceInfo {
    /// Service name
    pub name: String,
    /// Capability IDs this service provides
    pub capabilities: Vec<String>,
    /// Service endpoint URL
    pub endpoint: String,
    /// Optional metadata key-value pairs
    pub metadata: HashMap<String, String>,
}

/// Registration token from coordination service
#[derive(Debug, Clone)]
pub struct RegistrationToken {
    /// Opaque token for heartbeat and unregister
    pub token: String,
}

/// Peer service information from discovery
#[derive(Debug, Clone)]
pub struct PeerInfo {
    /// Peer service name
    pub name: String,
    /// Peer endpoint URL
    pub endpoint: String,
    /// Capabilities the peer provides
    pub capabilities: Vec<String>,
    /// Health status (healthy, unhealthy, unknown)
    pub health: String,
}

/// Distributed lock handle for coordination
#[derive(Debug, Clone)]
pub struct LockHandle {
    /// Lock name (resource being locked)
    pub lock_name: String,
    /// Unique lock ID for release
    pub lock_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_info_creation() {
        let info = ServiceInfo {
            name: "test-service".to_string(),
            capabilities: vec!["compute.native".to_string()],
            endpoint: toadstool_common::constants::http_url(
                toadstool_common::constants::DEFAULT_HOSTNAME,
                toadstool_common::constants::DEFAULT_HTTP_PORT,
            ),
            metadata: HashMap::new(),
        };

        assert_eq!(info.name, "test-service");
        assert_eq!(info.capabilities.len(), 1);
    }

    #[test]
    fn test_service_info_with_metadata() {
        let mut metadata = HashMap::new();
        metadata.insert("version".to_string(), "1.0".to_string());
        metadata.insert("region".to_string(), "us-east".to_string());
        let info = ServiceInfo {
            name: "storage-svc".to_string(),
            capabilities: vec!["storage.s3".to_string(), "storage.kv".to_string()],
            endpoint: "http://localhost:9000".to_string(),
            metadata,
        };
        assert_eq!(info.capabilities.len(), 2);
        assert_eq!(info.metadata.get("version"), Some(&"1.0".to_string()));
    }

    #[test]
    fn test_registration_token() {
        let token = RegistrationToken {
            token: "abc-123-xyz".to_string(),
        };
        assert_eq!(token.token, "abc-123-xyz");
    }

    #[test]
    fn test_registration_token_clone() {
        let token = RegistrationToken {
            token: "test-token".to_string(),
        };
        let cloned = token.clone();
        assert_eq!(token.token, cloned.token);
    }

    #[test]
    fn test_peer_info_creation() {
        let peer = PeerInfo {
            name: "songbird".to_string(),
            endpoint: "http://192.168.1.10:8080".to_string(),
            capabilities: vec!["discovery".to_string(), "coordination".to_string()],
            health: "healthy".to_string(),
        };
        assert_eq!(peer.name, "songbird");
        assert_eq!(peer.capabilities.len(), 2);
        assert_eq!(peer.health, "healthy");
    }

    #[test]
    fn test_lock_handle_creation() {
        let handle = LockHandle {
            lock_name: "migration-lock".to_string(),
            lock_id: "uuid-12345".to_string(),
        };
        assert_eq!(handle.lock_name, "migration-lock");
        assert_eq!(handle.lock_id, "uuid-12345");
    }

    #[test]
    fn test_lock_handle_clone() {
        let handle = LockHandle {
            lock_name: "lock".to_string(),
            lock_id: "id".to_string(),
        };
        let cloned = handle.clone();
        assert_eq!(handle.lock_name, cloned.lock_name);
    }

    #[test]
    fn test_coordination_adapter_new() {
        use crate::ecosystem::adapters::AdapterFactory;
        let factory = AdapterFactory::new();
        let adapter = factory.coordination_adapter().unwrap();
        let _ = adapter;
    }

    #[tokio::test]
    async fn test_register_service_no_coordination_returns_err() {
        use crate::ecosystem::adapters::AdapterFactory;
        use std::collections::HashMap;

        let factory = AdapterFactory::new();
        let coordination = factory.coordination_adapter().unwrap();

        let service_info = ServiceInfo {
            name: "test-svc".to_string(),
            capabilities: vec!["compute.native".to_string()],
            endpoint: "http://127.0.0.1:9999".to_string(),
            metadata: HashMap::new(),
        };

        let result = coordination.register_service(service_info).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_discover_peers_no_service_returns_err() {
        use crate::ecosystem::adapters::AdapterFactory;

        let factory = AdapterFactory::new();
        let coordination = factory.coordination_adapter().unwrap();

        let result = coordination.discover_peers(Some("crypto.*")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_send_heartbeat_no_service_returns_err() {
        use crate::ecosystem::adapters::AdapterFactory;

        let factory = AdapterFactory::new();
        let coordination = factory.coordination_adapter().unwrap();

        let token = RegistrationToken {
            token: "test-token".to_string(),
        };
        let result = coordination.send_heartbeat(&token).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_unregister_service_no_service_returns_err() {
        use crate::ecosystem::adapters::AdapterFactory;

        let factory = AdapterFactory::new();
        let coordination = factory.coordination_adapter().unwrap();

        let token = RegistrationToken {
            token: "test-token".to_string(),
        };
        let result = coordination.unregister_service(&token).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_config_no_service_returns_err() {
        use crate::ecosystem::adapters::AdapterFactory;

        let factory = AdapterFactory::new();
        let coordination = factory.coordination_adapter().unwrap();

        let result = coordination.get_config("test_key").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_set_config_no_service_returns_err() {
        use crate::ecosystem::adapters::AdapterFactory;

        let factory = AdapterFactory::new();
        let coordination = factory.coordination_adapter().unwrap();

        let result = coordination
            .set_config("key", serde_json::json!("value"))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_acquire_lock_no_service_returns_err() {
        use crate::ecosystem::adapters::AdapterFactory;

        let factory = AdapterFactory::new();
        let coordination = factory.coordination_adapter().unwrap();

        let result = coordination.acquire_lock("migration-lock", 30).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_release_lock_no_service_returns_err() {
        use crate::ecosystem::adapters::AdapterFactory;

        let factory = AdapterFactory::new();
        let coordination = factory.coordination_adapter().unwrap();

        let handle = LockHandle {
            lock_name: "lock".to_string(),
            lock_id: "id-123".to_string(),
        };
        let result = coordination.release_lock(handle).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_discover_peers_none_filter() {
        use crate::ecosystem::adapters::AdapterFactory;

        let factory = AdapterFactory::new();
        let coordination = factory.coordination_adapter().unwrap();

        // Same error - no service, but tests the None path for capability_filter
        let result = coordination.discover_peers(None).await;
        assert!(result.is_err());
    }
}
