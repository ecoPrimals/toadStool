// SPDX-License-Identifier: AGPL-3.0-only
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
//! - `hotSpring/experiments/059_CORALREEF_GPU_POWER_MANAGEMENT_DESIGN.md`

use crate::error::{NvPmuError, Result};
use crate::power::{GpuPowerController, PciPowerState};
use crate::registers;
use hw_learn::applicator::RegisterAccess;

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
        #[allow(
            clippy::cast_possible_truncation,
            reason = "masked to 4 bits, always fits u8"
        )]
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
        // checking channel context which is coralReef's domain.
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

        std::thread::sleep(std::time::Duration::from_millis(50));

        let pmc_readback = self
            .regs
            .read_u32(registers::PMC_ENABLE)
            .map_err(|e| NvPmuError::Hardware(format!("PMC_ENABLE readback: {e}")))?;

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
                // do from nvpmu. Channel loading is coralReef's domain.
                self.warm()?;
                tracing::info!(
                    "Warm reached. Sovereign requires channel loading (coralReef domain)."
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
    #[allow(
        clippy::cast_precision_loss,
        reason = "temperature fits in f64 without precision loss"
    )]
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
mod tests {
    use super::*;

    struct MockRegs {
        values: std::collections::HashMap<u64, u32>,
    }

    impl MockRegs {
        fn new() -> Self {
            Self {
                values: std::collections::HashMap::new(),
            }
        }

        fn with_warm_state() -> Self {
            let mut m = Self::new();
            m.values
                .insert(registers::PMC_ENABLE, registers::PMC_ENABLE_WARM);
            m.values.insert(registers::PFIFO_ENABLE, 0x0000_0000);
            m.values.insert(registers::GPU_TEMP, 0x0000_2E00); // 46 C
            m.values.insert(registers::PBUS_EXT_CG, 0x0000_0000);
            m
        }

        fn with_glow_state() -> Self {
            let mut m = Self::new();
            m.values
                .insert(registers::PMC_ENABLE, registers::PMC_ENABLE_GATED);
            m.values
                .insert(registers::PFIFO_ENABLE, registers::PFIFO_GATED_SENTINEL);
            m.values.insert(registers::GPU_TEMP, 0x0000_2600); // 38 C
            m.values.insert(registers::PBUS_EXT_CG, 0x0000_0000);
            m
        }
    }

    impl RegisterAccess for MockRegs {
        fn read_u32(&self, offset: u64) -> std::result::Result<u32, String> {
            self.values
                .get(&offset)
                .copied()
                .ok_or_else(|| format!("unmapped register {offset:#x}"))
        }

        fn write_u32(&mut self, offset: u64, value: u32) -> std::result::Result<(), String> {
            self.values.insert(offset, value);
            Ok(())
        }
    }

    #[test]
    fn clock_gate_config_roundtrip() {
        let cfg = ClockGateConfig {
            idle_delay: 5,
            idle_cg_en: true,
            stall_cg_en: true,
            wakeup_delay: 3,
        };
        let encoded = cfg.encode();
        let decoded = ClockGateConfig::decode(encoded);
        assert_eq!(cfg, decoded);
    }

    #[test]
    fn clock_gate_config_default_is_zero() {
        assert_eq!(ClockGateConfig::default().encode(), 0);
    }

    #[test]
    fn gpu_temp_read_warm() {
        let regs = MockRegs::with_warm_state();
        let pm = PowerManager::new(regs, "ffff:ff:ff.f");
        let temp = pm.read_gpu_temp_c();
        assert!(temp.is_ok());
        assert!((temp.unwrap() - 46.0).abs() < 0.01);
    }

    #[test]
    fn gpu_temp_read_glow() {
        let regs = MockRegs::with_glow_state();
        let pm = PowerManager::new(regs, "ffff:ff:ff.f");
        let temp = pm.read_gpu_temp_c();
        assert!(temp.is_ok());
        assert!((temp.unwrap() - 38.0).abs() < 0.01);
    }

    #[test]
    fn clock_gate_encode_individual_bits() {
        let idle_only = ClockGateConfig {
            idle_delay: 0,
            idle_cg_en: true,
            stall_cg_en: false,
            wakeup_delay: 0,
        };
        assert_eq!(idle_only.encode(), registers::CG_IDLE_EN);

        let stall_only = ClockGateConfig {
            idle_delay: 0,
            idle_cg_en: false,
            stall_cg_en: true,
            wakeup_delay: 0,
        };
        assert_eq!(stall_only.encode(), registers::CG_STALL_EN);
    }

    #[test]
    fn power_state_display() {
        assert_eq!(GpuPowerState::Sovereign.to_string(), "Sovereign");
        assert_eq!(GpuPowerState::Warm.to_string(), "Warm");
        assert_eq!(GpuPowerState::Glow.to_string(), "Glow");
        assert_eq!(GpuPowerState::Sleep.to_string(), "Sleep");
        assert_eq!(GpuPowerState::Off.to_string(), "Off");
    }
}
