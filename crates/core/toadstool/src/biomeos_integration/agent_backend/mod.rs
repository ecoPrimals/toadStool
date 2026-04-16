// SPDX-License-Identifier: AGPL-3.0-or-later
//! Agent backend traits and implementations for BiomeOS intelligence service integration
//!
//! This module defines the trait interface for agent deployment backends and
//! provides production and test implementations using proper dependency injection.

mod inmemory;
mod intelligence;
mod types;

#[cfg(test)]
mod tests;

use std::future::Future;
use std::pin::Pin;

use super::types::{AgentConfig, ModelConfig};
use crate::ToadStoolResult;
pub use inmemory::InMemoryAgentBackend;
pub use intelligence::IntelligenceBackend;
pub use types::{
    AgentInfo, AgentResourceUsage, AgentStatus, ModelInfo, ModelPerformanceMetrics,
    ModelResourceRequirements, ModelStatus,
};

/// Trait defining the interface for agent deployment backends
///
/// This allows dependency injection of different agent deployment implementations
/// (production intelligence-service backend, in-memory test backend, etc.) without relying
/// on feature flags or conditional compilation.
// NOTE(async-dyn): async methods return `Pin<Box<dyn Future>>` — native async fn in trait is not dyn-compatible
pub trait AgentBackend: Send + Sync {
    /// Initialize/test connection to agent backend
    ///
    /// For network backends (intelligence service), this tests connectivity.
    /// For local backends (in-memory), this is typically a no-op.
    fn initialize(&self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }

    /// Deploy an AI agent from configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Agent configuration is invalid
    /// - Backend service is unavailable
    /// - Resource allocation fails
    /// - Agent name conflicts with existing agent
    fn deploy_agent<'a>(
        &'a self,
        config: &'a AgentConfig,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<AgentInfo>> + Send + 'a>>;

    /// Load a model for agent use
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Model configuration is invalid
    /// - Model file cannot be accessed or downloaded
    /// - Insufficient memory for model
    /// - Model format is unsupported
    fn load_model<'a>(
        &'a self,
        config: &'a ModelConfig,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<ModelInfo>> + Send + 'a>>;

    /// Scale an agent to specified replica count
    fn scale_agent<'a>(
        &'a self,
        agent_name: &'a str,
        replicas: u32,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + 'a>>;

    /// Stop an agent
    fn stop_agent<'a>(
        &'a self,
        agent_name: &'a str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + 'a>>;

    /// Remove an agent
    fn remove_agent<'a>(
        &'a self,
        agent_name: &'a str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + 'a>>;

    /// Get agent status
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Agent does not exist
    /// - Backend service is unavailable
    /// - Network communication fails
    fn get_agent_status<'a>(
        &'a self,
        agent_name: &'a str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<AgentStatus>> + Send + 'a>>;

    /// List all deployed agents
    fn list_agents(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<Vec<AgentInfo>>> + Send + '_>>;

    /// List all loaded models
    fn list_models(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<Vec<ModelInfo>>> + Send + '_>>;

    /// Get agent resource usage
    fn get_agent_resources<'a>(
        &'a self,
        agent_name: &'a str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<AgentResourceUsage>> + Send + 'a>>;

    /// Unload a model
    fn unload_model<'a>(
        &'a self,
        model_name: &'a str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + 'a>>;

    /// Health check for agent backend
    fn health_check(&self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }
}
