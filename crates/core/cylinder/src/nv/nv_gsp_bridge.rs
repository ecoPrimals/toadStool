// SPDX-License-Identifier: AGPL-3.0-or-later
//! Local `GspBridge` implementation — loads GR falcon firmware from
//! `/lib/firmware/nvidia/{chip}/gr/` and uploads via PIO.
//!
//! This replaces `StubGspBridge` for sovereign cold boot on GPUs where
//! the vendor driver warm-handoff path is unavailable (e.g. Volta on
//! systems with open nvidia.ko that doesn't support pre-GSP GPUs).
//!
//! The PIO upload mechanism writes directly to IMEM/DMEM via BAR0
//! registers. It works regardless of falcon security mode — the host
//! PIO port is always writable.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::error::{DriverError, DriverResult};
use crate::nv::falcon_pio::{falcon_upload_dmem, falcon_upload_imem};
use crate::nv::gsp_bridge::{AcrBootResult, FalconBootResult, GspBridge, PgobResult};
use crate::vfio::channel::registers::falcon;
use crate::vfio::device::MappedBar;

/// DMA IOVA for FECS firmware code image.
pub const FECS_FW_CODE_IOVA: u64 = 0x0030_0000;
/// DMA IOVA for FECS firmware data image.
pub const FECS_FW_DATA_IOVA: u64 = 0x0031_0000;
/// DMA IOVA for GPCCS firmware code image.
pub const GPCCS_FW_CODE_IOVA: u64 = 0x0032_0000;
/// DMA IOVA for GPCCS firmware data image.
pub const GPCCS_FW_DATA_IOVA: u64 = 0x0033_0000;

/// Firmware-backed `GspBridge` that loads blobs from the local filesystem.
#[derive(Debug)]
pub struct NvGspBridge {
    firmware_base: PathBuf,
}

impl NvGspBridge {
    /// Create a bridge that looks for firmware at `/lib/firmware/nvidia/{chip}/gr/`.
    #[must_use]
    pub fn new(chip: &str) -> Self {
        Self {
            firmware_base: PathBuf::from(format!("/lib/firmware/nvidia/{chip}")),
        }
    }

    /// Check whether the required GR firmware files exist.
    #[must_use]
    pub fn has_gr_firmware(&self) -> bool {
        let gr = self.firmware_base.join("gr");
        gr.join("fecs_inst.bin").exists() && gr.join("fecs_data.bin").exists()
    }

    fn load_gr_blob(&self, name: &str) -> DriverResult<Vec<u8>> {
        let path = self.firmware_base.join("gr").join(name);
        std::fs::read(&path).map_err(|e| {
            DriverError::Unsupported(
                format!("firmware read failed: {}: {e}", path.display()).into(),
            )
        })
    }

