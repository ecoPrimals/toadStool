// SPDX-License-Identifier: AGPL-3.0-or-later
//! Compatibility layer trait definition.
//!
//! Canonical definition of the `CompatibilityLayer` trait. All OS-specific
//! implementations use this trait.

use std::future::Future;

use crate::{ExecutionRequest, ExecutionResponse, ToadStoolResult};

use super::legacy::LegacyCompatibilityLayer;
use super::linux::LinuxCompatibilityLayer;
use super::macos::MacOSCompatibilityLayer;
use super::windows::WindowsCompatibilityLayer;

/// Compatibility layer trait for different operating systems
///
/// This is the canonical definition of the `CompatibilityLayer` trait.
/// All OS-specific compatibility implementations should use this trait.
///
/// Migrated from `async_trait` to native async for zero-cost abstraction.
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
    ) -> impl Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_;

    /// Initialize the compatibility layer
    fn initialize(&mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_;

    /// Shutdown the compatibility layer
    fn shutdown(&mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_;
}

/// Concrete compatibility layers bundled for dispatch from the OS layer manager.
#[derive(Debug)]
pub enum CompatibilityLayerDispatch {
    /// Linux compatibility implementation.
    Linux(LinuxCompatibilityLayer),
    /// macOS compatibility implementation.
    MacOS(MacOSCompatibilityLayer),
    /// Windows compatibility implementation.
    Windows(WindowsCompatibilityLayer),
    /// Legacy compatibility implementation.
    Legacy(LegacyCompatibilityLayer),
}

impl CompatibilityLayer for CompatibilityLayerDispatch {
    fn name(&self) -> &str {
        match self {
            Self::Linux(l) => l.name(),
            Self::MacOS(m) => m.name(),
            Self::Windows(w) => w.name(),
            Self::Legacy(l) => l.name(),
        }
    }

    fn features(&self) -> Vec<String> {
        match self {
            Self::Linux(l) => l.features(),
            Self::MacOS(m) => m.features(),
            Self::Windows(w) => w.features(),
            Self::Legacy(l) => l.features(),
        }
    }

    fn can_handle(&self, request: &ExecutionRequest) -> bool {
        match self {
            Self::Linux(l) => l.can_handle(request),
            Self::MacOS(m) => m.can_handle(request),
            Self::Windows(w) => w.can_handle(request),
            Self::Legacy(l) => l.can_handle(request),
        }
    }

    fn execute_with_compatibility(
        &self,
        request: ExecutionRequest,
    ) -> impl Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_ {
        async move {
            match self {
                Self::Linux(l) => l.execute_with_compatibility(request).await,
                Self::MacOS(m) => m.execute_with_compatibility(request).await,
                Self::Windows(w) => w.execute_with_compatibility(request).await,
                Self::Legacy(l) => l.execute_with_compatibility(request).await,
            }
        }
    }

    fn initialize(&mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async move {
            match self {
                Self::Linux(l) => l.initialize().await,
                Self::MacOS(m) => m.initialize().await,
                Self::Windows(w) => w.initialize().await,
                Self::Legacy(l) => l.initialize().await,
            }
        }
    }

    fn shutdown(&mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async move {
            match self {
                Self::Linux(l) => l.shutdown().await,
                Self::MacOS(m) => m.shutdown().await,
                Self::Windows(w) => w.shutdown().await,
                Self::Legacy(l) => l.shutdown().await,
            }
        }
    }
}
