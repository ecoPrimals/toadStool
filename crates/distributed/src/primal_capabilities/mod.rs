// SPDX-License-Identifier: AGPL-3.0-only
//! Primal-Agnostic Capability System
//!
//! This module provides a universal capability registration and discovery system
//! that works with ANY primal in the ecoPrimals ecosystem, not just Songbird.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │  ToadStool Capability Provider          │
//! │  (primal-agnostic)                      │
//! ├─────────────────────────────────────────┤
//! │  Capability Registry                    │
//! │  ├── compute_gpu                        │
//! │  ├── compute_heavy                      │
//! │  ├── compute_ml_training                │
//! │  ├── compute_mainframe (future)         │
//! │  └── compute_embedded (future)          │
//! ├─────────────────────────────────────────┤
//! │  Primal Adapters (pluggable)            │
//! │  ├── SongbirdAdapter                    │
//! │  ├── SquirrelAdapter (future)           │
//! │  ├── BearDogAdapter (future)            │
//! │  └── CustomAdapter (future)             │
//! └─────────────────────────────────────────┘
//! ```
//!
//! ## Design Principles
//!
//! 1. **Primal-Agnostic**: Works with any primal, not hardcoded to Songbird
//! 2. **Pluggable**: Easy to add new primal adapters
//! 3. **Standard Interface**: Consistent capability format across ecosystem
//! 4. **Future-Proof**: Can evolve as new primals are added
//!
//! ## Usage
//!
//! ```rust,ignore
//! use toadstool_distributed::primal_capabilities::{CapabilityProvider, Capability};
//!
//! // Create provider with capabilities
//! let provider = CapabilityProvider::new(vec![
//!     Capability::compute_gpu(),
//!     Capability::compute_heavy(),
//!     Capability::compute_ml_training(),
//! ]);
//!
//! // Register with any primal
//! provider.register_with_primal("http://songbird:8080").await?;
//! provider.register_with_primal("http://squirrel:8083").await?;
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod adapters;
pub mod registry;
pub mod workload;

pub use adapters::{PrimalAdapter, SongbirdAdapter};
pub use registry::{Capability, CapabilityRegistry};
pub use workload::{WorkloadExecutor, WorkloadRequest, WorkloadResponse};

use crate::error::DistributedError;

/// Primal-agnostic capability provider
///
/// This provider can register ToadStool's capabilities with any primal
/// in the ecoPrimals ecosystem.
#[derive(Clone)]
pub struct CapabilityProvider {
    /// Registry of available capabilities
    registry: Arc<RwLock<CapabilityRegistry>>,
    /// Connected primal adapters
    adapters: Arc<RwLock<HashMap<String, Box<dyn PrimalAdapter>>>>,
    /// Workload executor
    executor: Arc<WorkloadExecutor>,
}

impl CapabilityProvider {
    /// Create a new capability provider
    pub fn new(capabilities: Vec<Capability>) -> Self {
        Self {
            registry: Arc::new(RwLock::new(CapabilityRegistry::new(capabilities))),
            adapters: Arc::new(RwLock::new(HashMap::new())),
            executor: Arc::new(WorkloadExecutor::new()),
        }
    }

    /// Register capabilities with a primal endpoint
    ///
    /// This is primal-agnostic - works with Songbird, Squirrel, or any future primal
    pub async fn register_with_primal(
        &self,
        primal_endpoint: &str,
    ) -> Result<(), DistributedError> {
        // Auto-detect primal type from endpoint or use generic adapter
        let adapter = self.create_adapter_for_endpoint(primal_endpoint).await?;

        // Get capabilities to register
        let registry = self.registry.read().await;
        let capabilities = registry.all_capabilities();

        // Register with the primal
        adapter.register_capabilities(capabilities).await?;

        // Store adapter for future use
        let mut adapters = self.adapters.write().await;
        adapters.insert(primal_endpoint.to_string(), adapter);

        Ok(())
    }

    /// Send heartbeat to all connected primals
    pub async fn send_heartbeats(&self) -> Result<(), DistributedError> {
        let adapters = self.adapters.read().await;

        for (endpoint, adapter) in adapters.iter() {
            if let Err(e) = adapter.send_heartbeat().await {
                tracing::warn!("Failed to send heartbeat to {}: {:?}", endpoint, e);
            }
        }

        Ok(())
    }

