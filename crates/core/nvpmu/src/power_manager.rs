// SPDX-License-Identifier: AGPL-3.0-or-later
//! Five-state sovereign GPU power management.
//!
//! Absorbs hotSpring's glow plug discovery into a proper state machine.
//! Desktop Volta (Titan V) has no PMU firmware — all power management is
//! pure BAR0 register writes. The `PowerManager` reads real hardware state
//! via PMC/PCI/PFIFO registers and performs transitions between five
//! power states.
//!
//! # Power State Model
//!
//! ```text
//! State      | PMC_ENABLE   | PFIFO       | PCIe | Power  | Wake
//! -----------+--------------+-------------+------+--------+--------
//! Sovereign  | 0x5fecdff1   | Channels    | D0   | ~25W   | < 1ms
//! Warm       | 0x5fecdff1   | Enabled     | D0   | ~20W   | ~5ms
//! Glow       | 0x40000020   | Gated       | D0   | ~10W   | ~50ms
//! Sleep      | (0xFFFFFFFF) | (0xFFFFFFFF)| D3   | ~3W    | ~100ms
//! Off        | —            | —           | Off  | 0W     | seconds
//! ```
//!
//! # References
//!
//! - `wateringHole/handoffs/HOTSPRING_GLOWPLUG_SOVEREIGN_POWER_TRIO_HANDOFF_MAR14_2026.md`
//! - `hotSpring/experiments/059_GPU_POWER_MANAGEMENT_DESIGN.md`

use crate::error::{NvPmuError, Result};
use crate::power::{GpuPowerController, PciPowerState};
use crate::registers;
use hw_learn::applicator::RegisterAccess;
use std::time::{Duration, Instant};

/// GPU power state in the five-state sovereign model.
///
/// Ordered from highest power (Sovereign) to lowest (Off).
/// Classification is derived from hardware register reads, not cached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum GpuPowerState {
    /// All engines clocked, channels loaded, instant dispatch.
    Sovereign,
    /// All engines clocked, PFIFO enabled, no active channels.
    Warm,
    /// Engine clocks gated, `PCIe` D0. ~50ms wake to Warm.
    Glow,
    /// `PCIe` D3hot. BAR0 inaccessible. ~100ms wake to Glow.
    Sleep,
    /// Powered off or not present.
    Off,
}

impl std::fmt::Display for GpuPowerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sovereign => write!(f, "Sovereign"),
            Self::Warm => write!(f, "Warm"),
            Self::Glow => write!(f, "Glow"),
            Self::Sleep => write!(f, "Sleep"),
            Self::Off => write!(f, "Off"),
        }
    }
}

/// Bus-level clock gating configuration for `NV_PBUS_EXT_CG` (0x1C00).
///
/// Nouveau leaves these at zero on desktop Volta — untapped headroom.
/// Enabling idle/stall CG can reduce idle power by several watts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ClockGateConfig {
    /// Idle cycles before clock gating activates (4 bits, 0-15).
    pub idle_delay: u8,
    /// Enable idle clock gating.
    pub idle_cg_en: bool,
    /// Enable stall-based clock gating.
    pub stall_cg_en: bool,
    /// Wake-up delay count after clock gate release (4 bits, 0-15).
    pub wakeup_delay: u8,
}

impl ClockGateConfig {
    /// Encode into a u32 value for writing to `NV_PBUS_EXT_CG`.
    #[must_use]
    pub fn encode(self) -> u32 {
        let mut val = u32::from(self.idle_delay & 0x0F) & registers::CG_IDLE_DLY_MASK;
        if self.idle_cg_en {
            val |= registers::CG_IDLE_EN;
        }
        if self.stall_cg_en {
            val |= registers::CG_STALL_EN;
        }
        val |= (u32::from(self.wakeup_delay & 0x0F) << registers::CG_WAKEUP_DLY_SHIFT)
            & registers::CG_WAKEUP_DLY_MASK;
        val
    }

    /// Decode from a register readback value.
    #[must_use]
    pub const fn decode(val: u32) -> Self {
        Self {
            idle_delay: (val & registers::CG_IDLE_DLY_MASK) as u8,
            idle_cg_en: val & registers::CG_IDLE_EN != 0,
            stall_cg_en: val & registers::CG_STALL_EN != 0,
            wakeup_delay: ((val & registers::CG_WAKEUP_DLY_MASK) >> registers::CG_WAKEUP_DLY_SHIFT)
                as u8,
        }
    }
}

