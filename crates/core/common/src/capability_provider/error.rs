// SPDX-License-Identifier: AGPL-3.0-only
//! Capability provider error types.
//!
//! Errors for capability-based discovery and invocation.

use crate::primal_identity::Capability;

/// Errors for capability-based discovery
#[derive(Debug, thiserror::Error)]
pub enum CapabilityError {
    #[error("No provider found for capability: {0:?}")]
    NoProviderFound(Capability),

    #[error("Provider unreachable: {0}")]
    ProviderUnreachable(String),

    #[error("RPC call failed: {0}")]
    RpcFailed(String),

    #[error("Discovery service unavailable")]
    DiscoveryUnavailable,

    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

pub type Result<T> = std::result::Result<T, CapabilityError>;
