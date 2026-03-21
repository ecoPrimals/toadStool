// SPDX-License-Identifier: AGPL-3.0-only
//! Capability provider error types.
//!
//! Errors for capability-based discovery and invocation.

use crate::primal_identity::Capability;

/// Errors for capability-based discovery
#[derive(Debug, thiserror::Error)]
pub enum CapabilityError {
    /// No provider advertises the requested capability
    #[error("No provider found for capability: {0:?}")]
    NoProviderFound(Capability),

    /// Provider endpoint is unreachable
    #[error("Provider unreachable: {0}")]
    ProviderUnreachable(String),

    /// RPC call to provider failed
    #[error("RPC call failed: {0}")]
    RpcFailed(String),

    /// Discovery service could not be reached
    #[error("Discovery service unavailable")]
    DiscoveryUnavailable,

    /// Provider returned invalid or malformed response
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

/// Result type for capability provider operations
pub type Result<T> = std::result::Result<T, CapabilityError>;