/// Sovereign GPU power manager.
///
/// Generic over register access backend — works with both `Bar0Access`
/// (sysfs resource0) and `VfioBar0Access` (VFIO mmap). Composes the
/// PCI-level `GpuPowerController` for D-state transitions with BAR0
/// register writes for engine clock control.
pub struct PowerManager<R> {
    regs: R,
    pci: GpuPowerController,
}

impl<R: RegisterAccess> PowerManager<R> {
    /// Create a new `PowerManager` for a GPU.
    ///
    /// `regs` provides BAR0 register access (sysfs or VFIO).
    /// `bdf` is the PCI bus-device-function address (e.g. "0000:4b:00.0").
    pub fn new(regs: R, bdf: &str) -> Self {
        Self {
            regs,
            pci: GpuPowerController::new(bdf),
        }
    }

    /// Classify current GPU power state from hardware registers.
    ///
    /// Reads `PMC_ENABLE`, `PFIFO_ENABLE`, and PCI power state to determine
    /// which of the five states the GPU is in. Always reads real hardware —
    /// never returns cached state.
    ///
    /// # Errors
    ///
    /// Returns error if PCI power state cannot be read.
    pub fn current_state(&self) -> Result<GpuPowerState> {
        let pci_state = self.pci.power_state()?;

        match pci_state {
            PciPowerState::D3Hot | PciPowerState::D3Cold => return Ok(GpuPowerState::Sleep),
            PciPowerState::Unknown => return Ok(GpuPowerState::Off),
            _ => {}
        }

        let pmc = self
            .regs
            .read_u32(registers::PMC_ENABLE)
            .map_err(|e| NvPmuError::Hardware(format!("PMC_ENABLE read: {e}")))?;

        if pmc == registers::BAR0_D3HOT_SENTINEL {
            return Ok(GpuPowerState::Sleep);
        }

        if pmc == registers::PMC_ENABLE_GATED {
            return Ok(GpuPowerState::Glow);
        }

        let pfifo = self
            .regs
            .read_u32(registers::PFIFO_ENABLE)
            .map_err(|e| NvPmuError::Hardware(format!("PFIFO_ENABLE read: {e}")))?;

        if pfifo == registers::PFIFO_GATED_SENTINEL {
            return Ok(GpuPowerState::Glow);
        }

        // PMC is warm-ish (engines clocked). Distinguish Warm vs Sovereign
        // by checking if PFIFO has active channels. For now, treat any
        // warm PMC + working PFIFO as Warm. Sovereign detection requires
        // checking channel context which is the visualization service's domain.
        Ok(GpuPowerState::Warm)
    }

    /// Transition from Glow to Warm: the "glow plug" sequence.
    ///
    /// Writes `PMC_ENABLE` = `0xFFFF_FFFF` to enable all engine clock domains,
    /// waits for clock stabilization, then verifies. Toggles `PFIFO_ENABLE`
    /// if PFIFO is still gated after PMC warm-up.
    ///
    /// # Errors
    ///
    /// Returns error if register writes fail or the GPU doesn't warm.
    pub fn warm(&mut self) -> Result<()> {
        const PMC_SETTLE_DEADLINE_MS: u64 = 50;
        const PMC_POLL_INTERVAL_US: u64 = 100;

        let state = self.current_state()?;
        match state {
            GpuPowerState::Warm | GpuPowerState::Sovereign => return Ok(()),
            GpuPowerState::Sleep => {
                self.wake()?;
            }
            GpuPowerState::Off => {
                return Err(NvPmuError::PowerTransition {
                    from: state.to_string(),
                    to: "Warm".into(),
                    reason: "GPU is off — cannot warm without power-on".into(),
                });
            }
            GpuPowerState::Glow => {}
        }

        tracing::info!(bdf = %self.pci.bdf(), "glow plug: enabling engine clocks");

        self.regs
            .write_u32(registers::PMC_ENABLE, registers::PMC_ENABLE_ALL)
            .map_err(|e| NvPmuError::Hardware(format!("PMC_ENABLE write: {e}")))?;

        // Poll until PMC readback matches the warm value (faster than a fixed delay when
        // the hardware settles early); cap total wait at 50ms with 100µs sleeps.
        let deadline = Instant::now() + Duration::from_millis(PMC_SETTLE_DEADLINE_MS);
        let pmc_readback = loop {
            let readback = self
                .regs
                .read_u32(registers::PMC_ENABLE)
                .map_err(|e| NvPmuError::Hardware(format!("PMC_ENABLE readback: {e}")))?;
            if readback == registers::PMC_ENABLE_WARM {
                break readback;
            }
            if Instant::now() >= deadline {
                break readback;
            }
            std::thread::sleep(Duration::from_micros(PMC_POLL_INTERVAL_US));
        };

        if pmc_readback == registers::PMC_ENABLE_GATED
            || pmc_readback == registers::BAR0_D3HOT_SENTINEL
        {
            return Err(NvPmuError::RegisterTimeout {
                offset: registers::PMC_ENABLE,
                expected: registers::PMC_ENABLE_WARM,
                got: pmc_readback,
            });
        }

        let pfifo = self
            .regs
            .read_u32(registers::PFIFO_ENABLE)
            .map_err(|e| NvPmuError::Hardware(format!("PFIFO_ENABLE read: {e}")))?;

        if pfifo == registers::PFIFO_GATED_SENTINEL {
            tracing::info!("PFIFO still gated, toggling PFIFO_ENABLE");
            self.regs
                .write_u32(registers::PFIFO_ENABLE, 0)
                .map_err(|e| NvPmuError::Hardware(format!("PFIFO_ENABLE clear: {e}")))?;
            self.regs
                .write_u32(registers::PFIFO_ENABLE, 1)
                .map_err(|e| NvPmuError::Hardware(format!("PFIFO_ENABLE set: {e}")))?;
        }

        tracing::info!(
            bdf = %self.pci.bdf(),
            pmc = format_args!("{pmc_readback:#010x}"),
            "glow plug complete — GPU warm"
        );

        Ok(())
    }

