//! Universal Primal Registry for capability-based discovery

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::info;

use crate::{ToadStoolError, ToadStoolResult};

use super::requests::{PrimalRequest, PrimalResponse};
use super::traits::UniversalPrimalProvider;
use super::types::{PrimalCapability, PrimalContext};

/// Universal primal registry for capability-based discovery
pub struct UniversalPrimalRegistry {
    /// Registered primal providers
    providers: RwLock<HashMap<String, Arc<dyn UniversalPrimalProvider>>>,
    /// Capability index: capability -> provider instance IDs
    capability_index: RwLock<HashMap<String, Vec<String>>>,
    /// Context index: `user_id` -> provider instance IDs
    context_index: RwLock<HashMap<String, Vec<String>>>,
    /// Type index: `primal_type` -> provider instance IDs
    type_index: RwLock<HashMap<String, Vec<String>>>,
}

impl Default for UniversalPrimalRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl UniversalPrimalRegistry {
    /// Create new registry
    #[must_use]
    pub fn new() -> Self {
        Self {
            providers: RwLock::new(HashMap::new()),
            capability_index: RwLock::new(HashMap::new()),
            context_index: RwLock::new(HashMap::new()),
            type_index: RwLock::new(HashMap::new()),
        }
    }

    /// Register a primal provider
    pub async fn register_primal(
        &self,
        provider: Arc<dyn UniversalPrimalProvider>,
    ) -> ToadStoolResult<()> {
        let instance_id = provider.instance_id().to_string();
        let capabilities = provider.capabilities();
        let context = provider.context().clone();
        let primal_type = provider.primal_type();

        // Register provider
        self.providers
            .write()
            .await
            .insert(instance_id.clone(), provider);

        // Index capabilities
        let mut capability_index = self.capability_index.write().await;
        for capability in capabilities {
            let cap_key = format!("{capability:?}");
            capability_index
                .entry(cap_key)
                .or_insert_with(Vec::new)
                .push(instance_id.clone());
        }

        // Index context
        let mut context_index = self.context_index.write().await;
        context_index
            .entry(context.user_id.clone())
            .or_insert_with(Vec::new)
            .push(instance_id.clone());

        // Index type
        let mut type_index = self.type_index.write().await;
        let type_key = format!("{primal_type:?}");
        type_index
            .entry(type_key)
            .or_insert_with(Vec::new)
            .push(instance_id.clone());

        info!("Registered primal provider: {}", instance_id);
        Ok(())
    }

    /// Find providers by capability
    pub async fn find_by_capability(
        &self,
        capability: &PrimalCapability,
    ) -> Vec<Arc<dyn UniversalPrimalProvider>> {
        let cap_key = format!("{capability:?}");
        let capability_index = self.capability_index.read().await;
        let providers = self.providers.read().await;

        if let Some(instance_ids) = capability_index.get(&cap_key) {
            instance_ids
                .iter()
                .filter_map(|id| providers.get(id))
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Find providers by context
    pub async fn find_by_context(
        &self,
        context: &PrimalContext,
    ) -> Vec<Arc<dyn UniversalPrimalProvider>> {
        let context_index = self.context_index.read().await;
        let providers = self.providers.read().await;

        if let Some(instance_ids) = context_index.get(&context.user_id) {
            instance_ids
                .iter()
                .filter_map(|id| providers.get(id))
                .filter(|provider| provider.can_serve_context(context))
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Route a request to appropriate provider
    pub async fn route_request(&self, request: PrimalRequest) -> ToadStoolResult<PrimalResponse> {
        let providers = self.providers.read().await;

        if let Some(provider) = providers.get(&request.target) {
            provider.handle_primal_request(request).await
        } else {
            Err(ToadStoolError::execution(format!(
                "Target primal not found: {}",
                request.target
            )))
        }
    }

    /// Get all registered providers
    pub async fn get_all_providers(&self) -> Vec<Arc<dyn UniversalPrimalProvider>> {
        self.providers.read().await.values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::universal::provider::ToadStoolPrimalProvider;
    use crate::universal::requests::PrimalRequest;
    use crate::universal::types::{
        NetworkLocation, PrimalCapability, PrimalContext, SecurityLevel,
    };
    use std::collections::HashMap;
    use uuid::Uuid;

    fn make_test_context() -> PrimalContext {
        PrimalContext {
            user_id: "test-user".to_string(),
            device_id: "test-device".to_string(),
            session_id: "test-session".to_string(),
            network_location: NetworkLocation {
                ip_address: "127.0.0.1".to_string(),
                subnet: None,
                network_id: None,
                geo_location: None,
            },
            security_level: SecurityLevel::Standard,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn test_registry_default() {
        let _registry = UniversalPrimalRegistry::default();
        // Default creates new registry
    }

    #[tokio::test]
    async fn test_registry_new_and_register() {
        let registry = UniversalPrimalRegistry::new();
        let context = make_test_context();
        let provider = Arc::new(ToadStoolPrimalProvider::new(context));
        let result = registry.register_primal(provider).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_registry_find_by_capability() {
        let registry = UniversalPrimalRegistry::new();
        let context = make_test_context();
        let provider = Arc::new(ToadStoolPrimalProvider::new(context));
        registry.register_primal(provider).await.unwrap();

        let providers = registry
            .find_by_capability(&PrimalCapability::WasmExecution { wasi_support: true })
            .await;
        assert!(!providers.is_empty());
    }

    #[tokio::test]
    async fn test_registry_find_by_context() {
        let registry = UniversalPrimalRegistry::new();
        let context = make_test_context();
        let provider = Arc::new(ToadStoolPrimalProvider::new(context.clone()));
        registry.register_primal(provider).await.unwrap();

        let providers = registry.find_by_context(&context).await;
        assert!(!providers.is_empty());
    }

    #[tokio::test]
    async fn test_registry_get_all_providers() {
        let registry = UniversalPrimalRegistry::new();
        let context = make_test_context();
        let provider = Arc::new(ToadStoolPrimalProvider::new(context));
        registry.register_primal(provider).await.unwrap();

        let all = registry.get_all_providers().await;
        assert_eq!(all.len(), 1);
    }

    #[tokio::test]
    async fn test_registry_route_request() {
        let registry = UniversalPrimalRegistry::new();
        let context = make_test_context();
        let provider = Arc::new(ToadStoolPrimalProvider::new(context));
        registry.register_primal(provider).await.unwrap();

        let request = PrimalRequest {
            id: Uuid::new_v4(),
            source: "source".to_string(),
            target: "toadstool-main".to_string(),
            request_type: "ping".to_string(),
            payload: serde_json::json!({}),
            context: make_test_context(),
            metadata: HashMap::new(),
            timestamp: std::time::SystemTime::now(),
        };
        let response = registry.route_request(request).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_registry_route_request_unknown_target() {
        let registry = UniversalPrimalRegistry::new();
        let request = PrimalRequest {
            id: Uuid::new_v4(),
            source: "source".to_string(),
            target: "nonexistent".to_string(),
            request_type: "ping".to_string(),
            payload: serde_json::json!({}),
            context: make_test_context(),
            metadata: HashMap::new(),
            timestamp: std::time::SystemTime::now(),
        };
        let response = registry.route_request(request).await;
        assert!(response.is_err());
    }
}
