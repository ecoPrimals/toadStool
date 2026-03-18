// SPDX-License-Identifier: AGPL-3.0-or-later
//! Autonomous GPU power policy engine.
//!
//! Wraps [`PowerManager`] with configurable policies that govern
//! automatic power state transitions. Designed for the sovereign
//! compute path where toadStool manages GPU power without any
//! proprietary driver.
//!
//! # Policies
//!
//! - `OnDemand`: Sleep when idle, warm on dispatch request.
//! - `AlwaysWarm`: Keep engines clocked. Good for sustained inference.
//! - `AlwaysSovereign`: Channels pre-loaded, instant dispatch.
//! - **Eco**: Aggressive clock gating, minimum idle power.
//! - **Custom**: User-defined clock gate configuration.
//!
//! # Integration
//!
//! `PolicyEngine::tick()` is called periodically (by the watchdog or
//! an external loop). `request_warm()` / `release_warm()` are the
//! pre-warm hint API for barraCuda dispatch integration.

use crate::error::Result;
use crate::power_manager::{ClockGateConfig, GpuPowerState, PowerManager};
use hw_learn::applicator::RegisterAccess;

/// Power management policy for autonomous GPU state control.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PowerPolicy {
    /// Sleep when idle, warm on dispatch. Default for edge/batch.
    OnDemand {
        /// Seconds of idle before transitioning to Sleep.
        idle_timeout_secs: u64,
    },
    /// Keep engines clocked at all times. Suitable for cloud/inference
    /// workloads with frequent dispatches.
    AlwaysWarm,
    /// Channels pre-loaded, instant dispatch. Highest power, lowest latency.
    /// Note: Sovereign state requires coralReef channel loading — the policy
    /// engine will maintain Warm and signal readiness for channel load.
    AlwaysSovereign,
    /// Aggressive clock gating with bus-level CG enabled.
    /// Minimum idle power while keeping the GPU in `PCIe` D0.
    Eco,
    /// User-defined bus clock gate configuration with a target state.
    Custom {
        /// Target power state for this policy.
        target_state: GpuPowerState,
        /// Bus-level clock gating configuration for `NV_PBUS_EXT_CG`.
        clock_gating: ClockGateConfig,
    },
}

impl Default for PowerPolicy {
    fn default() -> Self {
        Self::OnDemand {
            idle_timeout_secs: 60,
        }
    }
}

impl std::fmt::Display for PowerPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OnDemand { idle_timeout_secs } => {
                write!(f, "OnDemand({idle_timeout_secs}s)")
            }
            Self::AlwaysWarm => write!(f, "AlwaysWarm"),
            Self::AlwaysSovereign => write!(f, "AlwaysSovereign"),
            Self::Eco => write!(f, "Eco"),
            Self::Custom {
                target_state,
                clock_gating,
            } => {
                write!(
                    f,
                    "Custom({target_state}, cg={:#010x})",
                    clock_gating.encode()
                )
            }
        }
    }
}

/// Autonomous power policy engine.
///
/// Wraps a `PowerManager` and enforces the active policy via periodic
/// `tick()` calls. Tracks warm requests from dispatch paths to prevent
/// sleeping while work is pending.
pub struct PolicyEngine<R> {
    manager: PowerManager<R>,
    policy: PowerPolicy,
    warm_requests: u32,
    idle_ticks: u64,
    tick_interval_secs: u64,
}

impl<R: RegisterAccess> PolicyEngine<R> {
    /// Create a new policy engine with the default `OnDemand` policy.
    ///
    /// `tick_interval_secs` is the expected interval between `tick()` calls,
    /// used for idle timeout calculation.
    pub fn new(manager: PowerManager<R>, tick_interval_secs: u64) -> Self {
        Self {
            manager,
            policy: PowerPolicy::default(),
            warm_requests: 0,
            idle_ticks: 0,
            tick_interval_secs: tick_interval_secs.max(1),
        }
    }

    /// Set the active power policy.
    ///
    /// Resets idle tracking. The new policy takes effect on the next `tick()`.
    pub fn set_policy(&mut self, policy: PowerPolicy) {
        tracing::info!(policy = %policy, "power policy changed");
        self.policy = policy;
        self.idle_ticks = 0;
    }

    /// Get the current policy.
    #[must_use]
    pub const fn policy(&self) -> &PowerPolicy {
        &self.policy
    }

    /// Pre-warm hint: a dispatch is about to begin.
    ///
    /// Increments the warm request counter and immediately transitions
    /// the GPU to Warm if it isn't already. Multiple callers can hold
    /// warm requests simultaneously — the GPU stays warm until all
    /// release.
    ///
    /// # Errors
    ///
    /// Returns error if the warm transition fails.
    pub fn request_warm(&mut self) -> Result<()> {
        self.warm_requests = self.warm_requests.saturating_add(1);
        self.idle_ticks = 0;
        tracing::debug!(warm_requests = self.warm_requests, "warm request acquired");
        self.manager.warm()
    }

    /// Release a warm request: workload complete.
    ///
    /// Decrements the warm request counter. When zero, the policy engine
    /// may transition to a lower power state on the next tick (depending
    /// on active policy).
    pub fn release_warm(&mut self) {
        self.warm_requests = self.warm_requests.saturating_sub(1);
        tracing::debug!(warm_requests = self.warm_requests, "warm request released");
    }

    /// Number of outstanding warm requests.
    #[must_use]
    pub const fn warm_requests(&self) -> u32 {
        self.warm_requests
    }

