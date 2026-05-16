// SPDX-License-Identifier: AGPL-3.0-or-later

//! PMU falcon bootstrap for Kepler-class GPUs.
//!
//! The PMU (Power Management Unit) falcon controls clock gating, power
//! management, and critically gates PFIFO enable. Without PMU firmware
//! running, writes to `PFIFO_ENABLE` (0x2200) are silently ignored.
//!
//! On Kepler (GK110/GK210), the PMU does NOT require signed firmware
//! (no ACR/HS). The init sequence is:
//!
//! ```text
//! 1. Reset PMU falcon (CPUCTL = 0x02)
//! 2. Upload IMEM (instruction memory) via PIO
//! 3. Upload DMEM (data memory) via PIO
//! 4. Set boot vector (BOOTVEC)
//! 5. Start CPU (CPUCTL bit 1)
//! 6. Wait for mailbox handshake
//! 7. Verify PFIFO_ENABLE becomes writable
//! ```
//!
//! This module provides the `PmuBootstrap` type that captures the PMU
//! init sequence from warm-state observations and replays it.

use serde::{Deserialize, Serialize};

use crate::nv::driver_probe::FalconState;
use crate::nv::gr_init::ChipFamily;
use crate::nv::pri::is_pri_fault;
use crate::vfio::device::MappedBar;

/// PMU falcon register offsets (BAR0-absolute).
///
/// See also `vfio::channel::devinit::pmu::pmu_reg` for the devinit-era naming
/// (FALCON_CTRL, FALCON_PC, etc.) which covers the same physical registers.
pub mod pmu_reg {
    /// PMU falcon base address in BAR0.
    pub const BASE: usize = 0x0010_A000;
    /// Interrupt set register.
    pub const IRQSSET: usize = 0x0010_A000;
    /// Interrupt clear register.
    pub const IRQSCLR: usize = 0x0010_A004;
    /// Interrupt status.
    pub const IRQSTAT: usize = 0x0010_A008;
    /// Interrupt mask set.
    pub const IRQMSET: usize = 0x0010_A010;
    /// Interrupt mask clear.
    pub const IRQMCLR: usize = 0x0010_A014;
    /// Mailbox 0 (handshake).
    pub const MAILBOX0: usize = 0x0010_A040;
    /// Mailbox 1.
    pub const MAILBOX1: usize = 0x0010_A044;
    /// OS register.
    pub const OS: usize = 0x0010_A080;
    /// CPU control (start/halt/reset).
    pub const CPUCTL: usize = 0x0010_A100;
    /// Boot vector (entry point).
    pub const BOOTVEC: usize = 0x0010_A104;
    /// Hardware config (IMEM/DMEM sizes, security).
    pub const HWCFG: usize = 0x0010_A108;
    /// DMA control.
    pub const DMACTL: usize = 0x0010_A10C;
    /// Engine control.
    pub const ENGCTL: usize = 0x0010_A110;
    /// Current context.
    pub const CURCTX: usize = 0x0010_A118;
    /// IMEM port (PIO upload).
    pub const IMEMC: usize = 0x0010_A180;
    /// IMEM data (PIO upload).
    pub const IMEMD: usize = 0x0010_A184;
    /// IMEM tag (PIO upload).
    pub const IMEMT: usize = 0x0010_A188;
    /// DMEM port (PIO upload).
    pub const DMEMC: usize = 0x0010_A1C0;
    /// DMEM data (PIO upload).
    pub const DMEMD: usize = 0x0010_A1C4;
    /// Program counter.
    pub const PC: usize = 0x0010_A130;
    /// SCTL (security control, HS lock indicator).
    pub const SCTL: usize = 0x0010_A240;
}

/// PFIFO register offsets.
pub mod pfifo_reg {
    /// PFIFO enable register.
    pub const ENABLE: usize = 0x0000_2200;
    /// PFIFO scheduler enable.
    pub const SCHED_EN: usize = 0x0000_2204;
}

