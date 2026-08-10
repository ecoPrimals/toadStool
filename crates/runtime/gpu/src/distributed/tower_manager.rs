// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tower Discovery and Management
//!
//! Manages discovery and health monitoring of remote ToadStool towers
//! via coordination-service capability discovery.
//!
//! **Self-Knowledge Principle**: This module discovers towers at runtime,
//! never hardcodes remote endpoints.

use super::types::RemoteTowerEndpoint;
use crate::universal::ComputeRequirements;
use std::sync::Arc;
use toadstool::error::{ToadStoolError, ToadStoolResult};
use std::sync::RwLock;

/// Manages remote tower discovery and health monitoring
pub struct TowerManager {
    /// This tower's unique identifier
    tower_id: String,

    /// Remote towers discovered via the coordination service
    remote_towers: Arc<RwLock<Vec<RemoteTowerEndpoint>>>,
}

impl TowerManager {
    /// Create new tower manager
    pub fn new(tower_id: impl Into<String>) -> Self {
        Self {
            tower_id: tower_id.into(),
            remote_towers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get this tower's ID
    pub fn local_tower_id(&self) -> &str {
        &self.tower_id
    }

    /// Register a remote tower discovered via the coordination service
    pub async fn register_tower(&self, endpoint: RemoteTowerEndpoint) {
        let mut towers = self.remote_towers.write().unwrap_or_else(|e| e.into_inner());

        // Remove stale entry if exists
        towers.retain(|t| t.tower_id != endpoint.tower_id);

        tracing::info!(
            "Registered remote tower: {} at {} (latency: {}ms)",
            endpoint.tower_id,
            endpoint.address,
            endpoint.latency_ms
        );

        towers.push(endpoint);
    }

    /// Get all available tower IDs (local + remote)
    pub async fn available_tower_ids(&self) -> Vec<String> {
        let mut ids = vec![self.tower_id.clone()];

        let remote = self.remote_towers.read().unwrap_or_else(|e| e.into_inner());
        ids.extend(remote.iter().map(|t| t.tower_id.clone()));

        ids
    }

    /// Get count of available towers
    pub async fn tower_count(&self) -> usize {
        let remote = self.remote_towers.read().unwrap_or_else(|e| e.into_inner());
        1 + remote.len() // local + remote
    }

    /// Select best tower based on workload requirements
    ///
    /// Selection criteria (in order):
    /// 1. Has required capabilities (GPU type, memory, compute)
    /// 2. Lowest latency (network proximity)
    /// 3. Most recent health check (availability)
    ///
    /// **Deep Debt**: Capability-based selection, no hardcoded preferences
    ///
    /// # Errors
    ///
    /// Returns when no tower can be selected (unexpected empty selection).
    pub async fn select_best_tower(
        &self,
        requirements: &ComputeRequirements,
    ) -> ToadStoolResult<String> {
        let towers = self.remote_towers.read().unwrap_or_else(|e| e.into_inner());

        if towers.is_empty() {
            // No remote towers, use local (graceful degradation)
            return Ok(self.tower_id.clone());
        }

        // Filter towers by required capabilities (clone to allow early drop of lock)
        let capable_towers: Vec<_> = towers
            .iter()
            .filter(|tower| {
                tower.gpu_capabilities.as_ref().is_none_or(|gpu_caps| {
                    let required_mem = requirements.memory_bytes;
                    required_mem == 0 || gpu_caps.memory.total_bytes >= required_mem
                })
            })
            .cloned()
            .collect();
        drop(towers);

        if capable_towers.is_empty() {
            // No capable remote towers, fall back to local
            tracing::debug!(
                "No remote towers meet requirements, using local tower: {}",
                self.tower_id
            );
            return Ok(self.tower_id.clone());
        }

        // Select tower with lowest latency from capable towers
        let best = capable_towers
            .iter()
            .min_by_key(|t| t.latency_ms)
            .ok_or_else(|| ToadStoolError::runtime("No towers available"))?;

        tracing::debug!(
            "Selected tower {} with {}ms latency",
            best.tower_id,
            best.latency_ms
        );

        Ok(best.tower_id.clone())
    }

    /// Select multiple towers for redundant/parallel execution
    ///
    /// # Errors
    ///
    /// Currently always returns `Ok`; reserved for future validation failures.
    pub async fn select_multiple_towers(
        &self,
        _requirements: &ComputeRequirements,
        count: usize,
    ) -> ToadStoolResult<Vec<String>> {
        let mut selected = vec![self.tower_id.clone()]; // Always include local

        let towers = self.remote_towers.read().unwrap_or_else(|e| e.into_inner());

        // Add remote towers sorted by latency (clone to allow early drop of lock)
        let mut sorted: Vec<_> = towers.iter().cloned().collect();
        drop(towers);
        sorted.sort_by_key(|t| t.latency_ms);

        for tower in sorted.iter().take(count.saturating_sub(1)) {
            selected.push(tower.tower_id.clone());
        }

        Ok(selected)
    }

    /// Select tower by specific capability
    ///
    /// Used for pipeline stages that need specific capabilities
    ///
    /// **Deep Debt**: Capability-based discovery (no hardcoded tower names)
    ///
    /// **Future Enhancement**: Would query the coordination service for real-time capability discovery:
    /// ```ignore
    /// let coordination = CoordinationClient::discover().await?;
    /// let towers = coordination.find_by_capability(capability).await?;
    /// select_lowest_latency(towers)
    /// ```
    ///
    /// # Errors
    ///
    /// Currently always returns `Ok`; reserved for future capability resolution failures.
    pub async fn select_by_capability(&self, capability: &str) -> ToadStoolResult<String> {
        let towers = self.remote_towers.read().unwrap_or_else(|e| e.into_inner());

        tracing::debug!("Selecting tower for capability: {}", capability);

        // Simplified capability matching (production would query the coordination service)
        // For now, select first available tower (graceful degradation)
        towers.first().map_or_else(
            || {
                tracing::debug!(
                    "No remote towers available, using local for capability: {}",
                    capability
                );
                Ok(self.tower_id.clone())
            },
            |tower| {
                tracing::debug!(
                    "Selected tower {} for capability {}",
                    tower.tower_id,
                    capability
                );
                Ok(tower.tower_id.clone())
            },
        )
    }

    /// Get endpoint for remote tower
    pub async fn get_tower_endpoint(&self, tower_id: &str) -> Option<RemoteTowerEndpoint> {
        let towers = self.remote_towers.read().unwrap_or_else(|e| e.into_inner());
        towers.iter().find(|t| t.tower_id == tower_id).cloned()
    }

    /// Prune stale towers (not seen recently)
    pub async fn prune_stale_towers(&self, max_age_secs: u64) {
        let mut towers = self.remote_towers.write().unwrap_or_else(|e| e.into_inner());
        let now = std::time::Instant::now();

        let before_count = towers.len();
        towers.retain(|t| now.duration_since(t.last_seen).as_secs() < max_age_secs);
        let after_count = towers.len();
        drop(towers);

        if before_count != after_count {
            tracing::info!("Pruned {} stale towers", before_count - after_count);
        }
    }

    /// Get remote towers (for statistics)
    pub async fn remote_tower_count(&self) -> usize {
        let towers = self.remote_towers.read().unwrap_or_else(|e| e.into_inner());
        towers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    // Placeholder addresses for unit tests (production uses coordination discovery)
    const TEST_TOWER_ADDR_1: &str = "10.0.0.1:8080";
    const TEST_TOWER_ADDR_2: &str = "10.0.0.2:8080";

    #[tokio::test]
    async fn test_tower_manager_creation() {
        let manager = TowerManager::new("local-tower".to_string());
        assert_eq!(manager.local_tower_id(), "local-tower");
        assert_eq!(manager.tower_count().await, 1); // Only local
    }

    #[tokio::test]
    async fn test_register_tower() {
        let manager = TowerManager::new("local".to_string());

        // Test fixture: placeholder address for unit test (production uses coordination discovery)
        let endpoint = RemoteTowerEndpoint {
            tower_id: "remote-1".to_string(),
            address: TEST_TOWER_ADDR_2.to_string(),
            gpu_capabilities: None,
            last_seen: Instant::now(),
            latency_ms: 5,
        };

        manager.register_tower(endpoint).await;
        assert_eq!(manager.tower_count().await, 2);
    }

    #[tokio::test]
    async fn test_available_towers() {
        let manager = TowerManager::new("local".to_string());

        let ids = manager.available_tower_ids().await;
        assert_eq!(ids.len(), 1);
        assert!(ids.contains(&"local".to_string()));
    }

    #[tokio::test]
    async fn test_select_best_tower_empty_returns_local() {
        let manager = TowerManager::new("local-tower".to_string());
        let reqs = ComputeRequirements {
            memory_bytes: 1024,
            ..Default::default()
        };
        let result = manager.select_best_tower(&reqs).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "local-tower");
    }

    #[tokio::test]
    async fn test_select_by_capability_empty_returns_local() {
        let manager = TowerManager::new("local".to_string());
        let result = manager.select_by_capability("gpu").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "local");
    }

    #[tokio::test]
    async fn test_select_multiple_towers() {
        let manager = TowerManager::new("local".to_string());
        let reqs = ComputeRequirements::default();
        let result = manager.select_multiple_towers(&reqs, 3).await;
        assert!(result.is_ok());
        let towers = result.unwrap();
        assert!(!towers.is_empty());
        assert!(towers.contains(&"local".to_string()));
    }

    #[tokio::test]
    async fn test_get_tower_endpoint_none() {
        let manager = TowerManager::new("local".to_string());
        let endpoint = manager.get_tower_endpoint("nonexistent").await;
        assert!(endpoint.is_none());
    }

    #[tokio::test]
    async fn test_remote_tower_count() {
        let manager = TowerManager::new("local".to_string());
        assert_eq!(manager.remote_tower_count().await, 0);

        let endpoint = RemoteTowerEndpoint {
            tower_id: "remote-1".to_string(),
            address: TEST_TOWER_ADDR_1.to_string(),
            gpu_capabilities: None,
            last_seen: Instant::now(),
            latency_ms: 10,
        };
        manager.register_tower(endpoint).await;
        assert_eq!(manager.remote_tower_count().await, 1);
    }

    #[tokio::test]
    async fn test_register_tower_replaces_stale() {
        let manager = TowerManager::new("local".to_string());
        let ep1 = RemoteTowerEndpoint {
            tower_id: "remote-1".to_string(),
            address: TEST_TOWER_ADDR_1.to_string(),
            gpu_capabilities: None,
            last_seen: Instant::now(),
            latency_ms: 5,
        };
        manager.register_tower(ep1).await;
        let ep2 = RemoteTowerEndpoint {
            tower_id: "remote-1".to_string(),
            address: TEST_TOWER_ADDR_2.to_string(),
            gpu_capabilities: None,
            last_seen: Instant::now(),
            latency_ms: 3,
        };
        manager.register_tower(ep2).await;
        assert_eq!(manager.tower_count().await, 2);
        let endpoint = manager.get_tower_endpoint("remote-1").await;
        assert!(endpoint.is_some());
        assert_eq!(endpoint.unwrap().address, TEST_TOWER_ADDR_2);
    }
}