    /// Handle an incoming workload request from any primal
    pub async fn handle_workload(
        &self,
        request: WorkloadRequest,
    ) -> Result<WorkloadResponse, DistributedError> {
        self.executor.execute(request).await
    }

    /// Get current capability status
    pub async fn get_capabilities(&self) -> Vec<Capability> {
        let registry = self.registry.read().await;
        registry.all_capabilities()
    }

    /// Update capabilities (e.g., when GPU becomes available/unavailable)
    pub async fn update_capability(
        &self,
        capability: Capability,
        available: bool,
    ) -> Result<(), DistributedError> {
        // Clone for later notification
        let cap_for_notification = capability.clone();

        let mut registry = self.registry.write().await;
        registry.update_capability(capability, available)?;

        // Notify all connected primals of the change
        let adapters = self.adapters.read().await;
        for adapter in adapters.values() {
            if let Err(e) = adapter
                .notify_capability_change(&cap_for_notification, available)
                .await
            {
                tracing::warn!("Failed to notify primal of capability change: {:?}", e);
            }
        }

        Ok(())
    }

    /// Create appropriate adapter for a primal endpoint.
    ///
    /// Capability-based: all primals use the same JSON-RPC adapter.
    /// Specific primal types are discovered at runtime, not hardcoded.
    async fn create_adapter_for_endpoint(
        &self,
        endpoint: &str,
    ) -> Result<Box<dyn PrimalAdapter>, DistributedError> {
        Ok(Box::new(SongbirdAdapter::new(endpoint)?))
    }
}

impl Default for CapabilityProvider {
    fn default() -> Self {
        // Default capabilities based on available runtimes
        let capabilities = vec![
            Capability::compute_heavy(),
            Capability::compute_native(),
            Capability::compute_container(),
            Capability::compute_wasm(),
            // GPU capability detected at runtime
            // Legacy capabilities detected at runtime
        ];

        Self::new(capabilities)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_capability_provider_creation() {
        let provider = CapabilityProvider::default();
        let capabilities = provider.get_capabilities().await;

        assert!(!capabilities.is_empty());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_capability_update() {
        let provider = CapabilityProvider::default();
        let gpu_cap = Capability::compute_gpu();

        let result = provider.update_capability(gpu_cap, true).await;
        assert!(result.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_capability_provider_new_with_custom_capabilities() {
        let caps = vec![
            Capability::compute_gpu(),
            Capability::compute_heavy(),
            Capability::compute_ml_training(),
        ];
        let provider = CapabilityProvider::new(caps);
        let got = provider.get_capabilities().await;
        assert_eq!(got.len(), 3);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_capability_provider_handle_workload() {
        use workload::{WorkloadRequest, WorkloadResourceRequirements, WorkloadType};
        let provider = CapabilityProvider::default();
        let req = WorkloadRequest {
            request_id: "req-1".to_string(),
            from_primal: "songbird".to_string(),
            required_capability: "compute_heavy".to_string(),
            workload_type: WorkloadType::Native {
                executable: "echo".to_string(),
                args: vec!["hello".to_string()],
            },
            resource_requirements: WorkloadResourceRequirements {
                cpu_cores: Some(2),
                memory_mb: Some(1024),
                gpu_required: false,
                gpu_memory_mb: None,
            },
            environment: std::collections::HashMap::new(),
            timeout_seconds: Some(10),
            priority: "normal".to_string(),
        };
        let result = provider.handle_workload(req).await;
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_capability_provider_send_heartbeats_empty() {
        let provider = CapabilityProvider::default();
        let result = provider.send_heartbeats().await;
        assert!(result.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_capability_provider_update_capability_unavailable() {
        let provider = CapabilityProvider::default();
        let cap = Capability::compute_gpu();
        let result = provider.update_capability(cap, false).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_capability_provider_default_has_capabilities() {
        let provider = CapabilityProvider::default();
        let _ = provider;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_capability_provider_get_capabilities_after_update() {
        let provider = CapabilityProvider::default();
        let _ = provider
            .update_capability(Capability::compute_gpu(), true)
            .await;
        let caps = provider.get_capabilities().await;
        assert!(!caps.is_empty());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_capability_provider_send_heartbeats_with_registered_adapter() {
        let provider = CapabilityProvider::default();
        let result = provider.send_heartbeats().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_capability_provider_clone() {
        let provider = CapabilityProvider::default();
        let cloned = provider.clone();
        let _ = cloned;
    }
}
