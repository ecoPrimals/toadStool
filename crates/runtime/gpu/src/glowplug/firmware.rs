// SPDX-License-Identifier: AGPL-3.0-or-later

//! GPU firmware interface — direct FECS/GPCCS/PMU register access.
//!
//! Implements [`FirmwareInterface`] using toadStool's own `nvpmu::Bar0Access`
//! (backed by `hw-safe::SafeMmapRegion`) for direct BAR0 MMIO reads. toadStool
//! IS the hardware primal — no proxy to external primals needed.
//!
//! The firmware boundary is at the Falcon microcontroller registers:
//! FECS, GPCCS, and PMU are treated as firmware interfaces we observe
//! but do not try to replace. We read their state for health monitoring,
//! orchestration decisions, and capability reporting.
//!
//! ## Register Map (NVIDIA Falcon engines on BAR0)
//!
//! Each Falcon engine has a 0x1000-byte register block with:
//! - `+0x100` CPUCTL — execution control (run/halt/restart)
//! - `+0x104` BOOTVEC or current PC (varies by engine generation)
//!
//! Standard engine base offsets:
//! - FECS: `0x409000` (Front-End Context Switch)
//! - GPCCS: `0x41A000` (GPC Context Switch)
//! - PMU: `0x10A000` (Power Management Unit)

use std::fmt;

use serde::{Deserialize, Serialize};
use toadstool_glowplug::firmware::FirmwareInterface;

/// Falcon engine register block offsets within BAR0.
mod regs {
    /// FECS (Front-End Context Switch) engine base.
    pub const FECS_BASE: u64 = 0x0040_9000;
    /// GPCCS (GPC Context Switch) engine base.
    pub const GPCCS_BASE: u64 = 0x0041_A000;
    /// PMU (Power Management Unit) engine base.
    pub const PMU_BASE: u64 = 0x0010_A000;

    /// CPUCTL register offset within a Falcon block.
    pub const FALCON_CPUCTL: u64 = 0x100;
    /// PC / BOOTVEC register offset within a Falcon block.
    pub const FALCON_PC: u64 = 0x104;

    /// CPUCTL bit 5: engine is halted (software halt / context-switch freeze).
    /// Bit 4 (0x10) is HRESET — distinct from HALTED.
    pub const CPUCTL_HALTED: u32 = 0x20;
}

/// Status snapshot of a GPU's Falcon firmware engines.
///
/// Read-only snapshot of FECS, GPCCS, and PMU register state,
/// captured from direct BAR0 MMIO reads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuFirmwareStatus {
    /// FECS (Front-End Context Switch) engine state.
    pub fecs: Option<FalconState>,
    /// GPCCS (Graphics Processing Cluster Context Switch) engine state.
    pub gpccs: Option<FalconState>,
    /// PMU (Power Management Unit) engine state.
    pub pmu: Option<FalconState>,
}

/// State of a single Falcon microcontroller.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FalconState {
    /// CPUCTL register value (execution state).
    pub cpuctl: u32,
    /// Program counter.
    pub pc: u32,
    /// Whether the engine appears halted.
    pub halted: bool,
}

/// Command that can be sent to GPU firmware (future use).
#[derive(Debug)]
pub enum GpuFirmwareCommand {
    /// Request a firmware status refresh.
    RefreshStatus,
}

/// Error for firmware operations.
#[derive(Debug, thiserror::Error)]
pub enum GpuFirmwareError {
    /// No BAR0 access available (device not bound, no permissions).
    #[error("BAR0 unavailable: {0}")]
    Bar0Unavailable(String),
    /// A register read failed (out of bounds, device hung).
    #[error("register read failed: {0}")]
    RegisterReadFailed(String),
}

/// Direct GPU firmware interface via BAR0 MMIO.
///
/// Reads Falcon engine registers through toadStool's own `hw-safe`
/// backed BAR0 mapping. No external primal dependency.
pub struct GpuFirmwareAccess {
    bdf: String,
}

impl fmt::Debug for GpuFirmwareAccess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GpuFirmwareAccess")
            .field("bdf", &self.bdf)
            .finish()
    }
}

impl GpuFirmwareAccess {
    /// Create a firmware interface for the GPU at the given PCI BDF address.
    ///
    /// BAR0 is mapped lazily on first `probe_status` call.
    #[must_use]
    pub fn new(bdf: String) -> Self {
        Self { bdf }
    }

    /// Create a firmware access that will fail (no BDF configured).
    #[must_use]
    pub fn unavailable() -> Self {
        Self { bdf: String::new() }
    }