    /// Periodic policy enforcement tick.
    ///
    /// Call this at the configured interval (e.g., from the watchdog loop
    /// or a dedicated timer). Enforces the active policy by transitioning
    /// to the appropriate power state.
    ///
    /// # Errors
    ///
    /// Returns error if a power transition fails.
    pub fn tick(&mut self) -> Result<()> {
        if self.warm_requests > 0 {
            self.idle_ticks = 0;
            return self.ensure_warm();
        }

        match &self.policy {
            PowerPolicy::OnDemand { idle_timeout_secs } => {
                self.idle_ticks += 1;
                let idle_secs = self.idle_ticks * self.tick_interval_secs;
                if idle_secs >= *idle_timeout_secs {
                    self.manager.sleep()
                } else {
                    Ok(())
                }
            }
            PowerPolicy::AlwaysWarm | PowerPolicy::AlwaysSovereign => {
                self.idle_ticks = 0;
                self.ensure_warm()
            }
            PowerPolicy::Eco => self.apply_eco(),
            PowerPolicy::Custom {
                target_state,
                clock_gating,
            } => {
                let target = *target_state;
                let cg = *clock_gating;
                self.manager.set_profile(target)?;
                if target == GpuPowerState::Warm
                    || target == GpuPowerState::Glow
                    || target == GpuPowerState::Sovereign
                {
                    self.manager.set_clock_gating(&cg)?;
                }
                Ok(())
            }
        }
    }

    fn ensure_warm(&mut self) -> Result<()> {
        let state = self.manager.current_state()?;
        if state != GpuPowerState::Warm && state != GpuPowerState::Sovereign {
            self.manager.warm()?;
        }
        Ok(())
    }

    fn apply_eco(&mut self) -> Result<()> {
        let state = self.manager.current_state()?;

        match state {
            GpuPowerState::Warm | GpuPowerState::Sovereign => {
                self.manager.cool()?;
            }
            GpuPowerState::Glow | GpuPowerState::Sleep | GpuPowerState::Off => {}
        }

        // Enable aggressive bus-level CG in Glow state
        if self.manager.current_state()? == GpuPowerState::Glow {
            let eco_cg = ClockGateConfig {
                idle_delay: 4,
                idle_cg_en: true,
                stall_cg_en: true,
                wakeup_delay: 2,
            };
            self.manager.set_clock_gating(&eco_cg)?;
        }

        Ok(())
    }

    /// Access the underlying power manager.
    pub const fn manager(&self) -> &PowerManager<R> {
        &self.manager
    }

    /// Mutable access to the underlying power manager.
    pub const fn manager_mut(&mut self) -> &mut PowerManager<R> {
        &mut self.manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registers;
    use std::collections::HashMap;

    struct MockRegs {
        values: HashMap<u64, u32>,
    }

    impl MockRegs {
        fn warm() -> Self {
            let mut values = HashMap::new();
            values.insert(registers::PMC_ENABLE, registers::PMC_ENABLE_WARM);
            values.insert(registers::PFIFO_ENABLE, 0);
            values.insert(registers::GPU_TEMP, 0x2E00);
            values.insert(registers::PBUS_EXT_CG, 0);
            Self { values }
        }
    }

    impl RegisterAccess for MockRegs {
        fn read_u32(&self, offset: u64) -> std::result::Result<u32, String> {
            self.values
                .get(&offset)
                .copied()
                .ok_or_else(|| format!("unmapped {offset:#x}"))
        }

        fn write_u32(&mut self, offset: u64, value: u32) -> std::result::Result<(), String> {
            self.values.insert(offset, value);
            Ok(())
        }
    }

    #[test]
    fn default_policy_is_on_demand() {
        let pm = PowerManager::new(MockRegs::warm(), "ffff:ff:ff.f");
        let engine = PolicyEngine::new(pm, 1);
        assert!(matches!(engine.policy(), PowerPolicy::OnDemand { .. }));
    }

    #[test]
    fn warm_request_tracking() {
        let pm = PowerManager::new(MockRegs::warm(), "ffff:ff:ff.f");
        let mut engine = PolicyEngine::new(pm, 1);
        assert_eq!(engine.warm_requests(), 0);

        // request_warm will fail on mock PCI but that's fine for tracking
        let _ = engine.request_warm();
        assert_eq!(engine.warm_requests(), 1);

        let _ = engine.request_warm();
        assert_eq!(engine.warm_requests(), 2);

        engine.release_warm();
        assert_eq!(engine.warm_requests(), 1);

        engine.release_warm();
        assert_eq!(engine.warm_requests(), 0);

        engine.release_warm(); // saturating
        assert_eq!(engine.warm_requests(), 0);
    }

    #[test]
    fn policy_display() {
        assert_eq!(
            PowerPolicy::OnDemand {
                idle_timeout_secs: 60
            }
            .to_string(),
            "OnDemand(60s)"
        );
        assert_eq!(PowerPolicy::AlwaysWarm.to_string(), "AlwaysWarm");
        assert_eq!(PowerPolicy::AlwaysSovereign.to_string(), "AlwaysSovereign");
        assert_eq!(PowerPolicy::Eco.to_string(), "Eco");
    }

    #[test]
    fn custom_policy_display() {
        let policy = PowerPolicy::Custom {
            target_state: GpuPowerState::Glow,
            clock_gating: ClockGateConfig {
                idle_delay: 4,
                idle_cg_en: true,
                stall_cg_en: false,
                wakeup_delay: 0,
            },
        };
        let s = policy.to_string();
        assert!(s.starts_with("Custom(Glow"));
    }

    #[test]
    fn set_policy_resets_idle() {
        let pm = PowerManager::new(MockRegs::warm(), "ffff:ff:ff.f");
        let mut engine = PolicyEngine::new(pm, 1);
        engine.idle_ticks = 100;
        engine.set_policy(PowerPolicy::AlwaysWarm);
        assert_eq!(engine.idle_ticks, 0);
    }
}
