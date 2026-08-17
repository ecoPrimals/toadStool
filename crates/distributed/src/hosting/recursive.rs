// SPDX-License-Identifier: AGPL-3.0-or-later
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::SystemTime;
use toadstool::ToadStoolResult;
use toadstool_common::constants::network::HTTP_PROTOCOL;

use crate::types::{
    InstanceStatus, ProcessHandle, ResourceAllocation, ResourceAllocationStrategy, ResourceLimits,
    ToadStoolHostingConfig,
};

/// Recursive hosting configuration
#[derive(Debug, Clone)]
pub struct RecursiveHostingConfig {
    /// Enable recursive hosting
    pub enabled: bool,
    /// Current depth level
    pub current_depth: u32,
    /// Maximum depth allowed
    pub max_depth: u32,
    /// Parent `ToadStool` if hosted
    pub parent_toadstool: Option<String>,
    /// Child `ToadStools` being hosted
    pub child_toadstools: Vec<String>,
    /// Resource allocation for children
    pub child_resource_allocation: ResourceAllocationStrategy,
}

impl Default for RecursiveHostingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            current_depth: 0,
            max_depth: crate::common::defaults::MAX_HOSTING_DEPTH,
            parent_toadstool: None,
            child_toadstools: Vec::new(),
            child_resource_allocation: ResourceAllocationStrategy::Fair,
        }
    }
}

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

/// Bidirectional channel for inter-instance communication.
#[derive(Debug, Clone)]
pub struct CommunicationChannel {
    /// Unique channel identifier.
    pub channel_id: String,
    /// Remote endpoint for this channel.
    pub endpoint: String,
    /// Last activity timestamp for liveness.
    pub last_activity: std::time::SystemTime,
}

impl RecursiveHostingManager {
    /// Creates a recursive hosting manager with the given config.
    pub async fn new(config: RecursiveHostingConfig) -> ToadStoolResult<Self> {
        Ok(Self {
            _config: config,
            child_instances: Arc::new(RwLock::new(HashMap::new())),
            _resource_allocator: Arc::new(ChildResourceAllocator::new()),
            _inter_instance_comm: Arc::new(InterInstanceCommunication::new()),
        })
    }

    /// Spawns a child ToadStool instance with the given hosting config.
    pub async fn create_child_instance(
        &self,
        toadstool_config: ToadStoolHostingConfig,
    ) -> ToadStoolResult<ChildToadStoolInstance> {
        let instance_id = uuid::Uuid::new_v4().to_string();

        // Use environment-aware configuration
        let port: u16 = toadstool_config::ports::daemon_port();
        let config = toadstool_config::env_config::EnvironmentConfig::from_env();
        let host = &config.network.bind_address;

        let base_url = format!("{HTTP_PROTOCOL}{host}:{port}");
        let instance = ChildToadStoolInstance {
            instance_id: instance_id.clone(),
            process_handle: ProcessHandle::default(),
            resource_allocation: toadstool_config.resource_allocation.unwrap_or_default(),
            endpoint: format!("{base_url}/{instance_id}"),
            status: InstanceStatus::Starting,
            started_at: SystemTime::now(),
        };

        {
            let mut instances = self
                .child_instances
                .write()
                .unwrap_or_else(|e| e.into_inner());
            instances.insert(instance_id.clone(), instance.clone());
        }

        Ok(instance)
    }
}

impl ChildResourceAllocator {
    /// Creates a child resource allocator with default limits.
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
    /// Creates an empty inter-instance communication manager.
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
    use super::*;
    use std::collections::HashMap;
    use toadstool_common::constants::{LOCALHOST_IPV4, network::HTTP_PROTOCOL};

    /// Loopback host for recursive hosting unit tests (not a fixed deployment address).
    const TEST_LOOPBACK_HOST: &str = LOCALHOST_IPV4;
    /// Arbitrary port for channel/instance tests that do not exercise binding.
    const TEST_HTTP_PORT: u16 = 8080;
    /// Default child API port aligned with `TOADSTOOL_API_PORT` fallback in production paths.
    const TEST_CHILD_API_PORT: u16 = 8084;

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
            endpoint: format!("{HTTP_PROTOCOL}{TEST_LOOPBACK_HOST}:{TEST_HTTP_PORT}"),
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
            endpoint: format!("{HTTP_PROTOCOL}{TEST_LOOPBACK_HOST}:{TEST_CHILD_API_PORT}/inst-1"),
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

    #[test]
    fn test_recursive_hosting_manager_create_child_with_custom_port() {
        temp_env::with_var("TOADSTOOL_DAEMON_API_PORT", Some("9999"), || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let config = RecursiveHostingConfig {
                enabled: true,
                current_depth: 0,
                max_depth: 3,
                parent_toadstool: None,
                child_toadstools: vec![],
                child_resource_allocation: crate::types::ResourceAllocationStrategy::Fair,
            };
            let manager = rt
                .block_on(RecursiveHostingManager::new(config))
                .expect("manager creation");

            let hosting_config = ToadStoolHostingConfig {
                enabled: true,
                mode: "child".to_string(),
                resource_limits: HashMap::new(),
                security_settings: HashMap::new(),
                resource_allocation: None,
            };

            let result = rt.block_on(manager.create_child_instance(hosting_config));
            assert!(result.is_ok());
            let instance = result.expect("instance");
            assert!(instance.endpoint.contains("9999"));
        });
    }
}
