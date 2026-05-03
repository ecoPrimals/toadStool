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
#[expect(
    async_fn_in_trait,
    reason = "generic via type param, no dyn dispatch; associated Error type prevents object safety"
)]
pub trait HealthProbe: Send + Sync {
    /// Error type for probe failures.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Passive health check (sysfs/procfs only, no device touch).
    async fn passive_check(&self) -> Result<HealthStatus, Self::Error>;

    /// Active health check (touches the device — BAR0 read, descriptor
    /// fetch, register ping, etc.).
    async fn active_check(&self) -> Result<HealthStatus, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn healthy_is_usable() {
        assert!(HealthStatus::Healthy.is_usable());
    }

    #[test]
    fn degraded_is_usable() {
        let status = HealthStatus::Degraded {
            reason: "thermal".into(),
        };
        assert!(status.is_usable());
    }

    #[test]
    fn faulted_is_not_usable() {
        let status = HealthStatus::Faulted {
            reason: "hw error".into(),
        };
        assert!(!status.is_usable());
    }

    #[test]
    fn unreachable_is_not_usable() {
        assert!(!HealthStatus::Unreachable.is_usable());
    }

    #[test]
    fn unknown_is_not_usable() {
        assert!(!HealthStatus::Unknown.is_usable());
    }

    #[test]
    fn display_all_variants() {
        assert_eq!(HealthStatus::Unknown.to_string(), "unknown");
        assert_eq!(HealthStatus::Healthy.to_string(), "healthy");
        assert_eq!(
            HealthStatus::Degraded {
                reason: "hot".into()
            }
            .to_string(),
            "degraded: hot"
        );
        assert_eq!(
            HealthStatus::Faulted {
                reason: "dead".into()
            }
            .to_string(),
            "faulted: dead"
        );
        assert_eq!(HealthStatus::Unreachable.to_string(), "unreachable");
    }

    #[test]
    fn serde_roundtrip_all_variants() {
        let variants = vec![
            HealthStatus::Unknown,
            HealthStatus::Healthy,
            HealthStatus::Degraded {
                reason: "thermal".into(),
            },
            HealthStatus::Faulted {
                reason: "pcie".into(),
            },
            HealthStatus::Unreachable,
        ];
        for v in &variants {
            let json = serde_json::to_string(v).unwrap();
            let back: HealthStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(&back, v);
        }
    }

    #[test]
    fn equality() {
        assert_eq!(HealthStatus::Healthy, HealthStatus::Healthy);
        assert_ne!(HealthStatus::Healthy, HealthStatus::Unknown);
    }
}
