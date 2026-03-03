// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 ecoPrimals

//! Container Runtime Configuration
//!
//! Configuration types for container runtime engine.
//! Re-exports canonical configuration from toadstool-types.

pub use toadstool_types::ToadStoolConfiguration;

/// Container-specific runtime configuration
#[derive(Debug, Clone)]
pub struct ContainerRuntimeConfig {
    /// Maximum number of concurrent containers
    pub max_concurrent: usize,
    /// Resource limits per container
    pub resource_limits: ResourceLimits,
}

/// Resource limits for containers
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum memory in bytes
    pub max_memory_bytes: Option<u64>,
    /// Maximum CPU cores
    pub max_cpu_cores: Option<f64>,
}

impl Default for ContainerRuntimeConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 10,
            resource_limits: ResourceLimits {
                max_memory_bytes: Some(4 * 1024 * 1024 * 1024), // 4GB
                max_cpu_cores: Some(4.0),
            },
        }
    }
}
