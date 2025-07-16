//! WebAssembly Component Model Support
//!
//! This module provides support for WebAssembly component model features,
//! enabling composable, interface-driven WebAssembly applications.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

use toadstool::{ToadStoolError, ToadStoolResult};

/// WebAssembly Component Model Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentModelConfig {
    /// Enable component model support
    pub enabled: bool,
    /// Maximum number of component instances
    pub max_instances: usize,
    /// Component linking timeout in milliseconds
    pub linking_timeout_ms: u64,
    /// Enable component composition
    pub composition_enabled: bool,
    /// Interface definition language support
    pub wit_support: bool,
}

impl Default for ComponentModelConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_instances: 1000,
            linking_timeout_ms: 5000,
            composition_enabled: true,
            wit_support: true,
        }
    }
}

/// WebAssembly Component Interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentInterface {
    /// Interface name
    pub name: String,
    /// Interface version
    pub version: String,
    /// Exported functions
    pub exports: Vec<InterfaceFunction>,
    /// Imported functions
    pub imports: Vec<InterfaceFunction>,
    /// Type definitions
    pub types: Vec<InterfaceType>,
}

/// Interface function definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceFunction {
    /// Function name
    pub name: String,
    /// Parameters
    pub params: Vec<InterfaceType>,
    /// Return type
    pub return_type: Option<InterfaceType>,
    /// Function documentation
    pub docs: Option<String>,
}

/// Interface type definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InterfaceType {
    /// Basic types
    Bool,
    U8,
    U16,
    U32,
    U64,
    S8,
    S16,
    S32,
    S64,
    F32,
    F64,
    String,
    /// Complex types
    List(Box<InterfaceType>),
    Record(Vec<(String, InterfaceType)>),
    Variant(Vec<(String, Option<InterfaceType>)>),
    Option(Box<InterfaceType>),
    Result(Box<InterfaceType>, Box<InterfaceType>),
    /// Custom types
    Custom(String),
}

/// Component instance
#[derive(Debug)]
pub struct ComponentInstance {
    /// Instance ID
    pub id: String,
    /// Component interfaces
    pub interfaces: HashMap<String, ComponentInterface>,
    /// Instance state
    pub state: ComponentState,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Resource usage
    pub resource_usage: ComponentResourceUsage,
}

/// Component state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComponentState {
    /// Component is being initialized
    Initializing,
    /// Component is ready for use
    Ready,
    /// Component is executing
    Running,
    /// Component has failed
    Failed { error: String },
    /// Component is shutting down
    Terminating,
}

/// Component resource usage tracking
#[derive(Debug, Default, Clone)]
pub struct ComponentResourceUsage {
    /// Memory usage in bytes
    pub memory_bytes: u64,
    /// CPU time in milliseconds
    pub cpu_time_ms: u64,
    /// Number of function calls
    pub function_calls: u64,
    /// Number of interface imports
    pub imports_count: u32,
    /// Number of interface exports
    pub exports_count: u32,
}

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
            created_at: chrono::Utc::now(),
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

/// Component linker for connecting components
pub struct ComponentLinker {
    /// Configuration
    _config: ComponentModelConfig,
    /// Component registry
    registry: Arc<ComponentRegistry>,
}

impl ComponentLinker {
    /// Create a new component linker
    pub fn new(config: ComponentModelConfig, registry: Arc<ComponentRegistry>) -> Self {
        Self {
            _config: config,
            registry,
        }
    }

    /// Link components together
    pub async fn link_components(
        &self,
        consumer_id: &str,
        provider_id: &str,
        interface_name: &str,
    ) -> ToadStoolResult<()> {
        let consumer = self.registry.get_instance(consumer_id).await?;
        let provider = self.registry.get_instance(provider_id).await?;

        // Verify interface compatibility
        if !consumer.interfaces.contains_key(interface_name) {
            return Err(ToadStoolError::validation(format!(
                "Consumer {consumer_id} does not have interface: {interface_name}"
            )));
        }

        if !provider.interfaces.contains_key(interface_name) {
            return Err(ToadStoolError::validation(format!(
                "Provider {provider_id} does not have interface: {interface_name}"
            )));
        }

        // Perform the linking (simplified - in reality would involve runtime linking)
        info!(
            "Linked components: {} -> {} via {}",
            consumer_id, provider_id, interface_name
        );
        Ok(())
    }

    /// Validate component composition
    pub async fn validate_composition(&self, component_ids: &[String]) -> ToadStoolResult<bool> {
        if component_ids.is_empty() {
            return Ok(true);
        }

        // Get all components
        let mut components = Vec::new();
        for id in component_ids {
            let component = self.registry.get_instance(id).await?;
            components.push(component);
        }

        // Validate that all required imports are satisfied
        for component in &components {
            for interface in component.interfaces.values() {
                for import in &interface.imports {
                    let mut import_satisfied = false;

                    // Check if any other component exports this function
                    for other_component in &components {
                        if other_component.id == component.id {
                            continue;
                        }

                        for other_interface in other_component.interfaces.values() {
                            if other_interface
                                .exports
                                .iter()
                                .any(|exp| exp.name == import.name)
                            {
                                import_satisfied = true;
                                break;
                            }
                        }

                        if import_satisfied {
                            break;
                        }
                    }

                    if !import_satisfied {
                        warn!(
                            "Unsatisfied import: {} in component {}",
                            import.name, component.id
                        );
                        return Ok(false);
                    }
                }
            }
        }

        Ok(true)
    }
}

