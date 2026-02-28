//! Resource leak detection and automatic cleanup.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;
use tracing::{error, warn};
use uuid::Uuid;

use crate::resources::ResourceRequirements;

/// Resource allocation tracking
#[derive(Debug, Clone)]
pub struct ResourceAllocation {
    pub id: Uuid,
    pub resource_type: String,
    pub allocated_at: Instant,
    pub requirements: ResourceRequirements,
    pub owner: String,
    pub last_accessed: Instant,
}

/// Resource leak detector
pub struct ResourceLeakDetector {
    allocations: Arc<RwLock<HashMap<Uuid, ResourceAllocation>>>,
    leak_threshold: Duration,
    cleanup_interval: Duration,
}

impl ResourceLeakDetector {
    #[must_use]
    pub fn new(leak_threshold: Duration, cleanup_interval: Duration) -> Self {
        Self {
            allocations: Arc::new(RwLock::new(HashMap::new())),
            leak_threshold,
            cleanup_interval,
        }
    }

    pub async fn track_allocation(&self, allocation: ResourceAllocation) {
        let mut allocations = self.allocations.write().await;
        allocations.insert(allocation.id, allocation);
    }

    pub async fn update_access(&self, resource_id: Uuid) {
        let mut allocations = self.allocations.write().await;
        if let Some(allocation) = allocations.get_mut(&resource_id) {
            allocation.last_accessed = Instant::now();
        }
    }

    pub async fn remove_allocation(&self, resource_id: Uuid) {
        let mut allocations = self.allocations.write().await;
        allocations.remove(&resource_id);
    }

    pub async fn cleanup_leaked_resources(&self) -> Vec<ResourceAllocation> {
        let mut allocations = self.allocations.write().await;
        let now = Instant::now();
        let mut leaked = Vec::new();

        allocations.retain(|_, allocation| {
            if now.duration_since(allocation.last_accessed) > self.leak_threshold {
                warn!(
                    "Detected resource leak: {} ({}) allocated at {:?}",
                    allocation.id, allocation.resource_type, allocation.allocated_at
                );
                leaked.push(allocation.clone());
                false
            } else {
                true
            }
        });

        leaked
    }

    #[allow(clippy::unused_async)] // Spawns background task; async for API consistency
    pub async fn start_cleanup_task(&self) {
        let allocations = Arc::clone(&self.allocations);
        let leak_threshold = self.leak_threshold;
        let cleanup_interval = self.cleanup_interval;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);
            loop {
                interval.tick().await;
                let detector = ResourceLeakDetector {
                    allocations: Arc::clone(&allocations),
                    leak_threshold,
                    cleanup_interval,
                };
                let leaked = detector.cleanup_leaked_resources().await;
                if !leaked.is_empty() {
                    error!("Cleaned up {} leaked resources", leaked.len());
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::ResourceRequirements;
    use std::time::Duration;

    fn sample_resource_allocation(id: Uuid) -> ResourceAllocation {
        ResourceAllocation {
            id,
            resource_type: "test-resource".to_string(),
            allocated_at: Instant::now(),
            requirements: ResourceRequirements::default(),
            owner: "test-owner".to_string(),
            last_accessed: Instant::now(),
        }
    }

    #[test]
    fn test_resource_allocation_creation() {
        let id = Uuid::new_v4();
        let allocation = sample_resource_allocation(id);
        assert_eq!(allocation.id, id);
        assert_eq!(allocation.resource_type, "test-resource");
        assert_eq!(allocation.owner, "test-owner");
    }

    #[tokio::test]
    async fn test_resource_leak_detector_new() {
        let detector = ResourceLeakDetector::new(Duration::from_secs(300), Duration::from_secs(60));
        let leaked = detector.cleanup_leaked_resources().await;
        assert!(leaked.is_empty());
    }

    #[tokio::test]
    async fn test_track_and_remove_allocation() {
        let detector = ResourceLeakDetector::new(Duration::from_secs(300), Duration::from_secs(60));
        let id = Uuid::new_v4();
        let allocation = sample_resource_allocation(id);
        detector.track_allocation(allocation).await;

        detector.remove_allocation(id).await;

        let leaked = detector.cleanup_leaked_resources().await;
        assert!(leaked.is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn test_update_access_extends_lifetime() {
        let detector =
            ResourceLeakDetector::new(Duration::from_millis(50), Duration::from_secs(60));
        let id = Uuid::new_v4();
        let allocation = sample_resource_allocation(id);
        detector.track_allocation(allocation).await;

        // Update access to keep it alive
        tokio::time::advance(Duration::from_millis(30)).await;
        detector.update_access(id).await;
        tokio::time::advance(Duration::from_millis(30)).await;

        let leaked = detector.cleanup_leaked_resources().await;
        // Should not have leaked yet due to update_access
        assert!(leaked.is_empty());
    }

    #[tokio::test]
    async fn test_cleanup_detects_stale_allocation() {
        let detector =
            ResourceLeakDetector::new(Duration::from_millis(25), Duration::from_secs(60));
        let id = Uuid::new_v4();
        let allocation = ResourceAllocation {
            id,
            resource_type: "stale".to_string(),
            allocated_at: Instant::now(),
            requirements: ResourceRequirements::default(),
            owner: "test".to_string(),
            last_accessed: Instant::now() - Duration::from_secs(1),
        };
        detector.track_allocation(allocation).await;

        let leaked = detector.cleanup_leaked_resources().await;
        assert_eq!(leaked.len(), 1);
        assert_eq!(leaked[0].id, id);
        assert_eq!(leaked[0].resource_type, "stale");
    }

    #[tokio::test]
    async fn test_remove_nonexistent_does_not_panic() {
        let detector = ResourceLeakDetector::new(Duration::from_secs(300), Duration::from_secs(60));
        detector.remove_allocation(Uuid::new_v4()).await;
    }
}
