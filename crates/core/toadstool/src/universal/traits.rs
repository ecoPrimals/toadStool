// SPDX-License-Identifier: AGPL-3.0-or-later
//! Universal Primal Provider trait

use std::future::Future;
use std::pin::Pin;

use crate::ToadStoolResult;

use super::requests::{PrimalEndpoints, PrimalRequest, PrimalResponse};
use super::types::{PrimalCapability, PrimalContext, PrimalHealth, PrimalType};

/// Universal primal provider trait
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
    fn health_check(&self) -> Pin<Box<dyn Future<Output = PrimalHealth> + Send + '_>>;

    /// API endpoints
    fn endpoints(&self) -> PrimalEndpoints;

    /// Handle inter-primal requests
    fn handle_primal_request(
        &self,
        request: PrimalRequest,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<PrimalResponse>> + Send + '_>>;

    /// Initialize with configuration
    fn initialize(
        &mut self,
        config: serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>>;

    /// Shutdown gracefully
    fn shutdown(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>>;

    /// Check if can serve context
    fn can_serve_context(&self, context: &PrimalContext) -> bool;
}
