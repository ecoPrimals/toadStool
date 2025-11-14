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
