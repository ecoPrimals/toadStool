// SPDX-License-Identifier: AGPL-3.0-or-later
#![expect(
    missing_docs,
    reason = "PMU/FALCON registers mirror hardware; full docs planned"
)]
//! PMU FALCON register definitions, `DevinitStatus`, and `FalconDiagnostic`.

use std::fmt::Write as FmtWrite;

use crate::error::DevinitError;
use crate::vfio::device::MappedBar;

use super::super::vbios::{
    PROM_BASE, PROM_ENABLE_REG, read_vbios_file, read_vbios_prom, read_vbios_sysfs,
};

pub mod pmu_reg {
    pub const FALCON_CTRL: usize = 0x0010_A100;
    pub const FALCON_PC: usize = 0x0010_A104;
    pub const FALCON_TRIG: usize = 0x0010_A10C;
    pub const FALCON_MBOX0: usize = 0x0010_A040;
    pub const FALCON_MBOX1: usize = 0x0010_A044;
    pub const IMEM_PORT: usize = 0x0010_A180;
    pub const IMEM_DATA: usize = 0x0010_A184;
    pub const IMEM_TAG: usize = 0x0010_A188;
    pub const DMEM_PORT: usize = 0x0010_A1C0;
    pub const DMEM_DATA: usize = 0x0010_A1C4;

    pub const DEVINIT_STATUS: usize = 0x0002_240C;
    pub const FALCON_HWCFG: usize = 0x0010_A108;
    pub const FALCON_CPUCTL: usize = 0x0010_A100;
    pub const FALCON_ID: usize = 0x0010_A12C;
}

/// Devinit status check result.
#[derive(Debug, Clone)]
pub struct DevinitStatus {
    pub needs_post: bool,
    /// Whether `devinit_reg` was a real read rather than a bus sentinel.
    ///
    /// When false, nothing here is evidence: the device did not answer, and
    /// `needs_post` is a conservative default rather than a measurement.
    pub readable: bool,
    pub devinit_reg: u32,
    pub pmu_id: u32,
    pub pmu_hwcfg: u32,
    pub pmu_ctrl: u32,
    pub pmu_mbox0: u32,
}

impl DevinitStatus {
    /// Check the GPU's devinit status and PMU FALCON health.
    pub fn probe(bar0: &MappedBar) -> Self {
        let r = |reg| bar0.read_u32(reg).unwrap_or(0xDEAD_DEAD);

        let devinit_reg = r(pmu_reg::DEVINIT_STATUS);

        // Bit 1 means POST complete — but only if the register was actually
        // read. A device that is not answering returns all-ones, which has
        // bit 1 set, so an asleep GPU reports "devinit already complete" and
        // the one operation that would bring it up gets skipped.
        //
        // Observed 2026-08-16 on a Tesla K80: boot0 and DEVINIT_STATUS both
        // read 0xFFFFFFFF, and GDDR5 training was declined on that basis.
        let read = crate::nv::register_read::RegisterRead::classify(devinit_reg);
        let readable = read.is_valid();
        // Unreadable is never "complete". Callers check `readable` to tell
        // "needs POST" apart from "cannot tell".
        let needs_post = read.valid().is_none_or(|v| (v & 2) == 0);

        Self {
            needs_post,
            readable,
            devinit_reg,
            pmu_id: r(pmu_reg::FALCON_ID),
            pmu_hwcfg: r(pmu_reg::FALCON_HWCFG),
            pmu_ctrl: r(pmu_reg::FALCON_CPUCTL),
            pmu_mbox0: r(pmu_reg::FALCON_MBOX0),
        }
    }

    /// Append devinit status lines (shared with [`FalconDiagnostic::print_report`]).
    pub(crate) fn write_summary_lines(&self, s: &mut String) {
        let _ = writeln!(
            s,
            "╠══ DEVINIT STATUS ══════════════════════════════════════════╣"
        );
        let _ = writeln!(s, "║ devinit_reg[0x2240c]  = {:#010x}", self.devinit_reg);
        let _ = writeln!(s, "║ needs_post (bit1==0)  = {}", self.needs_post);
        let _ = writeln!(s, "║ PMU FALCON ID         = {:#010x}", self.pmu_id);
        let _ = writeln!(s, "║ PMU FALCON HWCFG      = {:#010x}", self.pmu_hwcfg);
        let _ = writeln!(s, "║ PMU FALCON CTRL       = {:#010x}", self.pmu_ctrl);
        let _ = writeln!(s, "║ PMU MBOX0             = {:#010x}", self.pmu_mbox0);
        if !self.readable {
            let _ = writeln!(
                s,
                "║ *** DEVINIT STATUS UNREADABLE — device is not answering ***"
            );
            let _ = writeln!(
                s,
                "║     Wake the device to D0 before drawing any conclusion."
            );
        } else if self.needs_post {
            let _ = writeln!(s, "║ *** GPU REQUIRES DEVINIT POST (memory not trained) ***");
        } else {
            let _ = writeln!(
                s,
                "║ GPU devinit already complete — HBM2 should be trained."
            );
        }
    }