/// Observed PMU state from a warm GPU.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PmuSnapshot {
    /// CPUCTL register value.
    pub cpuctl: u32,
    /// BOOTVEC register value.
    pub bootvec: u32,
    /// HWCFG register value (IMEM/DMEM sizes).
    pub hwcfg: u32,
    /// OS register value.
    pub os: u32,
    /// Mailbox 0 value.
    pub mailbox0: u32,
    /// Mailbox 1 value.
    pub mailbox1: u32,
    /// Program counter.
    pub pc: u32,
    /// SCTL (security control).
    pub sctl: u32,
    /// PFIFO_ENABLE value.
    pub pfifo_enable: u32,
    /// Whether PMU appears to be running.
    pub is_running: bool,
    /// Whether PFIFO is enabled.
    pub pfifo_enabled: bool,
}

impl PmuSnapshot {
    /// Capture PMU state from a mapped BAR0.
    pub fn capture(bar0: &MappedBar) -> Self {
        let r = |reg: usize| bar0.read_u32(reg).unwrap_or(0xDEAD_DEAD);

        let cpuctl = r(pmu_reg::CPUCTL);
        let bootvec = r(pmu_reg::BOOTVEC);
        let hwcfg = r(pmu_reg::HWCFG);
        let os = r(pmu_reg::OS);
        let mailbox0 = r(pmu_reg::MAILBOX0);
        let mailbox1 = r(pmu_reg::MAILBOX1);
        let pc = r(pmu_reg::PC);
        let sctl = r(pmu_reg::SCTL);
        let pfifo_enable = r(pfifo_reg::ENABLE);

        let halted = cpuctl & (1 << 4) != 0;
        let is_running = cpuctl != 0 && !halted && !is_pri_fault(cpuctl);
        let pfifo_enabled = pfifo_enable & 1 != 0;

        Self {
            cpuctl,
            bootvec,
            hwcfg,
            os,
            mailbox0,
            mailbox1,
            pc,
            sctl,
            pfifo_enable,
            is_running,
            pfifo_enabled,
        }
    }

    /// Get the PMU falcon state as a `FalconState` enum.
    ///
    /// Bridges the snapshot's raw register values into the shared
    /// falcon state abstraction used by `driver_probe`.
    pub fn falcon_state(&self) -> FalconState {
        if is_pri_fault(self.cpuctl) {
            return FalconState::PriGated;
        }
        if self.sctl & 0x02 != 0 {
            return FalconState::HsLocked { sctl: self.sctl };
        }
        if self.cpuctl == 0 {
            return FalconState::NotStarted;
        }
        let halted = self.cpuctl & (1 << 4) != 0;
        if halted {
            FalconState::Halted { pc: self.pc }
        } else {
            FalconState::Running { pc: self.pc }
        }
    }

    /// IMEM size in KB from HWCFG.
    pub fn imem_size_kb(&self) -> u32 {
        ((self.hwcfg >> 16) & 0x1FF) * 256 / 1024
    }

    /// DMEM size in KB from HWCFG.
    pub fn dmem_size_kb(&self) -> u32 {
        (self.hwcfg & 0x1FF) * 256 / 1024
    }

    /// Whether security (HS) is required from HWCFG bit 8.
    pub fn requires_signed(&self) -> bool {
        self.hwcfg & (1 << 8) != 0
    }

    /// Summary string.
    pub fn summary(&self) -> String {
        format!(
            "PMU: cpuctl={:#x} bootvec={:#x} pc={:#x} running={} \
             pfifo={} imem={}KB dmem={}KB signed={}",
            self.cpuctl,
            self.bootvec,
            self.pc,
            self.is_running,
            self.pfifo_enabled,
            self.imem_size_kb(),
            self.dmem_size_kb(),
            self.requires_signed(),
        )
    }
}