    /// Transition from Warm to Glow: gate engine clocks.
    ///
    /// Writes `PMC_ENABLE` to the gated value, disabling engine clock
    /// domains while keeping the GPU in `PCIe` D0. ~10W idle power.
    ///
    /// # Errors
    ///
    /// Returns error if the GPU is not in Warm state or writes fail.
    pub fn cool(&mut self) -> Result<()> {
        let state = self.current_state()?;
        if state == GpuPowerState::Glow || state == GpuPowerState::Sleep {
            return Ok(());
        }
        if state == GpuPowerState::Off {
            return Err(NvPmuError::PowerTransition {
                from: state.to_string(),
                to: "Glow".into(),
                reason: "GPU is off".into(),
            });
        }

        tracing::info!(bdf = %self.pci.bdf(), "cooling: gating engine clocks");

        self.regs
            .write_u32(registers::PMC_ENABLE, registers::PMC_ENABLE_GATED)
            .map_err(|e| NvPmuError::Hardware(format!("PMC_ENABLE gate: {e}")))?;

        Ok(())
    }

    /// Transition from Glow to Sleep: enter `PCIe` D3hot.
    ///
    /// Enables runtime PM `auto` on the PCI device, allowing Linux to
    /// transition to D3hot. BAR0 becomes inaccessible.
    ///
    /// # Errors
    ///
    /// Returns error if PCI power control write fails.
    pub fn sleep(&mut self) -> Result<()> {
        let state = self.current_state()?;
        if state == GpuPowerState::Sleep {
            return Ok(());
        }
        if state == GpuPowerState::Off {
            return Err(NvPmuError::PowerTransition {
                from: state.to_string(),
                to: "Sleep".into(),
                reason: "GPU is off".into(),
            });
        }

        if state == GpuPowerState::Warm || state == GpuPowerState::Sovereign {
            self.cool()?;
        }

        tracing::info!(bdf = %self.pci.bdf(), "sleep: requesting PCIe D3hot");
        self.pci.allow_d3hot()?;
        Ok(())
    }

    /// Transition from Sleep to Glow: exit `PCIe` D3hot.
    ///
    /// Sets PCI power/control to "on" to force D0 transition.
    /// After D0, the GPU will be in Glow state (engines clock-gated).
    ///
    /// # Errors
    ///
    /// Returns error if PCI power control write fails.
    pub fn wake(&mut self) -> Result<()> {
        let state = self.current_state()?;
        match state {
            GpuPowerState::Glow | GpuPowerState::Warm | GpuPowerState::Sovereign => return Ok(()),
            GpuPowerState::Off => {
                return Err(NvPmuError::PowerTransition {
                    from: state.to_string(),
                    to: "Glow".into(),
                    reason: "GPU is off — cannot wake without power-on".into(),
                });
            }
            GpuPowerState::Sleep => {}
        }

        tracing::info!(bdf = %self.pci.bdf(), "wake: requesting PCIe D0");
        self.pci.prevent_d3hot()?;
        Ok(())
    }

