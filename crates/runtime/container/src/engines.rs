// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2025 ecoPrimals

//! Container Runtime Engines
//!
//! Abstraction layer for different container runtimes (Docker, Podman, containerd, etc.)

pub use toadstool_types::UniversalError;

/// Container engine types
#[derive(Debug, Clone)]
pub enum ContainerEngine {
    /// Docker engine
    Docker,
    /// Podman engine
    Podman,
    /// containerd
    Containerd,
    /// Generic OCI-compliant runtime
    Generic(String),
}

impl Default for ContainerEngine {
    fn default() -> Self {
        Self::Docker
    }
}