    /// Read a single Falcon engine's state from BAR0.
    fn read_falcon(bar0: &nvpmu::Bar0Access, base: u64) -> Result<FalconState, GpuFirmwareError> {
        let cpuctl = bar0
            .read_u32(base + regs::FALCON_CPUCTL)
            .map_err(|e| GpuFirmwareError::RegisterReadFailed(e.to_string()))?;
        let pc = bar0
            .read_u32(base + regs::FALCON_PC)
            .map_err(|e| GpuFirmwareError::RegisterReadFailed(e.to_string()))?;
        let halted = cpuctl & regs::CPUCTL_HALTED != 0;

        Ok(FalconState { cpuctl, pc, halted })
    }
}

impl FirmwareInterface for GpuFirmwareAccess {
    type Status = GpuFirmwareStatus;
    type Command = GpuFirmwareCommand;
    type Error = GpuFirmwareError;

    fn probe_status(&self) -> Result<Self::Status, Self::Error> {
        if self.bdf.is_empty() {
            return Err(GpuFirmwareError::Bar0Unavailable(
                "no BDF configured".into(),
            ));
        }

        let bar0 = nvpmu::Bar0Access::open(&self.bdf)
            .map_err(|e| GpuFirmwareError::Bar0Unavailable(format!("{}: {e}", self.bdf)))?;

        let fecs = Self::read_falcon(&bar0, regs::FECS_BASE).ok();
        let gpccs = Self::read_falcon(&bar0, regs::GPCCS_BASE).ok();
        let pmu = Self::read_falcon(&bar0, regs::PMU_BASE).ok();

        Ok(GpuFirmwareStatus { fecs, gpccs, pmu })
    }

    fn send_command(&self, _cmd: Self::Command) -> Result<(), Self::Error> {
        if self.bdf.is_empty() {
            return Err(GpuFirmwareError::Bar0Unavailable(
                "no BDF configured".into(),
            ));
        }
        // RefreshStatus is a no-op — next probe_status will re-read.
        Ok(())
    }

    fn firmware_version(&self) -> Option<String> {
        if self.bdf.is_empty() {
            return None;
        }
        // FECS firmware version can be read from BAR0 but requires
        // engine-specific mailbox protocol. Report None until we
        // implement the Falcon mailbox exchange.
        None
    }

    fn is_responsive(&self) -> bool {
        if self.bdf.is_empty() {
            return false;
        }
        // Check if BAR0 resource file exists for this device
        let path = format!("/sys/bus/pci/devices/{}/resource0", self.bdf);
        std::path::Path::new(&path).exists()
    }

    fn engine_name(&self) -> &str {
        "gpu-falcon"
    }
}

impl fmt::Display for GpuFirmwareAccess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.bdf.is_empty() {
            write!(f, "GpuFirmwareAccess(unavailable)")
        } else {
            write!(f, "GpuFirmwareAccess({})", self.bdf)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_access() {
        let access = GpuFirmwareAccess::unavailable();
        assert!(!access.is_responsive());
        assert!(access.probe_status().is_err());
        assert!(access.firmware_version().is_none());
    }

    #[test]
    fn access_with_nonexistent_device() {
        let access = GpuFirmwareAccess::new("0000:ff:00.0".into());
        assert!(!access.is_responsive());
        assert!(access.probe_status().is_err());
    }

    #[test]
    fn engine_name() {
        let access = GpuFirmwareAccess::unavailable();
        assert_eq!(access.engine_name(), "gpu-falcon");
    }

    #[test]
    fn display() {
        let access = GpuFirmwareAccess::new("0000:01:00.0".into());
        assert_eq!(format!("{access}"), "GpuFirmwareAccess(0000:01:00.0)");

        let unavail = GpuFirmwareAccess::unavailable();
        assert_eq!(format!("{unavail}"), "GpuFirmwareAccess(unavailable)");
    }

    #[test]
    fn send_command_unavailable() {
        let access = GpuFirmwareAccess::unavailable();
        assert!(
            access
                .send_command(GpuFirmwareCommand::RefreshStatus)
                .is_err()
        );
    }

    #[test]
    fn falcon_state_serialization() {
        let state = FalconState {
            cpuctl: 0x40,
            pc: 0x1000,
            halted: false,
        };
        let json = serde_json::to_string(&state).unwrap();
        let round: FalconState = serde_json::from_str(&json).unwrap();
        assert_eq!(round.cpuctl, 0x40);
        assert!(!round.halted);
    }
}
