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
