//! Universal Primal Provider trait

use async_trait::async_trait;

use crate::ToadStoolResult;

use super::requests::{PrimalEndpoints, PrimalRequest, PrimalResponse};
use super::types::{PrimalCapability, PrimalContext, PrimalHealth, PrimalType};

/// Universal primal provider trait
// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
pub trait UniversalPrimalProvider: Send + Sync {
    /// Unique primal identifier
    fn primal_id(&self) -> &str;

    /// Instance identifier
    fn instance_id(&self) -> &str;

    /// Context this primal serves
    fn context(&self) -> &PrimalContext;

    /// Primal type
    fn primal_type(&self) -> PrimalType;

    /// Capabilities provided
    fn capabilities(&self) -> Vec<PrimalCapability>;

    /// Health check
    async fn health_check(&self) -> PrimalHealth;

    /// API endpoints
    fn endpoints(&self) -> PrimalEndpoints;

    /// Handle inter-primal requests
    async fn handle_primal_request(
        &self,
        request: PrimalRequest,
    ) -> ToadStoolResult<PrimalResponse>;

    /// Initialize with configuration
    async fn initialize(&mut self, config: serde_json::Value) -> ToadStoolResult<()>;

    /// Shutdown gracefully
    async fn shutdown(&mut self) -> ToadStoolResult<()>;

    /// Check if can serve context
    fn can_serve_context(&self, context: &PrimalContext) -> bool;
}
