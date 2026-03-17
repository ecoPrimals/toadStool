// SPDX-License-Identifier: AGPL-3.0-only
//! Component linking and composition

use std::sync::Arc;
use tracing::{info, warn};

use toadstool::{ToadStoolError, ToadStoolResult};

use super::core::ComponentModelConfig;
use super::registry::ComponentRegistry;

/// Component linker for connecting components
pub struct ComponentLinker {
    /// Configuration
    _config: ComponentModelConfig,
    /// Component registry
    registry: Arc<ComponentRegistry>,
}

impl ComponentLinker {
    /// Create a new component linker
    #[must_use]
    pub const fn new(config: ComponentModelConfig, registry: Arc<ComponentRegistry>) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component_model::core::ComponentInterface;

    #[tokio::test]
    async fn test_component_linker_creation() {
        let config = ComponentModelConfig::default();
        let registry = Arc::new(ComponentRegistry::new(config.clone()));
        let _linker = ComponentLinker::new(config, registry);
    }

    #[tokio::test]
    async fn test_link_components_consumer_missing_interface() {
        let config = ComponentModelConfig::default();
        let registry = Arc::new(ComponentRegistry::new(config.clone()));

        let iface = ComponentInterface {
            name: "provider-iface".to_string(),
            version: "1.0".to_string(),
            exports: vec![],
            imports: vec![],
            types: vec![],
        };
        registry.register_interface(iface).await.unwrap();
        let provider_id = registry.create_instance("provider-iface").await.unwrap();

        let consumer_iface = ComponentInterface {
            name: "consumer-iface".to_string(),
            version: "1.0".to_string(),
            exports: vec![],
            imports: vec![],
            types: vec![],
        };
        registry.register_interface(consumer_iface).await.unwrap();
        let consumer_id = registry.create_instance("consumer-iface").await.unwrap();

        let linker = ComponentLinker::new(config, registry);
        let result = linker
            .link_components(&consumer_id, &provider_id, "nonexistent")
            .await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("does not have interface")
        );
    }

    #[tokio::test]
    async fn test_link_components_success() {
        let config = ComponentModelConfig::default();
        let registry = Arc::new(ComponentRegistry::new(config.clone()));

        let iface = ComponentInterface {
            name: "shared-iface".to_string(),
            version: "1.0".to_string(),
            exports: vec![],
            imports: vec![],
            types: vec![],
        };
        registry.register_interface(iface).await.unwrap();
        let provider_id = registry.create_instance("shared-iface").await.unwrap();
        let consumer_id = registry.create_instance("shared-iface").await.unwrap();

        let linker = ComponentLinker::new(config, registry);
        let result = linker
            .link_components(&consumer_id, &provider_id, "shared-iface")
            .await;
        assert!(result.is_ok());
    }
}