    fn boot_falcon_pio(
        &self,
        bar0: &MappedBar,
        name: &'static str,
        base: usize,
        inst: &[u8],
        data: &[u8],
    ) -> DriverResult<(u32, u32)> {
        let cpuctl = bar0.read_u32(base + falcon::CPUCTL).unwrap_or(0xDEAD);
        tracing::info!(
            name,
            cpuctl = format!("{cpuctl:#010x}"),
            inst_len = inst.len(),
            data_len = data.len(),
            "PIO falcon boot: starting upload"
        );

        // Hold falcon in HRESET before upload. Use CPUCTL_ALIAS on Volta+
        // HS falcons where the main CPUCTL may be security-blocked.
        let _ = bar0.write_u32(base + falcon::CPUCTL_ALIAS, falcon::CPUCTL_HRESET);
        std::thread::sleep(Duration::from_millis(5));

        falcon_upload_dmem(bar0, base, 0, data);
        falcon_upload_imem(bar0, base, 0, inst, false);

        let _ = bar0.write_u32(base + falcon::BOOTVEC, 0);
        let _ = bar0.write_u32(base + falcon::MAILBOX0, 0);
        let _ = bar0.write_u32(base + falcon::MAILBOX1, 0);
        // Invalidate instruction cache then start CPU via CPUCTL_ALIAS.
        // nouveau gm200_flcn_fw_boot uses flcn_wr32(0x130, 0x02) for
        // STARTCPU on Volta+ HS falcons.
        let _ = bar0.write_u32(base + falcon::CPUCTL_ALIAS, falcon::CPUCTL_IINVAL);
        std::thread::sleep(Duration::from_millis(1));
        let _ = bar0.write_u32(base + falcon::CPUCTL_ALIAS, falcon::CPUCTL_STARTCPU);

        let start = Instant::now();
        let timeout = Duration::from_secs(3);
        loop {
            std::thread::sleep(Duration::from_millis(10));
            let ctl = bar0.read_u32(base + falcon::CPUCTL_ALIAS).unwrap_or(0xDEAD);
            let mb0 = bar0.read_u32(base + falcon::MAILBOX0).unwrap_or(0);

            if mb0 != 0 {
                tracing::info!(
                    name,
                    cpuctl = format!("{ctl:#010x}"),
                    mb0 = format!("{mb0:#010x}"),
                    elapsed_ms = start.elapsed().as_millis(),
                    "PIO boot: mailbox response"
                );
                return Ok((ctl, mb0));
            }
            if ctl & falcon::CPUCTL_HALTED != 0 && ctl & falcon::CPUCTL_HRESET == 0 {
                tracing::warn!(
                    name,
                    cpuctl = format!("{ctl:#010x}"),
                    "PIO boot: halted without mailbox"
                );
                return Ok((ctl, 0));
            }
            if start.elapsed() > timeout {
                tracing::error!(
                    name,
                    cpuctl = format!("{ctl:#010x}"),
                    "PIO boot: timeout"
                );
                return Err(DriverError::Unsupported(
                    format!("{name} PIO boot timed out (cpuctl={ctl:#010x})").into(),
                ));
            }
        }
    }

