// SPDX-License-Identifier: AGPL-3.0-or-later
//! Error types for the capability-based agent backend.

use toadstool_common::capability_provider::CapabilityError;

/// Errors for agent backend
#[derive(Debug, thiserror::Error)]
pub enum AgentBackendError {
    /// No AI agent provider discovered via capability lookup.
    #[error("AI agent provider not found")]
    NoAgentProvider,

    /// Agent deployment RPC or provider call failed.
    #[error("Agent deployment failed: {0}")]
    DeploymentFailed(String),

    /// Model loading from storage or provider failed.
    #[error("Model loading failed: {0}")]
    ModelLoadFailed(String),

    /// Agent replica scaling operation failed.
    #[error("Agent scaling failed: {0}")]
    ScalingFailed(String),

    /// Requested agent ID does not exist.
    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    /// Requested model ID does not exist or is not loaded.
    #[error("Model not found: {0}")]
    ModelNotFound(String),

    /// Agent stop or remove operation failed.
    #[error("Agent termination failed: {0}")]
    TerminationFailed(String),

    /// Underlying capability provider error (discovery, RPC, etc.).
    #[error("Capability error: {0}")]
    Capability(#[from] CapabilityError),

    /// JSON serialization or deserialization failed.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Result type for agent backend operations.
pub type Result<T> = std::result::Result<T, AgentBackendError>;
