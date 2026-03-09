// SPDX-License-Identifier: AGPL-3.0-only
//! Compatibility layer trait definition.
//!
//! Canonical definition of the CompatibilityLayer trait. All OS-specific
//! implementations use this trait.

use std::future::Future;
use std::pin::Pin;

use crate::{ExecutionRequest, ExecutionResponse, ToadStoolResult};

/// Compatibility layer trait for different operating systems
///
/// This is the canonical definition of the CompatibilityLayer trait.
/// All OS-specific compatibility implementations should use this trait.
///
/// Migrated from async_trait to native async for zero-cost abstraction.
pub trait CompatibilityLayer: Send + Sync {
    /// Get the name of this compatibility layer
    fn name(&self) -> &str;

    /// Get supported features
    fn features(&self) -> Vec<String>;

    /// Check if this layer can handle the given request
    fn can_handle(&self, request: &ExecutionRequest) -> bool;

    /// Execute a request with OS layer compatibility
    fn execute_with_compatibility(
        &self,
        request: ExecutionRequest,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_>>;

    /// Initialize the compatibility layer
    fn initialize(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>>;

    /// Shutdown the compatibility layer
    fn shutdown(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>>;
}