/// Component model support trait
#[async_trait]
pub trait ComponentModelSupport {
    /// Check if component model is supported
    fn supports_component_model(&self) -> bool;

    /// Get component model configuration
    fn get_component_config(&self) -> &ComponentModelConfig;

    /// Create component instance
    async fn create_component_instance(&self, interface_name: &str) -> ToadStoolResult<String>;

    /// Execute component function
    async fn execute_component_function(
        &self,
        instance_id: &str,
        function_name: &str,
        args: &[ComponentValue],
    ) -> ToadStoolResult<ComponentValue>;
}

/// Component value type for function parameters and returns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComponentValue {
    Bool(bool),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    S8(i8),
    S16(i16),
    S32(i32),
    S64(i64),
    F32(f32),
    F64(f64),
    String(String),
    List(Vec<ComponentValue>),
    Record(HashMap<String, ComponentValue>),
    Option(Option<Box<ComponentValue>>),
    Variant(String, Option<Box<ComponentValue>>),
}

impl ComponentValue {
    /// Check if value matches the expected type
    pub fn matches_type(&self, expected: &InterfaceType) -> bool {
        match (self, expected) {
            (ComponentValue::Bool(_), InterfaceType::Bool) => true,
            (ComponentValue::U8(_), InterfaceType::U8) => true,
            (ComponentValue::U16(_), InterfaceType::U16) => true,
            (ComponentValue::U32(_), InterfaceType::U32) => true,
            (ComponentValue::U64(_), InterfaceType::U64) => true,
            (ComponentValue::S8(_), InterfaceType::S8) => true,
            (ComponentValue::S16(_), InterfaceType::S16) => true,
            (ComponentValue::S32(_), InterfaceType::S32) => true,
            (ComponentValue::S64(_), InterfaceType::S64) => true,
            (ComponentValue::F32(_), InterfaceType::F32) => true,
            (ComponentValue::F64(_), InterfaceType::F64) => true,
            (ComponentValue::String(_), InterfaceType::String) => true,
            (ComponentValue::List(values), InterfaceType::List(element_type)) => {
                values.iter().all(|v| v.matches_type(element_type))
            }
            (ComponentValue::Option(Some(value)), InterfaceType::Option(inner_type)) => {
                value.matches_type(inner_type)
            }
            (ComponentValue::Option(None), InterfaceType::Option(_)) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_component_registry_creation() {
        let config = ComponentModelConfig::default();
        let registry = ComponentRegistry::new(config);

        let stats = registry.get_stats().await;
        assert_eq!(stats.total_instances, 0);
        assert_eq!(stats.total_interfaces, 0);
    }

    #[tokio::test]
    async fn test_interface_registration() {
        let config = ComponentModelConfig::default();
        let registry = ComponentRegistry::new(config);

        let interface = ComponentInterface {
            name: "test-interface".to_string(),
            version: "1.0.0".to_string(),
            exports: vec![],
            imports: vec![],
            types: vec![],
        };

        let result = registry.register_interface(interface).await;
        assert!(result.is_ok());

        let stats = registry.get_stats().await;
        assert_eq!(stats.total_interfaces, 1);
    }

    #[tokio::test]
    async fn test_component_instance_creation() {
        let config = ComponentModelConfig::default();
        let registry = ComponentRegistry::new(config);

        let interface = ComponentInterface {
            name: "test-interface".to_string(),
            version: "1.0.0".to_string(),
            exports: vec![],
            imports: vec![],
            types: vec![],
        };

        registry.register_interface(interface).await.unwrap();

        let instance_id = registry.create_instance("test-interface").await.unwrap();
        assert!(!instance_id.is_empty());

        let stats = registry.get_stats().await;
        assert_eq!(stats.total_instances, 1);
    }

    #[tokio::test]
    async fn test_component_value_type_matching() {
        let bool_value = ComponentValue::Bool(true);
        assert!(bool_value.matches_type(&InterfaceType::Bool));
        assert!(!bool_value.matches_type(&InterfaceType::U32));

        let string_value = ComponentValue::String("test".to_string());
        assert!(string_value.matches_type(&InterfaceType::String));
        assert!(!string_value.matches_type(&InterfaceType::Bool));

        let list_value = ComponentValue::List(vec![ComponentValue::U32(1), ComponentValue::U32(2)]);
        assert!(list_value.matches_type(&InterfaceType::List(Box::new(InterfaceType::U32))));
        assert!(!list_value.matches_type(&InterfaceType::List(Box::new(InterfaceType::Bool))));
    }

    #[tokio::test]
    async fn test_component_composition_validation() {
        let config = ComponentModelConfig::default();
        let registry = Arc::new(ComponentRegistry::new(config.clone()));
        let linker = ComponentLinker::new(config, registry.clone());

        // Test empty composition
        let result = linker.validate_composition(&[]).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }
}