    pub fn print_summary(&self) {
        let mut s = String::new();
        self.write_summary_lines(&mut s);
        tracing::info!(summary = %s, "devinit status");
    }

    /// Check if FALCON security bits indicate signed-only firmware is required.
    pub fn requires_signed_firmware(&self) -> bool {
        self.pmu_hwcfg & (1 << 8) != 0
    }

    /// Check if the PMU FALCON is halted (vs running).
    pub fn is_falcon_halted(&self) -> bool {
        self.pmu_ctrl & 0x10 != 0
    }
}

/// Comprehensive PMU FALCON diagnostic report.
#[derive(Debug, Clone)]
pub struct FalconDiagnostic {
    pub status: DevinitStatus,
    pub prom_accessible: bool,
    pub prom_signature: u32,
    pub prom_enable_reg: u32,
    pub secure_boot: bool,
    pub falcon_halted: bool,
    pub falcon_pc: u32,
    pub falcon_mbox1: u32,
    pub imem_size_kb: u32,
    pub dmem_size_kb: u32,
    pub vbios_sources: Vec<(String, bool, String)>,
}

impl FalconDiagnostic {
    /// Run comprehensive FALCON diagnostics.
    pub fn probe(bar0: &MappedBar, bdf: Option<&str>) -> Self {
        let r = |reg| bar0.read_u32(reg).unwrap_or(0xDEAD_DEAD);

        let status = DevinitStatus::probe(bar0);

        let prom_enable_reg = r(PROM_ENABLE_REG);
        let _ = bar0.write_u32(PROM_ENABLE_REG, prom_enable_reg & !1);
        let prom_signature = r(PROM_BASE);
        let prom_accessible = (prom_signature & 0xFFFF) == 0xAA55;
        let _ = bar0.write_u32(PROM_ENABLE_REG, prom_enable_reg);

        let hwcfg = status.pmu_hwcfg;
        let secure_boot = hwcfg & (1 << 8) != 0;
        let falcon_halted = status.pmu_ctrl & 0x10 != 0;
        let falcon_pc = r(pmu_reg::FALCON_PC);
        let falcon_mbox1 = r(pmu_reg::FALCON_MBOX1);

        let imem_size_kb = ((hwcfg >> 16) & 0x1FF) * 256 / 1024;
        let dmem_size_kb = (hwcfg & 0x1FF) * 256 / 1024;

        let mut vbios_sources = Vec::new();

        vbios_sources.push((
            "PROM (BAR0+0x300000)".into(),
            prom_accessible,
            if prom_accessible {
                format!("signature {prom_signature:#010x}")
            } else {
                format!("signature mismatch: {prom_signature:#010x}")
            },
        ));

        if let Some(bdf) = bdf {
            let rom_path = crate::linux_paths::sysfs_pci_device_file(bdf, "rom");
            let sysfs_ok = std::fs::metadata(&rom_path).is_ok();
            vbios_sources.push((
                format!("sysfs ({rom_path})"),
                sysfs_ok,
                if sysfs_ok {
                    "file exists".into()
                } else {
                    "not available".into()
                },
            ));
        }

        if let Some(data_dir) = crate::linux_paths::optional_data_dir() {
            let dump_names = ["vbios_0000_4a_00_0.bin", "vbios_0000_03_00_0.bin"];
            for name in &dump_names {
                let path = format!("{data_dir}/{name}");
                let exists = std::fs::metadata(&path).is_ok();
                let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                vbios_sources.push((
                    format!("file ({path})"),
                    exists,
                    if exists {
                        format!("{} KB", size / 1024)
                    } else {
                        "not found".into()
                    },
                ));
            }
        }

        Self {
            status,
            prom_accessible,
            prom_signature,
            prom_enable_reg,
            secure_boot,
            falcon_halted,
            falcon_pc,
            falcon_mbox1,
            imem_size_kb,
            dmem_size_kb,
            vbios_sources,
        }
    }

