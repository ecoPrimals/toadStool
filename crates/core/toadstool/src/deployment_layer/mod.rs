// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2024-2025 ToadStool Project

//! Deployment Layer Detection and Adaptation
//!
//! This module implements multi-layer OS support for Toadstool, enabling it to
//! work correctly whether running as:
//! - The base OS (bare metal)
//! - Middleware on another OS (e.g., Pop!_OS)
//! - Service provider to another OS (e.g., `SteamOS` on biomeOS)
//! - Inside a container (Docker/Podman)
//! - Inside a VM (QEMU/KVM)
//! - In the cloud (EC2/GCE/Azure)
//!
//! # Philosophy
//!
//! **Adaptation over assumption**: Don't assume where we're running,
//! detect it and adapt accordingly.

#[cfg(feature = "runtime")]
mod detector;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime")]
pub use detector::LayerDetector;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Deployment layer where Toadstool is running
///
/// This determines how Toadstool exposes capabilities and interacts
/// with other system components.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeploymentLayer {
    /// Running as the base OS on bare metal
    BareMetalOS,

    /// Running as middleware on another OS
    MiddlewareLayer {
        /// Host OS name (e.g. Pop!_OS).
        host_os: String,
        /// Host OS version if detectable.
        host_version: Option<String>,
    },

    /// Providing services to another OS layer above
    ServiceLayer {
        /// Guest OS(s) being served (e.g. `SteamOS`).
        guest_os: Vec<String>,
    },

    /// Running inside a container
    ContainerLayer {
        /// Container runtime (Docker, Podman, etc.).
        runtime: ContainerRuntime,
        /// Container ID if available.
        container_id: Option<String>,
    },

    /// Running inside a virtual machine
    VMLayer {
        /// Hypervisor name (e.g. QEMU, KVM).
        hypervisor: String,
        /// Whether GPU is passed through to the VM.
        gpu_passthrough: bool,
    },

    /// Running in a cloud environment
    CloudLayer {
        /// Cloud provider.
        provider: CloudProvider,
        /// Instance type if known.
        instance_type: Option<String>,
        /// Region if known.
        region: Option<String>,
    },
}

impl DeploymentLayer {
    /// Get a human-readable description of this layer
    pub const fn description(&self) -> &'static str {
        match self {
            Self::BareMetalOS => "Base OS on bare metal",
            Self::MiddlewareLayer { .. } => "Middleware on host OS",
            Self::ServiceLayer { .. } => "Service provider to guest OS",
            Self::ContainerLayer { .. } => "Inside container",
            Self::VMLayer { .. } => "Inside virtual machine",
            Self::CloudLayer { .. } => "Cloud environment",
        }
    }

    /// Get the host OS if running as middleware
    pub fn host_os(&self) -> Option<&str> {
        match self {
            Self::MiddlewareLayer { host_os, .. } => Some(host_os),
            _ => None,
        }
    }

    /// Get guest OS(s) if providing services
    pub fn guest_os(&self) -> Option<&[String]> {
        match self {
            Self::ServiceLayer { guest_os } => Some(guest_os),
            _ => None,
        }
    }

    /// Check if running in a virtualized environment
    pub const fn is_virtualized(&self) -> bool {
        matches!(
            self,
            Self::ContainerLayer { .. } | Self::VMLayer { .. } | Self::CloudLayer { .. }
        )
    }

    /// Check if we have direct hardware access
    pub const fn has_direct_hardware_access(&self) -> bool {
        matches!(self, Self::BareMetalOS)
            || matches!(
                self,
                Self::VMLayer {
                    gpu_passthrough: true,
                    ..
                }
            )
    }
}

impl fmt::Display for DeploymentLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BareMetalOS => write!(f, "BareMetalOS"),
            Self::MiddlewareLayer { host_os, .. } => write!(f, "Middleware on {host_os}"),
            Self::ServiceLayer { guest_os } => {
                write!(f, "ServiceLayer (serving: {})", guest_os.join(", "))
            }
            Self::ContainerLayer { runtime, .. } => write!(f, "Container ({runtime:?})"),
            Self::VMLayer { hypervisor, .. } => write!(f, "VM ({hypervisor})"),
            Self::CloudLayer { provider, .. } => write!(f, "Cloud ({provider:?})"),
        }
    }
}

/// Container runtime types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContainerRuntime {
    /// Docker runtime.
    Docker,
    /// Podman runtime.
    Podman,
    /// Containerd runtime.
    Containerd,
    /// CRI-O runtime.
    CRIO,
    /// Other or custom runtime.
    Other(String),
}

/// Cloud provider types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CloudProvider {
    /// Amazon Web Services.
    AWS,
    /// Google Cloud Platform.
    GCP,
    /// Microsoft Azure.
    Azure,
    /// Oracle Cloud.
    Oracle,
    /// `DigitalOcean`.
    DigitalOcean,
    /// Custom or unknown provider.
    Custom(String),
}

/// Detection errors
#[derive(Debug, thiserror::Error)]
pub enum DetectionError {
    /// I/O error during detection.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// External HTTP detection disabled.
    #[error("External HTTP detection disabled - use coordination service for external HTTP")]
    ExternalHttpDisabled,
    /// Container ID not found in environment.
    #[error("Container ID not found")]
    ContainerIdNotFound,
    /// Generic detection failure.
    #[error("Failed to detect deployment layer: {0}")]
    DetectionFailed(String),
}