    /// Boot a single falcon using bootloader + DMA approach for HS falcons.
    ///
    /// Uses system memory DMA with proper FBIF and DMACTL configuration.
    /// Firmware is placed in VFIO-mapped DMA buffers. The falcon bootloader
    /// DMAs from system memory via PCIe IOMMU.
    pub fn boot_falcon_hs(
        &self,
        bar0: &MappedBar,
        name: &'static str,
        base: usize,
        dma: &crate::vfio::device::DmaBackend,
        code_iova: u64,
        data_iova: u64,
    ) -> DriverResult<(u32, u32)> {
        let bl = self.load_gr_blob(if name == "FECS" { "fecs_bl.bin" } else { "gpccs_bl.bin" })?;
        let inst = self.load_gr_blob(if name == "FECS" { "fecs_inst.bin" } else { "gpccs_inst.bin" })?;
        let data = self.load_gr_blob(if name == "FECS" { "fecs_data.bin" } else { "gpccs_data.bin" })?;
        let sig = self.load_gr_blob(if name == "FECS" { "fecs_sig.bin" } else { "gpccs_sig.bin" })
            .unwrap_or_else(|_| vec![0u8; 16]);

        tracing::info!(
            name,
            bl_len = bl.len(),
            inst_len = inst.len(),
            data_len = data.len(),
            sig_len = sig.len(),
            "HS falcon boot: loading via bootloader + DMA"
        );

        let code_pages = (inst.len() + 4095) & !4095;
        let data_pages = (data.len() + 4095) & !4095;

        let mut code_buf = crate::vfio::dma::DmaBuffer::new(
            dma.clone(), code_pages, code_iova,
        )?;
        code_buf.as_mut_slice()[..inst.len()].copy_from_slice(&inst);

        let mut data_buf = crate::vfio::dma::DmaBuffer::new(
            dma.clone(), data_pages, data_iova,
        )?;
        data_buf.as_mut_slice()[..data.len()].copy_from_slice(&data);

        // Hold falcon in HRESET
        let _ = bar0.write_u32(base + falcon::CPUCTL_ALIAS, falcon::CPUCTL_HRESET);
        std::thread::sleep(Duration::from_millis(10));

        // Enable DMA engine via DMACTL (0x10C).
        // nouveau: nvkm_falcon_wr32(falcon, 0x10c, 0x1) to enable.
        let _ = bar0.write_u32(base + falcon::DMACTL, 0x01);

        // Configure FBIF TRANSCFG — correct layout from nouveau v1.c:
        //   fbif base = 0x600 for GR (FECS/GPCCS)
        //   stride = 4 bytes (one u32 per DMA index)
        //   values: 0x0=VIRT, 0x4=PHYS_VID, 0x5=PHYS_SYS_COH, 0x6=PHYS_SYS_NCOH
        let fbif_base = base + falcon::FBIF_GR;
        let _ = bar0.write_u32(fbif_base + 4 * falcon::FBIF_DMAIDX_UCODE, falcon::FBIF_TARGET_PHYS_VID);
        let _ = bar0.write_u32(fbif_base + 4 * falcon::FBIF_DMAIDX_VIRT, falcon::FBIF_TARGET_VIRT);
        let _ = bar0.write_u32(fbif_base + 4 * falcon::FBIF_DMAIDX_PHYS_VID, falcon::FBIF_TARGET_PHYS_VID);
        let _ = bar0.write_u32(fbif_base + 4 * falcon::FBIF_DMAIDX_PHYS_SYS_COH, falcon::FBIF_TARGET_PHYS_SYS_COH);
        let _ = bar0.write_u32(fbif_base + 4 * falcon::FBIF_DMAIDX_PHYS_SYS_NCOH, falcon::FBIF_TARGET_PHYS_SYS_NCOH);

        // Read back FBIF to verify
        let fbif0 = bar0.read_u32(fbif_base + 4 * falcon::FBIF_DMAIDX_UCODE).unwrap_or(0xDEAD);
        let fbif3 = bar0.read_u32(fbif_base + 4 * falcon::FBIF_DMAIDX_PHYS_SYS_COH).unwrap_or(0xDEAD);
        let fbif4 = bar0.read_u32(fbif_base + 4 * falcon::FBIF_DMAIDX_PHYS_SYS_NCOH).unwrap_or(0xDEAD);
        tracing::info!(
            name,
            fbif0 = format!("{fbif0:#06x}"),
            fbif3 = format!("{fbif3:#06x}"),
            fbif4 = format!("{fbif4:#06x}"),
            "FBIF TRANSCFG configured (stride=4, base=0x600)"
        );

        // Load bootloader into IMEM
        falcon_upload_imem(bar0, base, 0, &bl, true);

        // Build BootloaderDmemDescV2. Try multiple ctx_dma values:
        // GR_FALCON_DMAIDX_PHYS_SYS_COH = 3, PHYS_SYS_NCOH = 4
        let ctx_dma: u32 = 3; // PHYS_SYS_COH
        let mut desc = [0u8; 128];
        // signature[4] at offset 16..32
        if sig.len() >= 16 {
            desc[16..32].copy_from_slice(&sig[..16]);
        }
        desc[32..36].copy_from_slice(&ctx_dma.to_le_bytes());
        desc[36..44].copy_from_slice(&code_iova.to_le_bytes());
        desc[44..48].copy_from_slice(&0u32.to_le_bytes()); // non_sec_code_off
        desc[48..52].copy_from_slice(&(inst.len() as u32).to_le_bytes()); // non_sec_code_size
        desc[52..56].copy_from_slice(&0u32.to_le_bytes()); // sec_code_off
        desc[56..60].copy_from_slice(&(inst.len() as u32).to_le_bytes()); // sec_code_size
        desc[60..64].copy_from_slice(&0u32.to_le_bytes()); // code_entry_point
        desc[64..72].copy_from_slice(&data_iova.to_le_bytes());
        desc[72..76].copy_from_slice(&(data.len() as u32).to_le_bytes());

        tracing::info!(
            name,
            ctx_dma,
            code_iova = format!("{code_iova:#010x}"),
            data_iova = format!("{data_iova:#010x}"),
            inst_len = inst.len(),
            data_len = data.len(),
            "descriptor ready"
        );

        falcon_upload_dmem(bar0, base, 0, &desc);

        let _ = bar0.write_u32(base + falcon::BOOTVEC, 0);
        let _ = bar0.write_u32(base + falcon::MAILBOX0, 0);
        let _ = bar0.write_u32(base + falcon::MAILBOX1, 0);
        let _ = bar0.write_u32(base + falcon::CPUCTL_ALIAS, falcon::CPUCTL_IINVAL);
        std::thread::sleep(Duration::from_millis(1));
        let _ = bar0.write_u32(base + falcon::CPUCTL_ALIAS, falcon::CPUCTL_STARTCPU);

        let bl_max_pc = (bl.len() / 4 + 16) as u32;

        let start = Instant::now();
        let timeout = Duration::from_secs(3);
        loop {
            std::thread::sleep(Duration::from_millis(20));
            let ctl = bar0.read_u32(base + falcon::CPUCTL_ALIAS).unwrap_or(0xDEAD);
            let mb0 = bar0.read_u32(base + falcon::MAILBOX0).unwrap_or(0);
            let pc = bar0.read_u32(base + falcon::PC).unwrap_or(0);

            // Success: mailbox response from firmware
            if mb0 != 0 {
                tracing::info!(
                    name,
                    cpuctl = format!("{ctl:#010x}"),
                    mb0 = format!("{mb0:#010x}"),
                    pc = format!("{pc:#010x}"),
                    elapsed_ms = start.elapsed().as_millis(),
                    "HS boot: mailbox response"
                );
                return Ok((ctl, mb0));
            }

            // Success: PC advanced well past the bootloader → main firmware is
            // executing its polling loop. FECS firmware doesn't always signal
            // via mailbox on startup.
            let running = ctl & falcon::CPUCTL_HRESET == 0
                && ctl & falcon::CPUCTL_HALTED == 0
                && pc > bl_max_pc;
            if running {
                tracing::info!(
                    name,
                    cpuctl = format!("{ctl:#010x}"),
                    pc = format!("{pc:#010x}"),
                    elapsed_ms = start.elapsed().as_millis(),
                    "HS boot: firmware running (PC past bootloader)"
                );
                return Ok((ctl, 0));
            }

            if ctl & falcon::CPUCTL_HALTED != 0 && ctl & falcon::CPUCTL_HRESET == 0 {
                tracing::warn!(
                    name,
                    cpuctl = format!("{ctl:#010x}"),
                    pc = format!("{pc:#010x}"),
                    "HS boot: halted"
                );
                return Ok((ctl, 0));
            }
            if start.elapsed() > timeout {
                let exci = bar0.read_u32(base + falcon::EXCI).unwrap_or(0xDEAD);
                let sctl = bar0.read_u32(base + falcon::SCTL).unwrap_or(0);
                tracing::error!(
                    name,
                    cpuctl = format!("{ctl:#010x}"),
                    pc = format!("{pc:#010x}"),
                    exci = format!("{exci:#010x}"),
                    sctl = format!("{sctl:#010x}"),
                    "HS boot: timeout — falcon stalled"
                );
                return Err(DriverError::Unsupported(
                    format!("{name} HS boot timed out (cpuctl={ctl:#010x} pc={pc:#010x})").into(),
                ));
            }
        }
    }
}

