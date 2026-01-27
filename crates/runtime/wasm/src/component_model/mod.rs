//! WebAssembly Component Model Support
//!
//! This module provides support for WebAssembly component model features,
//! enabling composable, interface-driven WebAssembly applications.
//!
//! ## Architecture
//!
//! This module is organized into 4 component types:
//! - **core**: Interface definitions, types, and values
//! - **instances**: Component instance management
//! - **registry**: Component registry and statistics
//! - **linking**: Component linking and composition

pub mod core;
pub mod instances;
pub mod linking;
pub mod registry;

// Re-export all public types for backward compatibility
pub use core::*;
pub use instances::*;
pub use linking::*;
pub use registry::*;

use async_trait::async_trait;
use toadstool::ToadStoolResult;

// Re-export for backwards compatibility
pub use crate::WasmRuntimeEngine;

/// Component model support trait
#[async_trait]
pub trait ComponentModelSupport: Send + Sync {
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

#[async_trait]
impl ComponentModelSupport for WasmRuntimeEngine {
    /// Check if component model is supported
    /// TODO: Implement component model configuration
    fn supports_component_model(&self) -> bool {
        // Component model not yet fully integrated
        false
    }

    /// Get component model configuration
    /// TODO: Implement component model configuration  
    fn get_component_config(&self) -> &ComponentModelConfig {
        // Return default config for now
        static DEFAULT_CONFIG: ComponentModelConfig = ComponentModelConfig {
            enabled: false,
            max_instances: 0,
            linking_timeout_ms: 0,
            composition_enabled: false,
            wit_support: false,
        };
        &DEFAULT_CONFIG
    }

    /// Create component instance
    async fn create_component_instance(&self, _interface_name: &str) -> ToadStoolResult<String> {
        use toadstool::ToadStoolError;

        if !self.supports_component_model() {
            return Err(ToadStoolError::not_supported(
                "Component model support is disabled".to_string(),
            ));
        }

        // TODO: Implement component registry integration
        // self.component_registry.create_instance(interface_name).await
        Err(ToadStoolError::not_supported(
            "Component registry not yet integrated".to_string(),
        ))
    }

    /// Execute component function
    async fn execute_component_function(
        &self,
        instance_id: &str,
        function_name: &str,
        args: &[ComponentValue],
    ) -> ToadStoolResult<ComponentValue> {
        use toadstool::ToadStoolError;
        use tracing::info;

        if !self.supports_component_model() {
            return Err(ToadStoolError::not_supported(
                "Component model support is disabled".to_string(),
            ));
        }

        // TODO: Implement component registry integration
        // Get the component instance
        // let _instance = self.component_registry.get_instance(instance_id).await?;

        // Update instance state to running
        // self.component_registry
        //     .update_state(instance_id, ComponentState::Running)
        //     .await?;

        // For now, return a mock response - in a real implementation, this would
        // invoke the actual component function through Wasmtime
        info!(
            "Executing component function: {} on instance: {}",
            function_name, instance_id
        );

        // Simulate function execution result
        let result = match function_name {
            "add" => {
                if args.len() == 2 {
                    match (&args[0], &args[1]) {
                        (ComponentValue::U32(a), ComponentValue::U32(b)) => {
                            ComponentValue::U32(a + b)
                        }
                        _ => ComponentValue::String("Type error".to_string()),
                    }
                } else {
                    ComponentValue::String("Argument count error".to_string())
                }
            }
            "greet" => {
                if args.len() == 1 {
                    match &args[0] {
                        ComponentValue::String(name) => {
                            ComponentValue::String(format!("Hello, {name}!"))
                        }
                        _ => ComponentValue::String("Type error".to_string()),
                    }
                } else {
                    ComponentValue::String("Argument count error".to_string())
                }
            }
            _ => ComponentValue::String(format!("Unknown function: {function_name}")),
        };

        // Update instance state back to ready
        // TODO: Implement component registry integration
        // self.component_registry
        //     .update_state(instance_id, ComponentState::Ready)
        //     .await?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_component_registry_creation() {
        let config = ComponentModelConfig::default();
        let registry = ComponentRegistry::new(config);

        let stats = registry.get_stats().await;
        assert_eq!(stats.total_instances, 0);
        assert_eq!(stats.total_interfaces, 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

        registry
            .register_interface(interface)
            .await
            .expect("Interface registration should succeed in test");

        let instance_id = registry
            .create_instance("test-interface")
            .await
            .expect("Instance creation should succeed");
        assert!(!instance_id.is_empty());

        let stats = registry.get_stats().await;
        assert_eq!(stats.total_instances, 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_component_composition_validation() {
        use std::sync::Arc;

        let config = ComponentModelConfig::default();
        let registry = Arc::new(ComponentRegistry::new(config.clone()));
        let linker = ComponentLinker::new(config, registry.clone());

        // Test empty composition
        let result = linker.validate_composition(&[]).await;
        assert!(result.is_ok());
        assert!(result.expect("Result should be Ok for validation test"));
    }

    #[test]
    fn test_component_model_config_default() {
        let config = ComponentModelConfig::default();
        assert!(config.enabled);
        assert_eq!(config.max_instances, 1000);
        assert_eq!(config.linking_timeout_ms, 5000);
        assert!(config.composition_enabled);
        assert!(config.wit_support);
    }

    #[test]
    fn test_interface_type_variants() {
        assert!(matches!(InterfaceType::Bool, InterfaceType::Bool));
        assert!(matches!(InterfaceType::U32, InterfaceType::U32));
        assert!(matches!(InterfaceType::String, InterfaceType::String));

        let list_type = InterfaceType::List(Box::new(InterfaceType::U32));
        assert!(matches!(list_type, InterfaceType::List(_)));
    }

    #[test]
    fn test_component_value_variants() {
        assert!(matches!(
            ComponentValue::Bool(true),
            ComponentValue::Bool(_)
        ));
        assert!(matches!(ComponentValue::U32(1), ComponentValue::U32(_)));
        assert!(matches!(
            ComponentValue::String("test".to_string()),
            ComponentValue::String(_)
        ));
    }
}