    /// Orchestrate a multi-step transition to the target state.
    ///
    /// Automatically sequences through intermediate states. For example,
    /// Sleep → Warm requires `wake()` then `warm()`.
    ///
    /// # Errors
    ///
    /// Returns error if any transition step fails.
    pub fn set_profile(&mut self, target: GpuPowerState) -> Result<()> {
        let current = self.current_state()?;
        if current == target {
            return Ok(());
        }

        tracing::info!(
            bdf = %self.pci.bdf(),
            from = %current,
            to = %target,
            "power transition"
        );

        match target {
            GpuPowerState::Sovereign => {
                // Sovereign requires channels loaded — warm is the best we can
                // do from nvpmu. Channel loading is the visualization service's domain.
                self.warm()?;
                tracing::info!(
                    "Warm reached. Sovereign requires channel loading (visualization service domain)."
                );
            }
            GpuPowerState::Warm => self.warm()?,
            GpuPowerState::Glow => match current {
                GpuPowerState::Sleep | GpuPowerState::Off => self.wake()?,
                GpuPowerState::Warm | GpuPowerState::Sovereign => self.cool()?,
                GpuPowerState::Glow => {}
            },
            GpuPowerState::Sleep => self.sleep()?,
            GpuPowerState::Off => {
                return Err(NvPmuError::PowerTransition {
                    from: current.to_string(),
                    to: "Off".into(),
                    reason: "software power-off not supported — use IPMI or physical switch".into(),
                });
            }
        }

        Ok(())
    }

    /// Read GPU die temperature directly from BAR0 register.
    ///
    /// Only valid in Glow/Warm/Sovereign states. Returns temperature
    /// in degrees Celsius. In Sleep/Off, BAR0 is inaccessible.
    ///
    /// # Errors
    ///
    /// Returns error if the register read fails (GPU likely in D3hot).
    pub fn read_gpu_temp_c(&self) -> Result<f64> {
        let raw = self
            .regs
            .read_u32(registers::GPU_TEMP)
            .map_err(|e| NvPmuError::Hardware(format!("GPU_TEMP read: {e}")))?;

        if raw == registers::BAR0_D3HOT_SENTINEL {
            return Err(NvPmuError::Hardware(
                "GPU_TEMP returned 0xFFFFFFFF — GPU likely in D3hot".into(),
            ));
        }

        let temp = (raw & registers::GPU_TEMP_MASK) >> registers::GPU_TEMP_SHIFT;
        Ok(f64::from(temp))
    }

    /// Read current bus-level clock gating configuration.
    ///
    /// # Errors
    ///
    /// Returns error if the register read fails.
    pub fn read_clock_gating(&self) -> Result<ClockGateConfig> {
        let val = self
            .regs
            .read_u32(registers::PBUS_EXT_CG)
            .map_err(|e| NvPmuError::Hardware(format!("PBUS_EXT_CG read: {e}")))?;
        Ok(ClockGateConfig::decode(val))
    }

    /// Write bus-level clock gating configuration.
    ///
    /// Writes to `NV_PBUS_EXT_CG` (0x1C00). Nouveau leaves this at zero —
    /// enabling idle/stall CG can reduce idle power.
    ///
    /// # Errors
    ///
    /// Returns error if the register write fails.
    pub fn set_clock_gating(&mut self, config: &ClockGateConfig) -> Result<()> {
        let val = config.encode();
        tracing::info!(
            bdf = %self.pci.bdf(),
            value = format_args!("{val:#010x}"),
            idle_cg = config.idle_cg_en,
            stall_cg = config.stall_cg_en,
            "setting bus-level clock gating"
        );
        self.regs
            .write_u32(registers::PBUS_EXT_CG, val)
            .map_err(|e| NvPmuError::Hardware(format!("PBUS_EXT_CG write: {e}")))?;
        Ok(())
    }

    /// Access the underlying register interface.
    pub const fn registers(&self) -> &R {
        &self.regs
    }

    /// Mutable access to the underlying register interface.
    pub const fn registers_mut(&mut self) -> &mut R {
        &mut self.regs
    }

    /// Access the PCI power controller.
    pub const fn pci(&self) -> &GpuPowerController {
        &self.pci
    }
}

#[cfg(test)]
#[path = "power_manager_tests.rs"]
mod tests;