/// PMU bootstrap configuration for Kepler.
///
/// Captures the essential parameters needed to reset, upload firmware to,
/// and start the PMU falcon. The firmware bytes themselves are provided
/// at boot time (extracted from VBIOS or captured from a warm state).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PmuBootstrap {
    /// Target chip family.
    pub chip: ChipFamily,
    /// Boot vector address (entry point for IMEM execution).
    pub boot_vector: u32,
    /// Mailbox handshake value to set before STARTCPU.
    pub mailbox_init: u32,
    /// Expected mailbox completion pattern (mask).
    pub mailbox_done_mask: u32,
    /// Timeout for mailbox handshake in milliseconds.
    pub timeout_ms: u64,
}

impl Default for PmuBootstrap {
    fn default() -> Self {
        Self {
            chip: ChipFamily::Kepler,
            boot_vector: 0,
            mailbox_init: 0x0000_5000,
            mailbox_done_mask: 0x2000,
            timeout_ms: 2000,
        }
    }
}

/// Result of a PMU bootstrap attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PmuBootResult {
    /// Whether the bootstrap succeeded.
    pub success: bool,
    /// PMU state after bootstrap.
    pub post_state: PmuSnapshot,
    /// Whether PFIFO became writable after PMU boot.
    pub pfifo_unlocked: bool,
    /// Duration of the bootstrap in milliseconds.
    pub duration_ms: u64,
    /// Detail message.
    pub detail: String,
}

impl PmuBootstrap {
    /// Create a Kepler PMU bootstrap with default parameters.
    pub fn kepler() -> Self {
        Self {
            chip: ChipFamily::Kepler,
            ..Default::default()
        }
    }

    /// Create from a warm-state PMU snapshot (learn the boot vector from
    /// what the driver left running).
    pub fn from_warm_snapshot(chip: ChipFamily, snapshot: &PmuSnapshot) -> Self {
        Self {
            chip,
            boot_vector: snapshot.bootvec,
            mailbox_init: 0x0000_5000,
            mailbox_done_mask: 0x2000,
            timeout_ms: 2000,
        }
    }

    /// Check preconditions for PMU bootstrap.
    pub fn check_preconditions(&self, bar0: &MappedBar) -> Result<String, String> {
        let snapshot = PmuSnapshot::capture(bar0);

        if is_pri_fault(snapshot.cpuctl) {
            return Err("PMU registers PRI-faulted — PMU engine not enabled in PMC".into());
        }

        if snapshot.requires_signed() && !self.chip.allows_unsigned_falcon() {
            return Err(format!(
                "PMU requires signed firmware (hwcfg={:#x}) and {:?} doesn't allow unsigned",
                snapshot.hwcfg, self.chip,
            ));
        }

        if snapshot.is_running {
            return Ok(format!(
                "PMU already running (cpuctl={:#x} pc={:#x}), may need reset first",
                snapshot.cpuctl, snapshot.pc,
            ));
        }

        Ok(format!(
            "PMU ready for bootstrap: halted={} imem={}KB dmem={}KB",
            snapshot.cpuctl & (1 << 4) != 0,
            snapshot.imem_size_kb(),
            snapshot.dmem_size_kb(),
        ))
    }

