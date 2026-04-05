// SPDX-License-Identifier: AGPL-3.0-or-later

//! Device health monitoring.
//!
//! Health checking follows two modes:
//!
//! - **Passive**: read sysfs/procfs attributes without touching hardware
//! - **Active**: probe the device directly (e.g. BAR0 read, USB descriptor
//!   fetch, NPU register ping)
//!
//! Implementations provide a [`HealthProbe`] that glowPlug's orchestrator
//! calls on a schedule or after lifecycle events (swap, lend, reclaim).

use std::fmt;

use serde::{Deserialize, Serialize};

/// Device health status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Health has not been checked yet.
    Unknown,
    /// Device is healthy and fully operational.
    Healthy,
    /// Device is degraded but functional (e.g. thermal throttling, partial capability).
    Degraded {
        /// Reason for degradation.
        reason: String,
    },
    /// Device is faulted and not usable.
    Faulted {
        /// Fault description.
        reason: String,
    },
    /// Device is unreachable (e.g. removed, powered off, D3cold).
    Unreachable,
}

impl HealthStatus {
    /// Whether the device is usable (healthy or degraded).
    #[must_use]
    pub const fn is_usable(&self) -> bool {
        matches!(self, Self::Healthy | Self::Degraded { .. })
    }
}

impl fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown => write!(f, "unknown"),
            Self::Healthy => write!(f, "healthy"),
            Self::Degraded { reason } => write!(f, "degraded: {reason}"),
            Self::Faulted { reason } => write!(f, "faulted: {reason}"),
            Self::Unreachable => write!(f, "unreachable"),
        }
    }
}

/// Probes a device's health. Implementations are hardware-class-specific.
#[async_trait::async_trait]
pub trait HealthProbe: Send + Sync {
    /// Error type for probe failures.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Passive health check (sysfs/procfs only, no device touch).
    async fn passive_check(&self) -> Result<HealthStatus, Self::Error>;

    /// Active health check (touches the device — BAR0 read, descriptor
    /// fetch, register ping, etc.).
    async fn active_check(&self) -> Result<HealthStatus, Self::Error>;
}
