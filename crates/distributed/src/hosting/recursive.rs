use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use toadstool::ToadStoolResult;
use tokio::sync::RwLock;

use crate::types::*;
use crate::universal::RecursiveHostingConfig;

/// Recursive hosting manager for hosting child ToadStool instances
pub struct RecursiveHostingManager {
    /// Configuration
    config: RecursiveHostingConfig,
    /// Active child instances
    child_instances: Arc<RwLock<HashMap<String, ChildToadStoolInstance>>>,
    /// Resource allocator for children
    resource_allocator: Arc<ChildResourceAllocator>,
    /// Inter-instance communication
    inter_instance_comm: Arc<InterInstanceCommunication>,
}

/// Child ToadStool instance
#[derive(Debug, Clone)]
pub struct ChildToadStoolInstance {
    /// Instance identification
    pub instance_id: String,
    /// Process handle
    pub process_handle: ProcessHandle,
    /// Resource allocation
    pub resource_allocation: ResourceAllocation,
    /// Communication endpoint
    pub endpoint: String,
    /// Status
    pub status: InstanceStatus,
    /// Started timestamp
    pub started_at: DateTime<Utc>,
}

/// Child resource allocator
pub struct ChildResourceAllocator {
    allocations: Arc<RwLock<HashMap<String, ResourceAllocation>>>,
    total_resources: ResourceLimits,
}

/// Inter-instance communication
pub struct InterInstanceCommunication {
    channels: Arc<RwLock<HashMap<String, CommunicationChannel>>>,
}

/// Communication channel
#[derive(Debug, Clone)]
pub struct CommunicationChannel {
    pub channel_id: String,
    pub endpoint: String,
    pub last_activity: std::time::SystemTime,
}

impl RecursiveHostingManager {
    pub async fn new(config: RecursiveHostingConfig) -> ToadStoolResult<Self> {
        Ok(Self {
            config,
            child_instances: Arc::new(RwLock::new(HashMap::new())),
            resource_allocator: Arc::new(ChildResourceAllocator::new()),
            inter_instance_comm: Arc::new(InterInstanceCommunication::new()),
        })
    }

    pub async fn create_child_instance(
        &self,
        toadstool_config: ToadStoolHostingConfig,
    ) -> ToadStoolResult<ChildToadStoolInstance> {
        let instance_id = uuid::Uuid::new_v4().to_string();

        let instance = ChildToadStoolInstance {
            instance_id: instance_id.clone(),
            process_handle: ProcessHandle::default(),
            resource_allocation: toadstool_config
                .resource_allocation
                .unwrap_or_default(),
            endpoint: format!("http://localhost:8080/{instance_id}"),
            status: InstanceStatus::Starting,
            started_at: Utc::now(),
        };

        let mut instances = self.child_instances.write().await;
        instances.insert(instance_id, instance.clone());

        Ok(instance)
    }
}

impl ChildResourceAllocator {
    pub fn new() -> Self {
        Self {
            allocations: Arc::new(RwLock::new(HashMap::new())),
            total_resources: ResourceLimits::default(),
        }
    }
}

impl Default for ChildResourceAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl InterInstanceCommunication {
    pub fn new() -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InterInstanceCommunication {
    fn default() -> Self {
        Self::new()
    }
}
