// SPDX-License-Identifier: AGPL-3.0-or-later
//! Component registry and statistics

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use tracing::{info, warn};

use toadstool::{ToadStoolError, ToadStoolResult};

use super::core::{ComponentInterface, ComponentModelConfig};
use super::instances::{ComponentInstance, ComponentResourceUsage, ComponentState};

/// Component registry for managing component instances and interfaces
pub struct ComponentRegistry {
    /// Active component instances keyed by instance ID
    instances: Arc<RwLock<HashMap<String, ComponentInstance>>>,
    /// Component model configuration (limits, timeouts)
    config: ComponentModelConfig,
    /// Registered component interfaces by name
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
        let mut interfaces = self.interfaces.write().unwrap_or_else(|e| e.into_inner());

        if interfaces.contains_key(&interface.name) {
            warn!("Overriding existing interface: {}", interface.name);
        }

        let interface_name = interface.name.clone();
        interfaces.insert(interface_name.clone(), interface);
        drop(interfaces);
        info!("Registered component interface: {}", interface_name);
        Ok(())
    }

    /// Create a new component instance
    pub async fn create_instance(&self, interface_name: &str) -> ToadStoolResult<String> {
        let instances = self.instances.read().unwrap_or_else(|e| e.into_inner());

        if instances.len() >= self.config.max_instances {
            return Err(ToadStoolError::resource(
                "Maximum component instances reached".to_string(),
            ));
        }
        drop(instances);

        let interface = self
            .interfaces
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(interface_name)
            .ok_or_else(|| {
                ToadStoolError::not_found(format!("Interface not found: {interface_name}"))
            })?
            .clone();

        let instance_id = uuid::Uuid::new_v4().to_string();
        let mut instance_interfaces = HashMap::new();
        instance_interfaces.insert(interface_name.to_string(), interface);

        let instance = ComponentInstance {
            id: instance_id.clone(),
            interfaces: instance_interfaces,
            state: ComponentState::Initializing,
            created_at: std::time::SystemTime::now(),
            resource_usage: ComponentResourceUsage::default(),
        };

        self.instances
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(instance_id.clone(), instance);

        info!("Created component instance: {}", instance_id);
        Ok(instance_id)
    }

    /// Get component instance
    pub async fn get_instance(&self, instance_id: &str) -> ToadStoolResult<ComponentInstance> {
        let instances = self.instances.read().unwrap_or_else(|e| e.into_inner());
        let result = instances.get(instance_id).map_or_else(
            || {
                Err(ToadStoolError::not_found(format!(
                    "Component instance not found: {instance_id}"
                )))
            },
            |instance| {
                Ok(ComponentInstance {
                    id: instance.id.clone(),
                    interfaces: instance.interfaces.clone(),
                    state: instance.state.clone(),
                    created_at: instance.created_at,
                    resource_usage: instance.resource_usage.clone(),
                })
            },
        );
        drop(instances);
        result
    }

    /// Update component state
    pub async fn update_state(
        &self,
        instance_id: &str,
        state: ComponentState,
    ) -> ToadStoolResult<()> {
        let mut instances = self.instances.write().unwrap_or_else(|e| e.into_inner());

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
        let mut instances = self.instances.write().unwrap_or_else(|e| e.into_inner());

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
        let instances = self.instances.read().unwrap_or_else(|e| e.into_inner());
        instances.keys().cloned().collect()
    }

    /// Get component statistics
    pub async fn get_stats(&self) -> ComponentStats {
        let instances = self.instances.read().unwrap_or_else(|e| e.into_inner());
        let interfaces = self.interfaces.read().unwrap_or_else(|e| e.into_inner());

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
        drop(instances);
        drop(interfaces);
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
    /// Count of instances in initializing state
    pub initializing_instances: usize,
    /// Count of instances in ready state
    pub ready_instances: usize,
    /// Count of instances in running state
    pub running_instances: usize,
    /// Count of instances in failed state
    pub failed_instances: usize,
    /// Count of instances in terminating state
    pub terminating_instances: usize,
    /// Total memory used by all instances (bytes)
    pub total_memory_bytes: u64,
    /// Total CPU time consumed (ms)
    pub total_cpu_time_ms: u64,
    /// Total function invocations across instances
    pub total_function_calls: u64,
}

#[cfg(test)]
mod tests {
    use super::super::core::{ComponentInterface, ComponentModelConfig};
    use super::*;

    fn sample_interface(name: &str, version: &str) -> ComponentInterface {
        ComponentInterface {
            name: name.to_string(),
            version: version.to_string(),
            exports: vec![],
            imports: vec![],
            types: vec![],
        }
    }

    #[tokio::test]
    async fn register_interface_then_create_instance() {
        let reg = ComponentRegistry::new(ComponentModelConfig {
            max_instances: 10,
            ..Default::default()
        });
        reg.register_interface(sample_interface("api", "1"))
            .await
            .unwrap();
        let id = reg.create_instance("api").await.unwrap();
        let inst = reg.get_instance(&id).await.unwrap();
        assert_eq!(inst.interfaces.get("api").unwrap().version, "1");
    }

    #[tokio::test]
    async fn register_interface_overwrites_same_name() {
        let reg = ComponentRegistry::new(ComponentModelConfig {
            max_instances: 5,
            ..Default::default()
        });
        reg.register_interface(sample_interface("api", "1"))
            .await
            .unwrap();
        reg.register_interface(sample_interface("api", "2"))
            .await
            .unwrap();
        let id = reg.create_instance("api").await.unwrap();
        let inst = reg.get_instance(&id).await.unwrap();
        assert_eq!(inst.interfaces["api"].version, "2");
    }

    #[tokio::test]
    async fn create_instance_unknown_interface_errors() {
        let reg = ComponentRegistry::new(ComponentModelConfig::default());
        let err = reg.create_instance("missing").await.unwrap_err();
        assert!(err.to_string().contains("Interface not found"));
    }

    #[tokio::test]
    async fn create_instance_respects_max_instances() {
        let reg = ComponentRegistry::new(ComponentModelConfig {
            max_instances: 1,
            ..Default::default()
        });
        reg.register_interface(sample_interface("api", "1"))
            .await
            .unwrap();
        reg.create_instance("api").await.unwrap();
        let err = reg.create_instance("api").await.unwrap_err();
        assert!(err.to_string().contains("Maximum component instances"));
    }

    #[tokio::test]
    async fn remove_instance_deregistration() {
        let reg = ComponentRegistry::new(ComponentModelConfig {
            max_instances: 5,
            ..Default::default()
        });
        reg.register_interface(sample_interface("api", "1"))
            .await
            .unwrap();
        let id = reg.create_instance("api").await.unwrap();
        reg.remove_instance(&id).await.unwrap();
        assert!(reg.get_instance(&id).await.is_err());
        let err = reg.remove_instance(&id).await.unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn get_stats_counts_states_and_interfaces() {
        let reg = ComponentRegistry::new(ComponentModelConfig {
            max_instances: 20,
            ..Default::default()
        });
        reg.register_interface(sample_interface("a", "1"))
            .await
            .unwrap();
        reg.register_interface(sample_interface("b", "1"))
            .await
            .unwrap();
        let id = reg.create_instance("a").await.unwrap();
        reg.update_state(&id, ComponentState::Running)
            .await
            .unwrap();
        let stats = reg.get_stats().await;
        assert_eq!(stats.total_interfaces, 2);
        assert_eq!(stats.total_instances, 1);
        assert_eq!(stats.running_instances, 1);
        assert_eq!(stats.initializing_instances, 0);
    }
}
