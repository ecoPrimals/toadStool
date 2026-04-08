// SPDX-License-Identifier: AGPL-3.0-or-later
//! AI agent deployment and management via intelligence service.
//!
//! Configuration and the [`AgentDeploymentManager`] live in submodules; public
//! types from `agent_backend` are re-exported here for a stable API surface.

pub use super::agent_backend::{
    AgentInfo, AgentResourceUsage, AgentStatus, ModelInfo, ModelPerformanceMetrics,
    ModelResourceRequirements, ModelStatus,
};

mod config;
mod manager;

#[cfg(test)]
mod tests;

pub use config::AgentDeploymentConfig;
pub use manager::AgentDeploymentManager;
