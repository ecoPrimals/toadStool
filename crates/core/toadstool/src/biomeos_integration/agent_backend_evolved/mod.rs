// SPDX-License-Identifier: AGPL-3.0-or-later
//! Capability-based agent deployment backend.
//!
//! Discovers AI/agent providers by capability, not by name. Uses runtime discovery
//! via the capability provider and structured RPC calls (`ai.*`).

#[cfg(unix)]
mod backend;
mod error;
mod types;

#[cfg(unix)]
pub use backend::AgentBackend;
pub use error::{AgentBackendError, Result};
pub use types::{
    AgentInfo, AgentStatus, DeployAgentRequest, LoadModelRequest, ModelInfo, ModelStatus,
};