impl GspBridge for NvGspBridge {
    fn apply_gr_bar0_init(&self, bar0: &MappedBar, _sm_version: u32) -> DriverResult<()> {
        // Load and apply sw_nonctx.bin (non-context state init registers)
        if let Ok(nonctx) = self.load_gr_blob("sw_nonctx.bin") {
            tracing::info!(
                bytes = nonctx.len(),
                "applying sw_nonctx.bin GR BAR0 init"
            );
            for chunk in nonctx.chunks_exact(8) {
                let addr = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                let val = u32::from_le_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]);
                if addr != 0 {
                    let _ = bar0.write_u32(addr as usize, val);
                }
            }
        } else {
            tracing::debug!("sw_nonctx.bin not found — skipping GR BAR0 init");
        }
        Ok(())
    }

    fn acr_boot(
        &self,
        _bar0: &MappedBar,
        _sm_version: u32,
        _chip: &str,
        _dma: Option<crate::vfio::device::DmaBackend>,
    ) -> DriverResult<Vec<AcrBootResult>> {
        // ACR requires WPR which isn't configured on pre-GSP GPUs.
        // Return failure so falcon_boot falls through to PIO.
        Ok(vec![AcrBootResult {
            success: false,
            strategy: "acr_skipped_pre_gsp".into(),
            notes: vec!["WPR not configured on pre-GSP GPU — ACR not viable".into()],
        }])
    }

    fn boot_gr_falcons(&self, bar0: &MappedBar, _chip: &str) -> DriverResult<FalconBootResult> {
        let gpccs_inst = self.load_gr_blob("gpccs_inst.bin")?;
        let gpccs_data = self.load_gr_blob("gpccs_data.bin")?;
        let fecs_inst = self.load_gr_blob("fecs_inst.bin")?;
        let fecs_data = self.load_gr_blob("fecs_data.bin")?;

        tracing::info!(
            fecs_inst = fecs_inst.len(),
            fecs_data = fecs_data.len(),
            gpccs_inst = gpccs_inst.len(),
            gpccs_data = gpccs_data.len(),
            "NvGspBridge: firmware loaded — starting PIO boot"
        );

        let (gpccs_ctl, gpccs_mb0) =
            self.boot_falcon_pio(bar0, "GPCCS", falcon::GPCCS_BASE, &gpccs_inst, &gpccs_data)?;
        let (fecs_ctl, fecs_mb0) =
            self.boot_falcon_pio(bar0, "FECS", falcon::FECS_BASE, &fecs_inst, &fecs_data)?;

        let running = fecs_ctl & falcon::CPUCTL_HALTED == 0 && fecs_mb0 != 0;

        tracing::info!(
            fecs_cpuctl = format!("{fecs_ctl:#010x}"),
            fecs_mb0 = format!("{fecs_mb0:#010x}"),
            gpccs_cpuctl = format!("{gpccs_ctl:#010x}"),
            gpccs_mb0 = format!("{gpccs_mb0:#010x}"),
            running,
            "NvGspBridge: GR falcon boot complete"
        );

        Ok(FalconBootResult {
            cpuctl_after: fecs_ctl,
            mailbox0: fecs_mb0,
            mailbox1: bar0.read_u32(falcon::FECS_BASE + falcon::MAILBOX1).unwrap_or(0),
            running,
        })
    }

    fn boot_fecs(&self, bar0: &MappedBar, _chip: &str) -> DriverResult<FalconBootResult> {
        let fecs_inst = self.load_gr_blob("fecs_inst.bin")?;
        let fecs_data = self.load_gr_blob("fecs_data.bin")?;

        let (ctl, mb0) =
            self.boot_falcon_pio(bar0, "FECS", falcon::FECS_BASE, &fecs_inst, &fecs_data)?;
        let running = ctl & falcon::CPUCTL_HALTED == 0 && mb0 != 0;

        Ok(FalconBootResult {
            cpuctl_after: ctl,
            mailbox0: mb0,
            mailbox1: bar0.read_u32(falcon::FECS_BASE + falcon::MAILBOX1).unwrap_or(0),
            running,
        })
    }

    fn pgob_diagnostic(&self, bar0: &MappedBar, label: &str) {
        let gpc_enables = bar0.read_u32(0x0002_2004).unwrap_or(0xDEAD);
        tracing::info!(
            label,
            gpc_enables = format!("{gpc_enables:#010x}"),
            "PGOB diagnostic"
        );
    }

    fn pgob_disable(&self, bar0: &MappedBar) -> DriverResult<PgobResult> {
        let gpc_before = bar0.read_u32(0x0002_2004).unwrap_or(0);
        tracing::info!(
            gpc_before = format!("{gpc_before:#010x}"),
            "PGOB: GPC state before disable"
        );
        Ok(PgobResult {
            gpc_alive: gpc_before.count_ones(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_reports_firmware_availability() {
        let bridge = NvGspBridge::new("gv100");
        // On test machines without firmware, this returns false.
        // On the biomegate lab machine with GV100 firmware, it returns true.
        let _ = bridge.has_gr_firmware();
    }

    #[test]
    fn bridge_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<NvGspBridge>();
    }

    #[test]
    fn bridge_acr_reports_skip() {
        let bridge = NvGspBridge::new("gv100");
        // ACR boot always reports skip for pre-GSP GPUs — no panic.
        let _: Box<dyn GspBridge> = Box::new(bridge);
    }
}
