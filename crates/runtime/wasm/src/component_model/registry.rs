//! Component registry and statistics

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

use toadstool::{ToadStoolError, ToadStoolResult};

use super::core::{ComponentInterface, ComponentModelConfig};
use super::instances::{ComponentInstance, ComponentResourceUsage, ComponentState};

/// Component registry for managing component instances
pub struct ComponentRegistry {
    /// Active component instances
    instances: Arc<RwLock<HashMap<String, ComponentInstance>>>,
    /// Component model configuration
    config: ComponentModelConfig,
    /// Interface registry
    interfaces: Arc<RwLock<HashMap<String, ComponentInterface>>>,
}

impl ComponentRegistry {
    /// Create a new component registry
    #[must_use]
    pub fn new(config: ComponentModelConfig) -> Self {
        Self {
            instances: Arc::new(RwLock::new(HashMap::new())),
            config,
            interfaces: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a component interface
    pub async fn register_interface(&self, interface: ComponentInterface) -> ToadStoolResult<()> {
        let mut interfaces = self.interfaces.write().await;

        if interfaces.contains_key(&interface.name) {
            warn!("Overriding existing interface: {}", interface.name);
        }

        let interface_name = interface.name.clone();
        interfaces.insert(interface_name.clone(), interface);
        info!("Registered component interface: {}", interface_name);
        Ok(())
    }

    /// Create a new component instance
    pub async fn create_instance(&self, interface_name: &str) -> ToadStoolResult<String> {
        let instances = self.instances.read().await;

        if instances.len() >= self.config.max_instances {
            return Err(ToadStoolError::resource(
                "Maximum component instances reached".to_string(),
            ));
        }
        drop(instances);

        let interfaces = self.interfaces.read().await;
        let interface = interfaces.get(interface_name).ok_or_else(|| {
            ToadStoolError::not_found(format!("Interface not found: {interface_name}"))
        })?;

        let instance_id = uuid::Uuid::new_v4().to_string();
        let mut instance_interfaces = HashMap::new();
        instance_interfaces.insert(interface_name.to_string(), interface.clone());

        let instance = ComponentInstance {
            id: instance_id.clone(),
            interfaces: instance_interfaces,
            state: ComponentState::Initializing,
            created_at: std::time::SystemTime::now(),
            resource_usage: ComponentResourceUsage::default(),
        };

        let mut instances = self.instances.write().await;
        instances.insert(instance_id.clone(), instance);

        info!("Created component instance: {}", instance_id);
        Ok(instance_id)
    }

    /// Get component instance
    pub async fn get_instance(&self, instance_id: &str) -> ToadStoolResult<ComponentInstance> {
        let instances = self.instances.read().await;
        if let Some(instance) = instances.get(instance_id) {
            // Create a copy of the instance data without using Clone
            Ok(ComponentInstance {
                id: instance.id.clone(),
                interfaces: instance.interfaces.clone(),
                state: instance.state.clone(),
                created_at: instance.created_at,
                resource_usage: instance.resource_usage.clone(),
            })
        } else {
            Err(ToadStoolError::not_found(format!(
                "Component instance not found: {instance_id}"
            )))
        }
    }

    /// Update component state
    pub async fn update_state(
        &self,
        instance_id: &str,
        state: ComponentState,
    ) -> ToadStoolResult<()> {
        let mut instances = self.instances.write().await;

        if let Some(instance) = instances.get_mut(instance_id) {
            instance.state = state;
            Ok(())
        } else {
            Err(ToadStoolError::not_found(format!(
                "Component instance not found: {instance_id}"
            )))
        }
    }

    /// Remove component instance
    pub async fn remove_instance(&self, instance_id: &str) -> ToadStoolResult<()> {
        let mut instances = self.instances.write().await;

        if instances.remove(instance_id).is_some() {
            info!("Removed component instance: {}", instance_id);
            Ok(())
        } else {
            Err(ToadStoolError::not_found(format!(
                "Component instance not found: {instance_id}"
            )))
        }
    }

    /// Get all active instances
    pub async fn get_active_instances(&self) -> Vec<String> {
        let instances = self.instances.read().await;
        instances.keys().cloned().collect()
    }

    /// Get component statistics
    pub async fn get_stats(&self) -> ComponentStats {
        let instances = self.instances.read().await;
        let interfaces = self.interfaces.read().await;

        let mut stats = ComponentStats {
            total_instances: instances.len(),
            total_interfaces: interfaces.len(),
            ..Default::default()
        };

        // Count instances by state
        for instance in instances.values() {
            match instance.state {
                ComponentState::Initializing => stats.initializing_instances += 1,
                ComponentState::Ready => stats.ready_instances += 1,
                ComponentState::Running => stats.running_instances += 1,
                ComponentState::Failed { .. } => stats.failed_instances += 1,
                ComponentState::Terminating => stats.terminating_instances += 1,
            }

            // Aggregate resource usage
            stats.total_memory_bytes += instance.resource_usage.memory_bytes;
            stats.total_cpu_time_ms += instance.resource_usage.cpu_time_ms;
            stats.total_function_calls += instance.resource_usage.function_calls;
        }

        stats
    }
}

/// Component registry statistics
#[derive(Debug, Default)]
pub struct ComponentStats {
    /// Total number of instances
    pub total_instances: usize,
    /// Total number of interfaces
    pub total_interfaces: usize,
    /// Instances by state
    pub initializing_instances: usize,
    pub ready_instances: usize,
    pub running_instances: usize,
    pub failed_instances: usize,
    pub terminating_instances: usize,
    /// Aggregate resource usage
    pub total_memory_bytes: u64,
    pub total_cpu_time_ms: u64,
    pub total_function_calls: u64,
}
