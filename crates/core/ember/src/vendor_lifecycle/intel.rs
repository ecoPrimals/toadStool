// SPDX-License-Identifier: AGPL-3.0-or-later
//! Intel Xe / Arc discrete GPU lifecycle — conservative FLR-aware defaults.
//!
//! Absorbed from coralReef `coral-ember`. Intel discrete GPUs support FLR
//! which is well-behaved compared to NVIDIA's HBM2-destroying bus reset.

use crate::error::SwapError;
use crate::sysfs;

use toadstool_common::interned_strings::socket_env;

use super::types::{RebindStrategy, VendorLifecycle};

const DEFAULT_INTEL_SETTLE_SECS: u64 = 5;

/// Intel discrete Xe / Arc — FLR-oriented lifecycle.
#[derive(Debug)]
pub struct IntelXeLifecycle {
    /// PCI device ID — reserved for Arc vs Battlemage differentiation.
    pub device_id: u16,
    settle_secs: u64,
}

impl IntelXeLifecycle {
    /// Create a lifecycle handler for an Intel Xe/Arc device.
    #[must_use]
    pub fn new(device_id: u16) -> Self {
        let settle_secs = std::env::var(socket_env::TOADSTOOL_INTEL_SETTLE_SECS)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_INTEL_SETTLE_SECS);

        Self {
            device_id,
            settle_secs,
        }
    }
}

impl VendorLifecycle for IntelXeLifecycle {
    fn description(&self) -> &'static str {
        "Intel Xe/Arc (FLR-oriented, conservative defaults)"
    }

    fn prepare_for_unbind(&self, bdf: &str, _current_driver: &str) -> Result<(), SwapError> {
        sysfs::pin_power(bdf);
        Ok(())
    }

    fn rebind_strategy(&self, _target_driver: &str) -> RebindStrategy {
        RebindStrategy::SimpleBind
    }

    fn settle_secs(&self, _target_driver: &str) -> u64 {
        self.settle_secs
    }

    fn stabilize_after_bind(&self, bdf: &str, _target_driver: &str) {
        sysfs::pin_power(bdf);
    }

    fn verify_health(&self, bdf: &str, _target_driver: &str) -> Result<(), SwapError> {
        let power = sysfs::read_power_state(bdf);
        if power.as_deref() == Some("D3cold") {
            return Err(SwapError::VerifyHealth {
                bdf: bdf.to_string(),
                detail: "Intel Xe in D3cold after bind — FLR may have triggered \
                         unexpected power state transition"
                    .to_string(),
            });
        }
        Ok(())
    }
}