    /// Print a human-readable diagnostic report.
    pub fn print_report(&self) {
        let mut s = String::new();
        let _ = writeln!(
            &mut s,
            "╠══ PMU FALCON DIAGNOSTIC ═══════════════════════════════════╣"
        );
        self.status.write_summary_lines(&mut s);
        let _ = writeln!(&mut s, "║");
        let _ = writeln!(&mut s, "║ FALCON Security:");
        let _ = writeln!(&mut s, "║   Secure boot required: {}", self.secure_boot);
        let _ = writeln!(&mut s, "║   FALCON halted: {}", self.falcon_halted);
        let _ = writeln!(&mut s, "║   FALCON PC: {:#010x}", self.falcon_pc);
        let _ = writeln!(&mut s, "║   FALCON MBOX1: {:#010x}", self.falcon_mbox1);
        let _ = writeln!(
            &mut s,
            "║   IMEM: {} KB, DMEM: {} KB",
            self.imem_size_kb, self.dmem_size_kb
        );
        let _ = writeln!(&mut s, "║");
        let _ = writeln!(&mut s, "║ PROM Access:");
        let _ = writeln!(
            &mut s,
            "║   Enable reg (0x1854): {:#010x}",
            self.prom_enable_reg
        );
        let _ = writeln!(
            &mut s,
            "║   PROM signature: {:#010x} ({})",
            self.prom_signature,
            if self.prom_accessible { "OK" } else { "FAIL" }
        );
        let _ = writeln!(&mut s, "║");
        let _ = writeln!(&mut s, "║ VBIOS Sources:");
        for (name, ok, detail) in &self.vbios_sources {
            let _ = writeln!(
                &mut s,
                "║   {} {} — {}",
                if *ok { "✓" } else { "✗" },
                name,
                detail
            );
        }
        let _ = writeln!(&mut s, "║");

        if self.status.needs_post {
            if self.secure_boot {
                let _ = writeln!(&mut s, "║ RECOMMENDATION: PMU requires signed firmware.");
                let _ = writeln!(
                    &mut s,
                    "║   → Use host-side VBIOS interpreter (interpret_boot_scripts)"
                );
                let _ = writeln!(&mut s, "║   → Or use differential replay from oracle card");
            } else if self.prom_accessible {
                let _ = writeln!(&mut s, "║ RECOMMENDATION: FALCON upload should work.");
                let _ = writeln!(&mut s, "║   → Try execute_devinit() with PROM-read VBIOS");
            } else {
                let _ = writeln!(
                    &mut s,
                    "║ RECOMMENDATION: PROM inaccessible, FALCON unsigned."
                );
                if self.vbios_sources.iter().any(|(_, ok, _)| *ok) {
                    let _ = writeln!(&mut s, "║   → Try execute_devinit() with file-based VBIOS");
                } else {
                    let _ = writeln!(
                        &mut s,
                        "║   → No VBIOS source available — try oracle replay"
                    );
                }
            }
        } else {
            let _ = writeln!(
                &mut s,
                "║ RECOMMENDATION: Devinit already complete, no action needed."
            );
        }
        let _ = writeln!(
            &mut s,
            "╚═══════════════════════════════════════════════════════════╝"
        );
        tracing::info!(summary = %s, "PMU FALCON diagnostic");
    }

    /// Find the best available VBIOS ROM, trying all sources.
    pub fn best_vbios(&self, bar0: &MappedBar, bdf: Option<&str>) -> Result<Vec<u8>, DevinitError> {
        if self.prom_accessible
            && let Ok(rom) = read_vbios_prom(bar0)
        {
            return Ok(rom);
        }

        if let Some(bdf) = bdf
            && let Ok(rom) = read_vbios_sysfs(bdf)
        {
            return Ok(rom);
        }

        for (name, ok, _) in &self.vbios_sources {
            if !ok {
                continue;
            }
            if let Some(path) = name
                .strip_prefix("file (")
                .and_then(|s| s.strip_suffix(')'))
                && let Ok(rom) = read_vbios_file(path)
            {
                return Ok(rom);
            }
        }

        Err(DevinitError::NoVbiosSource)
    }
}

#[cfg(test)]
mod devinit_readability_tests {
    use crate::nv::register_read::RegisterRead;

    /// Mirror of the `needs_post` rule in `DevinitStatus::probe`.
    fn needs_post(raw: u32) -> (bool, bool) {
        let read = RegisterRead::classify(raw);
        (read.valid().is_none_or(|v| (v & 2) == 0), read.is_valid())
    }

    /// The Tesla K80 on 2026-08-16: BAR0 dead, every register all-ones.
    /// Bit 1 is set, so the old rule concluded POST was complete and
    /// declined the GDDR5 training the card actually needed.
    #[test]
    fn unreadable_never_reports_post_complete() {
        let (needs, readable) = needs_post(0xFFFF_FFFF);
        assert!(!readable, "all-ones is not a real read");
        assert!(needs, "an unanswering device must not be called POST-complete");
    }

    /// The Titan V on the same day: a genuine 0x2, POST really is done.
    #[test]
    fn real_post_complete_is_respected() {
        let (needs, readable) = needs_post(0x0000_0002);
        assert!(readable);
        assert!(!needs, "bit 1 set on a real read means POST complete");
    }

    /// A real register with bit 1 clear needs POST.
    #[test]
    fn real_post_needed_is_respected() {
        let (needs, readable) = needs_post(0x0000_0000);
        assert!(readable);
        assert!(needs);
    }

    /// A PRI fault is not a status either.
    #[test]
    fn pri_fault_is_not_a_status() {
        let (needs, readable) = needs_post(0xBADF_5040);
        assert!(!readable);
        assert!(needs);
    }
}
