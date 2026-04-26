// SPDX-License-Identifier: AGPL-3.0-or-later
//! Agent backend traits and implementations for BiomeOS intelligence service integration
//!
//! This module defines the trait interface for agent deployment backends and
//! provides production and test implementations using proper dependency injection.

#[cfg(any(test, feature = "test-mocks"))]
mod inmemory;
mod intelligence;
mod types;

#[cfg(test)]
mod tests;

use std::future::Future;

use super::types::{AgentConfig, ModelConfig};
use crate::ToadStoolResult;
#[cfg(any(test, feature = "test-mocks"))]
pub use inmemory::InMemoryAgentBackend;
pub use intelligence::IntelligenceBackend;
pub use types::{
    AgentInfo, AgentResourceUsage, AgentStatus, ModelInfo, ModelPerformanceMetrics,
    ModelResourceRequirements, ModelStatus,
};

/// Concrete agent backend for dependency injection (replaces `Arc<dyn AgentBackend>`).
pub enum AgentBackendDispatch {
    /// Intelligence / ML service backend (Unix JSON-RPC).
    Intelligence(IntelligenceBackend),
    /// In-memory backend for tests and local simulation.
    #[cfg(any(test, feature = "test-mocks"))]
    InMemory(InMemoryAgentBackend),
}

/// Trait defining the interface for agent deployment backends
///
/// This allows dependency injection of different agent deployment implementations
/// (production intelligence-service backend, in-memory test backend, etc.) without relying
/// on feature flags or conditional compilation.
pub trait AgentBackend: Send + Sync {
    /// Initialize/test connection to agent backend
    ///
    /// For network backends (intelligence service), this tests connectivity.
    /// For local backends (in-memory), this is typically a no-op.
    fn initialize(&self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async { Ok(()) }
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
    ) -> impl Future<Output = ToadStoolResult<AgentInfo>> + Send + 'a;

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
    ) -> impl Future<Output = ToadStoolResult<ModelInfo>> + Send + 'a;

    /// Scale an agent to specified replica count
    fn scale_agent<'a>(
        &'a self,
        agent_name: &'a str,
        replicas: u32,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a;

    /// Stop an agent
    fn stop_agent<'a>(
        &'a self,
        agent_name: &'a str,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a;

    /// Remove an agent
    fn remove_agent<'a>(
        &'a self,
        agent_name: &'a str,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a;

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
    ) -> impl Future<Output = ToadStoolResult<AgentStatus>> + Send + 'a;

    /// List all deployed agents
    fn list_agents(&self) -> impl Future<Output = ToadStoolResult<Vec<AgentInfo>>> + Send + '_;

    /// List all loaded models
    fn list_models(&self) -> impl Future<Output = ToadStoolResult<Vec<ModelInfo>>> + Send + '_;

    /// Get agent resource usage
    fn get_agent_resources<'a>(
        &'a self,
        agent_name: &'a str,
    ) -> impl Future<Output = ToadStoolResult<AgentResourceUsage>> + Send + 'a;

    /// Unload a model
    fn unload_model<'a>(
        &'a self,
        model_name: &'a str,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a;

    /// Health check for agent backend
    fn health_check(&self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async { Ok(()) }
    }
}

impl AgentBackend for AgentBackendDispatch {
    fn initialize(&self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async move {
            match self {
                Self::Intelligence(b) => b.initialize().await,
                #[cfg(any(test, feature = "test-mocks"))]
                Self::InMemory(b) => b.initialize().await,
            }
        }
    }

    fn deploy_agent<'a>(
        &'a self,
        config: &'a AgentConfig,
    ) -> impl Future<Output = ToadStoolResult<AgentInfo>> + Send + 'a {
        async move {
            match self {
                Self::Intelligence(b) => b.deploy_agent(config).await,
                #[cfg(any(test, feature = "test-mocks"))]
                Self::InMemory(b) => b.deploy_agent(config).await,
            }
        }
    }

    fn load_model<'a>(
        &'a self,
        config: &'a ModelConfig,
    ) -> impl Future<Output = ToadStoolResult<ModelInfo>> + Send + 'a {
        async move {
            match self {
                Self::Intelligence(b) => b.load_model(config).await,
                #[cfg(any(test, feature = "test-mocks"))]
                Self::InMemory(b) => b.load_model(config).await,
            }
        }
    }

    fn scale_agent<'a>(
        &'a self,
        agent_name: &'a str,
        replicas: u32,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        async move {
            match self {
                Self::Intelligence(b) => b.scale_agent(agent_name, replicas).await,
                #[cfg(any(test, feature = "test-mocks"))]
                Self::InMemory(b) => b.scale_agent(agent_name, replicas).await,
            }
        }
    }

    fn stop_agent<'a>(
        &'a self,
        agent_name: &'a str,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        async move {
            match self {
                Self::Intelligence(b) => b.stop_agent(agent_name).await,
                #[cfg(any(test, feature = "test-mocks"))]
                Self::InMemory(b) => b.stop_agent(agent_name).await,
            }
        }
    }

    fn remove_agent<'a>(
        &'a self,
        agent_name: &'a str,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        async move {
            match self {
                Self::Intelligence(b) => b.remove_agent(agent_name).await,
                #[cfg(any(test, feature = "test-mocks"))]
                Self::InMemory(b) => b.remove_agent(agent_name).await,
            }
        }
    }

    fn get_agent_status<'a>(
        &'a self,
        agent_name: &'a str,
    ) -> impl Future<Output = ToadStoolResult<AgentStatus>> + Send + 'a {
        async move {
            match self {
                Self::Intelligence(b) => b.get_agent_status(agent_name).await,
                #[cfg(any(test, feature = "test-mocks"))]
                Self::InMemory(b) => b.get_agent_status(agent_name).await,
            }
        }
    }

    fn list_agents(&self) -> impl Future<Output = ToadStoolResult<Vec<AgentInfo>>> + Send + '_ {
        async move {
            match self {
                Self::Intelligence(b) => b.list_agents().await,
                #[cfg(any(test, feature = "test-mocks"))]
                Self::InMemory(b) => b.list_agents().await,
            }
        }
    }

    fn list_models(&self) -> impl Future<Output = ToadStoolResult<Vec<ModelInfo>>> + Send + '_ {
        async move {
            match self {
                Self::Intelligence(b) => b.list_models().await,
                #[cfg(any(test, feature = "test-mocks"))]
                Self::InMemory(b) => b.list_models().await,
            }
        }
    }

    fn get_agent_resources<'a>(
        &'a self,
        agent_name: &'a str,
    ) -> impl Future<Output = ToadStoolResult<AgentResourceUsage>> + Send + 'a {
        async move {
            match self {
                Self::Intelligence(b) => b.get_agent_resources(agent_name).await,
                #[cfg(any(test, feature = "test-mocks"))]
                Self::InMemory(b) => b.get_agent_resources(agent_name).await,
            }
        }
    }

    fn unload_model<'a>(
        &'a self,
        model_name: &'a str,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        async move {
            match self {
                Self::Intelligence(b) => b.unload_model(model_name).await,
                #[cfg(any(test, feature = "test-mocks"))]
                Self::InMemory(b) => b.unload_model(model_name).await,
            }
        }
    }

    fn health_check(&self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async move {
            match self {
                Self::Intelligence(b) => b.health_check().await,
                #[cfg(any(test, feature = "test-mocks"))]
                Self::InMemory(b) => b.health_check().await,
            }
        }
    }
}