    /// Reset the PMU falcon to prepare for firmware upload.
    pub fn reset_falcon(&self, bar0: &MappedBar) {
        let _ = bar0.write_u32(pmu_reg::CPUCTL, 0x02);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    /// Upload firmware to PMU IMEM via PIO.
    ///
    /// `firmware` is the raw instruction memory blob. `secure` marks
    /// the upload as secure-mode (needed on some firmware layouts).
    pub fn upload_imem(&self, bar0: &MappedBar, addr: u32, firmware: &[u8], secure: bool) {
        crate::nv::falcon_pio::falcon_upload_imem(bar0, pmu_reg::BASE, addr, firmware, secure);
    }

    /// Upload data to PMU DMEM via PIO.
    pub fn upload_dmem(&self, bar0: &MappedBar, addr: u32, data: &[u8]) {
        crate::nv::falcon_pio::falcon_upload_dmem(bar0, pmu_reg::BASE, addr, data);
    }

    /// Start the PMU falcon and wait for mailbox handshake.
    pub fn start_and_wait(&self, bar0: &MappedBar) -> PmuBootResult {
        let start = std::time::Instant::now();

        // Set boot vector
        let _ = bar0.write_u32(pmu_reg::BOOTVEC, self.boot_vector);

        // Set mailbox init value
        let _ = bar0.write_u32(pmu_reg::MAILBOX0, self.mailbox_init);

        // Start CPU
        let _ = bar0.write_u32(pmu_reg::CPUCTL, 0x02);

        // Wait for mailbox handshake
        let timeout = std::time::Duration::from_millis(self.timeout_ms);
        let mut completed = false;

        while start.elapsed() < timeout {
            let mbox = bar0.read_u32(pmu_reg::MAILBOX0).unwrap_or(0);
            if mbox & self.mailbox_done_mask != 0 {
                completed = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let duration_ms = start.elapsed().as_millis() as u64;
        let post_state = PmuSnapshot::capture(bar0);

        // Test PFIFO writability
        let pfifo_before = bar0.read_u32(pfifo_reg::ENABLE).unwrap_or(0);
        let _ = bar0.write_u32(pfifo_reg::ENABLE, pfifo_before | 1);
        let pfifo_after = bar0.read_u32(pfifo_reg::ENABLE).unwrap_or(0);
        let pfifo_unlocked = (pfifo_after & 1) != 0;

        // Restore PFIFO state if we were just testing
        if !pfifo_unlocked {
            let _ = bar0.write_u32(pfifo_reg::ENABLE, pfifo_before);
        }

        let detail = if completed {
            format!(
                "PMU booted in {duration_ms}ms, PFIFO {}",
                if pfifo_unlocked { "unlocked" } else { "still locked" },
            )
        } else {
            format!(
                "PMU mailbox timeout after {duration_ms}ms (mbox0={:#x})",
                post_state.mailbox0,
            )
        };

        PmuBootResult {
            success: completed,
            post_state,
            pfifo_unlocked,
            duration_ms,
            detail,
        }
    }

    /// Full bootstrap: reset → upload IMEM → upload DMEM → start → wait.
    pub fn full_boot(
        &self,
        bar0: &MappedBar,
        imem: &[u8],
        dmem: &[u8],
    ) -> PmuBootResult {
        self.reset_falcon(bar0);
        self.upload_imem(bar0, 0, imem, false);
        if !dmem.is_empty() {
            self.upload_dmem(bar0, 0, dmem);
        }
        self.start_and_wait(bar0)
    }
}

impl std::fmt::Display for PmuBootstrap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PmuBootstrap({:?}, bootvec={:#x}, timeout={}ms)",
            self.chip, self.boot_vector, self.timeout_ms,
        )
    }
}

impl std::fmt::Display for PmuBootResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PmuBootResult(success={}, pfifo={}, {}ms)",
            self.success,
            if self.pfifo_unlocked { "unlocked" } else { "locked" },
            self.duration_ms,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pmu_bootstrap_default() {
        let pmu = PmuBootstrap::default();
        assert_eq!(pmu.chip, ChipFamily::Kepler);
        assert_eq!(pmu.boot_vector, 0);
        assert_eq!(pmu.mailbox_init, 0x0000_5000);
        assert_eq!(pmu.mailbox_done_mask, 0x2000);
        assert_eq!(pmu.timeout_ms, 2000);
    }

    #[test]
    fn pmu_bootstrap_kepler() {
        let pmu = PmuBootstrap::kepler();
        assert_eq!(pmu.chip, ChipFamily::Kepler);
    }

    #[test]
    fn pmu_bootstrap_from_warm_snapshot() {
        let snapshot = PmuSnapshot {
            cpuctl: 0x10,
            bootvec: 0x1234,
            hwcfg: 0x0002_0100,
            os: 0,
            mailbox0: 0x7000,
            mailbox1: 0,
            pc: 0x5678,
            sctl: 0,
            pfifo_enable: 1,
            is_running: false,
            pfifo_enabled: true,
        };
        let pmu = PmuBootstrap::from_warm_snapshot(ChipFamily::Kepler, &snapshot);
        assert_eq!(pmu.boot_vector, 0x1234);
        assert_eq!(pmu.chip, ChipFamily::Kepler);
    }

    #[test]
    fn pmu_snapshot_sizes() {
        let snapshot = PmuSnapshot {
            cpuctl: 0,
            bootvec: 0,
            hwcfg: 0x0040_0100,
            os: 0,
            mailbox0: 0,
            mailbox1: 0,
            pc: 0,
            sctl: 0,
            pfifo_enable: 0,
            is_running: false,
            pfifo_enabled: false,
        };
        assert_eq!(snapshot.imem_size_kb(), 16);
        assert_eq!(snapshot.dmem_size_kb(), 64);
    }

    #[test]
    fn pmu_snapshot_signed_check() {
        let unsigned = PmuSnapshot {
            cpuctl: 0,
            bootvec: 0,
            hwcfg: 0x0000_0000,
            os: 0,
            mailbox0: 0,
            mailbox1: 0,
            pc: 0,
            sctl: 0,
            pfifo_enable: 0,
            is_running: false,
            pfifo_enabled: false,
        };
        assert!(!unsigned.requires_signed());

        let signed = PmuSnapshot {
            hwcfg: 0x0000_0100,
            ..unsigned.clone()
        };
        assert!(signed.requires_signed());
    }

    #[test]
    fn pmu_snapshot_summary() {
        let snapshot = PmuSnapshot {
            cpuctl: 0x10,
            bootvec: 0x100,
            hwcfg: 0x0020_0080,
            os: 0,
            mailbox0: 0x7000,
            mailbox1: 0,
            pc: 0x42,
            sctl: 0,
            pfifo_enable: 1,
            is_running: false,
            pfifo_enabled: true,
        };
        let s = snapshot.summary();
        assert!(s.contains("PMU"));
        assert!(s.contains("pfifo=true"));
    }

    #[test]
    fn pmu_bootstrap_display() {
        let pmu = PmuBootstrap::kepler();
        let s = format!("{pmu}");
        assert!(s.contains("Kepler"));
        assert!(s.contains("bootvec=0x0"));
    }

    #[test]
    fn pmu_boot_result_display() {
        let result = PmuBootResult {
            success: true,
            post_state: PmuSnapshot {
                cpuctl: 0,
                bootvec: 0,
                hwcfg: 0,
                os: 0,
                mailbox0: 0x7000,
                mailbox1: 0,
                pc: 0x42,
                sctl: 0,
                pfifo_enable: 1,
                is_running: true,
                pfifo_enabled: true,
            },
            pfifo_unlocked: true,
            duration_ms: 150,
            detail: "test".into(),
        };
        let s = format!("{result}");
        assert!(s.contains("success=true"));
        assert!(s.contains("unlocked"));
        assert!(s.contains("150ms"));
    }

    #[test]
    fn pmu_snapshot_serde_roundtrip() {
        let snapshot = PmuSnapshot {
            cpuctl: 0x10,
            bootvec: 0x100,
            hwcfg: 0x0020_0080,
            os: 0,
            mailbox0: 0x7000,
            mailbox1: 0,
            pc: 0x42,
            sctl: 0,
            pfifo_enable: 1,
            is_running: false,
            pfifo_enabled: true,
        };
        let json = serde_json::to_string(&snapshot).unwrap();
        let back: PmuSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(back.cpuctl, snapshot.cpuctl);
        assert_eq!(back.bootvec, snapshot.bootvec);
        assert_eq!(back.pfifo_enabled, snapshot.pfifo_enabled);
    }

    #[test]
    fn pmu_bootstrap_serde_roundtrip() {
        let pmu = PmuBootstrap {
            chip: ChipFamily::Kepler,
            boot_vector: 0x1234,
            mailbox_init: 0x5000,
            mailbox_done_mask: 0x2000,
            timeout_ms: 3000,
        };
        let json = serde_json::to_string(&pmu).unwrap();
        let back: PmuBootstrap = serde_json::from_str(&json).unwrap();
        assert_eq!(back.boot_vector, 0x1234);
        assert_eq!(back.timeout_ms, 3000);
    }
}
