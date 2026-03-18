// SPDX-License-Identifier: AGPL-3.0-or-later
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use toadstool::ToadStoolResult;
use tokio::sync::RwLock;

use crate::types::{
    InstanceStatus, ProcessHandle, ResourceAllocation, ResourceLimits, ToadStoolHostingConfig,
};
use crate::universal::RecursiveHostingConfig;

/// Recursive hosting manager for hosting child `ToadStool` instances
pub struct RecursiveHostingManager {
    /// Configuration
    _config: RecursiveHostingConfig,
    /// Active child instances
    child_instances: Arc<RwLock<HashMap<String, ChildToadStoolInstance>>>,
    /// Resource allocator for children
    _resource_allocator: Arc<ChildResourceAllocator>,
    /// Inter-instance communication
    _inter_instance_comm: Arc<InterInstanceCommunication>,
}

/// Child `ToadStool` instance
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
    pub started_at: SystemTime,
}

/// Child resource allocator
pub struct ChildResourceAllocator {
    _allocations: Arc<RwLock<HashMap<String, ResourceAllocation>>>,
    _total_resources: ResourceLimits,
}

/// Inter-instance communication
pub struct InterInstanceCommunication {
    _channels: Arc<RwLock<HashMap<String, CommunicationChannel>>>,
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
            _config: config,
            child_instances: Arc::new(RwLock::new(HashMap::new())),
            _resource_allocator: Arc::new(ChildResourceAllocator::new()),
            _inter_instance_comm: Arc::new(InterInstanceCommunication::new()),
        })
    }

    pub async fn create_child_instance(
        &self,
        toadstool_config: ToadStoolHostingConfig,
    ) -> ToadStoolResult<ChildToadStoolInstance> {
        let instance_id = uuid::Uuid::new_v4().to_string();

        // Use environment-aware configuration
        let port: u16 = std::env::var("TOADSTOOL_API_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8084);
        let config = toadstool_config::env_config::EnvironmentConfig::from_env();
        let host = &config.network.bind_address;

        let instance = ChildToadStoolInstance {
            instance_id: instance_id.clone(),
            process_handle: ProcessHandle::default(),
            resource_allocation: toadstool_config.resource_allocation.unwrap_or_default(),
            endpoint: format!("http://{host}:{port}/{instance_id}"),
            status: InstanceStatus::Starting,
            started_at: SystemTime::now(),
        };

        {
            let mut instances = self.child_instances.write().await;
            instances.insert(instance_id.clone(), instance.clone());
        }

        Ok(instance)
    }
}

impl ChildResourceAllocator {
    #[must_use]
    pub fn new() -> Self {
        Self {
            _allocations: Arc::new(RwLock::new(HashMap::new())),
            _total_resources: ResourceLimits::default(),
        }
    }
}

impl Default for ChildResourceAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl InterInstanceCommunication {
    #[must_use]
    pub fn new() -> Self {
        Self {
            _channels: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InterInstanceCommunication {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    #![allow(unsafe_code)] // env::set_var/remove_var are unsafe in Rust 2024; test-only usage

    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_child_resource_allocator_new() {
        let allocator = ChildResourceAllocator::new();
        let _ = allocator;
    }

    #[test]
    fn test_child_resource_allocator_default() {
        let allocator = ChildResourceAllocator::default();
        let _ = allocator;
    }

    #[test]
    fn test_inter_instance_communication_new() {
        let comm = InterInstanceCommunication::new();
        let _ = comm;
    }

    #[test]
    fn test_inter_instance_communication_default() {
        let comm = InterInstanceCommunication::default();
        let _ = comm;
    }

    #[test]
    fn test_communication_channel_debug_clone() {
        let channel = CommunicationChannel {
            channel_id: "ch-1".to_string(),
            endpoint: "http://127.0.0.1:8080".to_string(),
            last_activity: std::time::SystemTime::now(),
        };
        let cloned = channel.clone();
        assert_eq!(channel.channel_id, cloned.channel_id);
        assert!(format!("{:?}", channel).contains("ch-1"));
    }

    #[test]
    fn test_child_toadstool_instance_debug_clone() {
        let instance = ChildToadStoolInstance {
            instance_id: "inst-1".to_string(),
            process_handle: ProcessHandle::default(),
            resource_allocation: ResourceAllocation::default(),
            endpoint: "http://127.0.0.1:8084/inst-1".to_string(),
            status: InstanceStatus::Starting,
            started_at: SystemTime::now(),
        };
        let cloned = instance.clone();
        assert_eq!(instance.instance_id, cloned.instance_id);
        assert_eq!(instance.endpoint, cloned.endpoint);
    }

    #[tokio::test]
    async fn test_recursive_hosting_manager_new() {
        let config = RecursiveHostingConfig {
            enabled: true,
            current_depth: 0,
            max_depth: 3,
            parent_toadstool: None,
            child_toadstools: vec![],
            child_resource_allocation: crate::types::ResourceAllocationStrategy::Fair,
        };
        let manager = RecursiveHostingManager::new(config).await;
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_recursive_hosting_manager_create_child_instance() {
        let config = RecursiveHostingConfig {
            enabled: true,
            current_depth: 0,
            max_depth: 3,
            parent_toadstool: None,
            child_toadstools: vec![],
            child_resource_allocation: crate::types::ResourceAllocationStrategy::Fair,
        };
        let manager = RecursiveHostingManager::new(config)
            .await
            .expect("manager creation");

        let hosting_config = ToadStoolHostingConfig {
            enabled: true,
            mode: "child".to_string(),
            resource_limits: HashMap::new(),
            security_settings: HashMap::new(),
            resource_allocation: Some(ResourceAllocation::default()),
        };

        let result = manager.create_child_instance(hosting_config).await;
        assert!(result.is_ok());
        let instance = result.expect("instance");
        assert!(!instance.instance_id.is_empty());
        assert!(instance.endpoint.contains(&instance.instance_id));
        assert!(matches!(instance.status, InstanceStatus::Starting));
    }

    #[tokio::test]
    async fn test_recursive_hosting_manager_create_child_with_custom_port() {
        // SAFETY: Test-only; no other threads access env vars during this test
        unsafe { std::env::set_var("TOADSTOOL_API_PORT", "9999") };
        let config = RecursiveHostingConfig {
            enabled: true,
            current_depth: 0,
            max_depth: 3,
            parent_toadstool: None,
            child_toadstools: vec![],
            child_resource_allocation: crate::types::ResourceAllocationStrategy::Fair,
        };
        let manager = RecursiveHostingManager::new(config)
            .await
            .expect("manager creation");

        let hosting_config = ToadStoolHostingConfig {
            enabled: true,
            mode: "child".to_string(),
            resource_limits: HashMap::new(),
            security_settings: HashMap::new(),
            resource_allocation: None,
        };

        let result = manager.create_child_instance(hosting_config).await;
        assert!(result.is_ok());
        let instance = result.expect("instance");
        assert!(instance.endpoint.contains("9999"));
        // SAFETY: Test-only; no other threads access env vars during this test
        unsafe { std::env::remove_var("TOADSTOOL_API_PORT") };
    }
}
