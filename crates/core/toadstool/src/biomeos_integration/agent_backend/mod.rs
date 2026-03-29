// SPDX-License-Identifier: AGPL-3.0-only
//! Agent backend traits and implementations for BiomeOS/Squirrel integration
//!
//! This module defines the trait interface for agent deployment backends and
//! provides production and test implementations using proper dependency injection.

mod inmemory;
mod squirrel;
mod types;

#[cfg(test)]
mod tests;

use async_trait::async_trait;

use super::types::{AgentConfig, ModelConfig};
use crate::ToadStoolResult;
pub use inmemory::InMemoryAgentBackend;
pub use squirrel::SquirrelBackend;
pub use types::{
    AgentInfo, AgentResourceUsage, AgentStatus, ModelInfo, ModelPerformanceMetrics,
    ModelResourceRequirements, ModelStatus,
};

/// Trait defining the interface for agent deployment backends
///
/// This allows dependency injection of different agent deployment implementations
/// (production Squirrel backend, in-memory test backend, etc.) without relying
/// on feature flags or conditional compilation.
// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
pub trait AgentBackend: Send + Sync {
    /// Initialize/test connection to agent backend
    ///
    /// For network backends (Squirrel), this tests connectivity.
    /// For local backends (in-memory), this is typically a no-op.
    async fn initialize(&self) -> ToadStoolResult<()> {
        Ok(()) // Default implementation is no-op
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
    async fn deploy_agent(&self, config: &AgentConfig) -> ToadStoolResult<AgentInfo>;

    /// Load a model for agent use
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Model configuration is invalid
    /// - Model file cannot be accessed or downloaded
    /// - Insufficient memory for model
    /// - Model format is unsupported
    async fn load_model(&self, config: &ModelConfig) -> ToadStoolResult<ModelInfo>;

    /// Scale an agent to specified replica count
    async fn scale_agent(&self, agent_name: &str, replicas: u32) -> ToadStoolResult<()>;

    /// Stop an agent
    async fn stop_agent(&self, agent_name: &str) -> ToadStoolResult<()>;

    /// Remove an agent
    async fn remove_agent(&self, agent_name: &str) -> ToadStoolResult<()>;

    /// Get agent status
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Agent does not exist
    /// - Backend service is unavailable
    /// - Network communication fails
    async fn get_agent_status(&self, agent_name: &str) -> ToadStoolResult<AgentStatus>;

    /// List all deployed agents
    async fn list_agents(&self) -> ToadStoolResult<Vec<AgentInfo>>;

    /// List all loaded models
    async fn list_models(&self) -> ToadStoolResult<Vec<ModelInfo>>;

    /// Get agent resource usage
    async fn get_agent_resources(&self, agent_name: &str) -> ToadStoolResult<AgentResourceUsage>;

    /// Unload a model
    async fn unload_model(&self, model_name: &str) -> ToadStoolResult<()>;

    /// Health check for agent backend
    async fn health_check(&self) -> ToadStoolResult<()> {
        Ok(()) // Default implementation
    }
}
