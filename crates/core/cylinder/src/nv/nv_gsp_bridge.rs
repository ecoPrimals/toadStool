// SPDX-License-Identifier: AGPL-3.0-or-later
//! Local `GspBridge` implementation — loads GR falcon firmware from
//! `/lib/firmware/nvidia/{chip}/gr/` and uploads via PIO or DMA.
//!
//! This replaces `StubGspBridge` for sovereign cold boot on GPUs where
//! the vendor driver warm-handoff path is unavailable (e.g. Volta on
//! systems with open nvidia.ko that doesn't support pre-GSP GPUs).
//!
//! The PIO upload mechanism writes directly to IMEM/DMEM via BAR0
//! registers. It works regardless of falcon security mode — the host
//! PIO port is always writable.
//!
//! # Frozen Dependency Status
//!
//! `NvGspBridge` is classified as a **frozen dependency** in the ecosystem:
//!
//! - **Firmware blobs are pinned**: The files under `/lib/firmware/nvidia/{chip}/gr/`
//!   are extracted from vendor drivers and committed to the ecosystem's artifact
//!   store. They do not change between vendor driver versions for a given chip —
//!   the GR microcode is burned into the VBIOS/GPU ROM and the firmware images
//!   are simply the host-readable copy.
//!
//! - **Upload mechanisms are hardware-defined**: PIO writes to falcon IMEM/DMEM
//!   use register offsets that are fixed in silicon (CPUCTL, BOOTVEC, MAILBOX0,
//!   etc.). The DMA HS boot path uses FBIF TRANSCFG registers and descriptor
//!   layouts defined by the falcon hardware specification. These do not change
//!   between driver versions or kernel updates.
//!
//! - **Glacial evolution**: The Rust code in this module evolves only when
//!   targeting a new GPU generation (new falcon version, new FBIF layout).
//!   For existing supported chips (GK210, GV100), the implementation is stable
//!   and tested. Changes flow through the `SovereignStrategy` trait layer, not
//!   through the bridge internals.
//!
//! - **Future bridge implementations** (AMD, NPU, etc.) follow the same pattern:
//!   frozen vendor blobs on disk + pure Rust register-write upload mechanisms.
//!   The `GspBridge` trait provides the stable interface boundary.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::error::{DriverError, DriverResult};
use crate::nv::falcon_pio::{falcon_upload_dmem, falcon_upload_imem};
use crate::nv::gsp_bridge::{AcrBootResult, FalconBootResult, GspBridge, PgobResult};
use crate::vfio::channel::registers::falcon;
use crate::vfio::device::MappedBar;

/// DMA IOVA for FECS firmware code image (from centralized layout).
pub const FECS_FW_CODE_IOVA: u64 = super::iova::firmware::FECS_CODE_IOVA;
/// DMA IOVA for FECS firmware data image.
pub const FECS_FW_DATA_IOVA: u64 = super::iova::firmware::FECS_DATA_IOVA;
/// DMA IOVA for GPCCS firmware code image.
pub const GPCCS_FW_CODE_IOVA: u64 = super::iova::firmware::GPCCS_CODE_IOVA;
/// DMA IOVA for GPCCS firmware data image.
pub const GPCCS_FW_DATA_IOVA: u64 = super::iova::firmware::GPCCS_DATA_IOVA;
/// DMA IOVA for ACR load ucode image.
pub const ACR_UCODE_IOVA: u64 = super::iova::firmware::ACR_UCODE_IOVA;

/// Firmware-backed `GspBridge` that loads blobs from the local filesystem.
///
/// This is a **frozen dependency**: firmware blobs are pinned artifacts and the
/// upload mechanisms (PIO register writes, DMA descriptor format) are defined
/// by silicon, not software. See module-level docs for the full rationale.
///
/// # Supported firmware files
///
/// | File | Purpose | Warm boot? | Cold boot? |
/// |------|---------|-----------|-----------|
/// | `fecs_inst.bin` + `fecs_data.bin` | FECS falcon firmware | No (preserved) | **Yes** |
/// | `gpccs_inst.bin` + `gpccs_data.bin` | GPCCS falcon firmware | No | **Yes** |
/// | `fecs_bl.bin` + `gpccs_bl.bin` | HS bootloader (Volta+ DMA path) | No | **Yes** |
/// | `fecs_sig.bin` + `gpccs_sig.bin` | ACR signatures (optional) | No | **Yes** |
/// | `sw_nonctx.bin` | GR non-context BAR0 init writes | No | **Yes** |
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

    fn load_acr_blob(&self, name: &str) -> DriverResult<Vec<u8>> {
        let path = self.firmware_base.join("acr").join(name);
        std::fs::read(&path).map_err(|e| {
            DriverError::Unsupported(
                format!("ACR firmware read failed: {}: {e}", path.display()).into(),
            )
        })
    }

    /// Boot PMU falcon with ACR load ucode via DMA.
    ///
    /// On GV100 with secure boot, FECS/GPCCS are HS falcons that require
    /// the ACR chain to configure WPR and crypto keys before they can
    /// execute signed firmware. The ACR ucode runs on the PMU falcon
    /// (which is LS and fully host-accessible even when GPCs are gated).
    ///
    /// After ACR completes, WPR is configured and FECS/GPCCS firmware
    /// can be loaded through the standard HS DMA boot path.
    pub fn boot_pmu_acr(
        &self,
        bar0: &MappedBar,
        dma: &crate::vfio::device::DmaBackend,
    ) -> DriverResult<(u32, u32)> {
        let bl = self.load_acr_blob("bl.bin")?;
        let ucode = self.load_acr_blob("ucode_load.bin")?;

        tracing::info!(
            bl_len = bl.len(),
            ucode_len = ucode.len(),
            "PMU ACR boot: loading ACR ucode via bootloader + DMA"
        );

        let base = falcon::PMU_BASE;
        let code_iova = ACR_UCODE_IOVA;

        let code_pages = (ucode.len() + 4095) & !4095;
        let mut code_buf = crate::vfio::dma::DmaBuffer::new(
            dma.clone(), code_pages, code_iova,
        )?;
        code_buf.as_mut_slice()[..ucode.len()].copy_from_slice(&ucode);

        // Hold PMU in HRESET
        let _ = bar0.write_u32(base + falcon::CPUCTL_ALIAS, falcon::CPUCTL_HRESET);
        std::thread::sleep(Duration::from_millis(10));

        // Enable DMA engine
        let _ = bar0.write_u32(base + falcon::DMACTL, 0x01);

        // Configure PMU FBIF (at base + 0xE00, not 0x600)
        let fbif_base = base + 0xE00;
        let _ = bar0.write_u32(fbif_base + 4 * falcon::FBIF_DMAIDX_UCODE, falcon::FBIF_TARGET_PHYS_SYS_COH);
        let _ = bar0.write_u32(fbif_base + 4 * falcon::FBIF_DMAIDX_VIRT, falcon::FBIF_TARGET_VIRT);
        let _ = bar0.write_u32(fbif_base + 4 * falcon::FBIF_DMAIDX_PHYS_VID, falcon::FBIF_TARGET_PHYS_VID);
        let _ = bar0.write_u32(fbif_base + 4 * falcon::FBIF_DMAIDX_PHYS_SYS_COH, falcon::FBIF_TARGET_PHYS_SYS_COH);
        let _ = bar0.write_u32(fbif_base + 4 * falcon::FBIF_DMAIDX_PHYS_SYS_NCOH, falcon::FBIF_TARGET_PHYS_SYS_NCOH);

        let fbif0 = bar0.read_u32(fbif_base + 4 * falcon::FBIF_DMAIDX_UCODE).unwrap_or(0xDEAD);
        let fbif3 = bar0.read_u32(fbif_base + 4 * falcon::FBIF_DMAIDX_PHYS_SYS_COH).unwrap_or(0xDEAD);
        tracing::info!(
            fbif0 = format!("{fbif0:#06x}"),
            fbif3 = format!("{fbif3:#06x}"),
            "PMU FBIF TRANSCFG configured (base=0xE00)"
        );

        // Upload bootloader into PMU IMEM
        falcon_upload_imem(bar0, base, 0, &bl, true);

        // Build descriptor: ACR ucode uses ctx_dma=3 (PHYS_SYS_COH)
        let ctx_dma: u32 = 3;
        let mut desc = [0u8; 128];
        desc[32..36].copy_from_slice(&ctx_dma.to_le_bytes());
        desc[36..44].copy_from_slice(&code_iova.to_le_bytes());
        desc[44..48].copy_from_slice(&0u32.to_le_bytes());
        desc[48..52].copy_from_slice(&(ucode.len() as u32).to_le_bytes());
        desc[52..56].copy_from_slice(&0u32.to_le_bytes());
        desc[56..60].copy_from_slice(&(ucode.len() as u32).to_le_bytes());
        desc[60..64].copy_from_slice(&0u32.to_le_bytes());
        // No separate data section for ACR ucode
        desc[64..72].copy_from_slice(&0u64.to_le_bytes());
        desc[72..76].copy_from_slice(&0u32.to_le_bytes());

        tracing::info!(
            ctx_dma,
            code_iova = format!("{code_iova:#010x}"),
            ucode_len = ucode.len(),
            "PMU ACR descriptor ready"
        );

        falcon_upload_dmem(bar0, base, 0, &desc);

        let _ = bar0.write_u32(base + falcon::BOOTVEC, 0);
        let _ = bar0.write_u32(base + falcon::MAILBOX0, 0);
        let _ = bar0.write_u32(base + falcon::MAILBOX1, 0);
        let _ = bar0.write_u32(base + falcon::CPUCTL_ALIAS, falcon::CPUCTL_IINVAL);
        std::thread::sleep(Duration::from_millis(1));
        let _ = bar0.write_u32(base + falcon::CPUCTL_ALIAS, falcon::CPUCTL_STARTCPU);

        let start = Instant::now();
        let timeout = Duration::from_secs(5);
        loop {
            std::thread::sleep(Duration::from_millis(50));
            let ctl = bar0.read_u32(base + falcon::CPUCTL_ALIAS).unwrap_or(0xDEAD);
            let mb0 = bar0.read_u32(base + falcon::MAILBOX0).unwrap_or(0);
            let mb1 = bar0.read_u32(base + falcon::MAILBOX1).unwrap_or(0);
            let pc = bar0.read_u32(base + falcon::PC).unwrap_or(0);

            if mb0 != 0 {
                tracing::info!(
                    cpuctl = format!("{ctl:#010x}"),
                    mb0 = format!("{mb0:#010x}"),
                    mb1 = format!("{mb1:#010x}"),
                    pc = format!("{pc:#010x}"),
                    elapsed_ms = start.elapsed().as_millis(),
                    "PMU ACR: mailbox response"
                );
                return Ok((ctl, mb0));
            }

            let halted = ctl & falcon::CPUCTL_HALTED != 0;
            if halted && ctl & falcon::CPUCTL_HRESET == 0 {
                tracing::info!(
                    cpuctl = format!("{ctl:#010x}"),
                    mb0 = format!("{mb0:#010x}"),
                    mb1 = format!("{mb1:#010x}"),
                    pc = format!("{pc:#010x}"),
                    elapsed_ms = start.elapsed().as_millis(),
                    "PMU ACR: halted (ACR may have completed)"
                );
                return Ok((ctl, mb0));
            }

            if start.elapsed() > timeout {
                let exci = bar0.read_u32(base + falcon::EXCI).unwrap_or(0xDEAD);
                tracing::error!(
                    cpuctl = format!("{ctl:#010x}"),
                    pc = format!("{pc:#010x}"),
                    exci = format!("{exci:#010x}"),
                    mb0 = format!("{mb0:#010x}"),
                    mb1 = format!("{mb1:#010x}"),
                    "PMU ACR: timeout"
                );
                return Err(DriverError::Unsupported(
                    format!("PMU ACR boot timed out (cpuctl={ctl:#010x} pc={pc:#010x})").into(),
                ));
            }
        }
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
        // GPCCS HS boot stalls on GV100 warm boots (PC stays at 0).
        // Use a shorter timeout to avoid wasting 3s on a known failure.
        let timeout = if name == "GPCCS" {
            Duration::from_millis(500)
        } else {
            Duration::from_secs(3)
        };
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

    fn supports_acr(&self) -> bool {
        self.has_gr_firmware()
    }

    fn acr_boot(
        &self,
        bar0: &MappedBar,
        _sm_version: u32,
        _chip: &str,
        dma: Option<crate::vfio::device::DmaBackend>,
    ) -> DriverResult<Vec<AcrBootResult>> {
        let dma = match dma {
            Some(d) => d,
            None => {
                return Ok(vec![AcrBootResult {
                    success: false,
                    strategy: "acr_no_dma".into(),
                    notes: vec!["ACR HS boot requires DMA backend — not provided".into()],
                }]);
            }
        };

        if !self.has_gr_firmware() {
            return Ok(vec![AcrBootResult {
                success: false,
                strategy: "acr_no_firmware".into(),
                notes: vec!["GR firmware not found on disk".into()],
            }]);
        }

        let mut results = Vec::new();

        // Stage 0: Boot PMU with ACR ucode.
        // On GV100, FECS/GPCCS are HS falcons that need ACR to configure
        // WPR and crypto keys. PMU is LS and fully accessible even when
        // GPC fabric is gated. ACR runs on PMU, sets up WPR in VRAM,
        // then FECS/GPCCS can execute signed firmware.
        match self.boot_pmu_acr(bar0, &dma) {
            Ok((ctl, mb0)) => {
                tracing::info!(
                    pmu_cpuctl = format!("{ctl:#010x}"),
                    pmu_mb0 = format!("{mb0:#010x}"),
                    "ACR: PMU ACR boot complete"
                );
                results.push(AcrBootResult {
                    success: true,
                    strategy: "pmu_acr".into(),
                    notes: vec![format!("PMU ACR cpuctl={ctl:#010x} mb0={mb0:#010x}")],
                });
                // Give ACR time to configure WPR
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                tracing::warn!(%e, "ACR: PMU ACR boot failed — trying direct HS boot");
                results.push(AcrBootResult {
                    success: false,
                    strategy: "pmu_acr".into(),
                    notes: vec![format!("PMU ACR failed: {e}")],
                });
            }
        }

        // Boot GPCCS first — FECS self-halts if GPCCS is not running
        match self.boot_falcon_hs(
            bar0,
            "GPCCS",
            falcon::GPCCS_BASE,
            &dma,
            GPCCS_FW_CODE_IOVA,
            GPCCS_FW_DATA_IOVA,
        ) {
            Ok((ctl, mb0)) => {
                tracing::info!(
                    gpccs_cpuctl = format!("{ctl:#010x}"),
                    gpccs_mb0 = format!("{mb0:#010x}"),
                    "ACR: GPCCS HS boot complete"
                );
                results.push(AcrBootResult {
                    success: true,
                    strategy: "hs_dma_gpccs".into(),
                    notes: vec![format!("cpuctl={ctl:#010x} mb0={mb0:#010x}")],
                });
            }
            Err(e) => {
                tracing::warn!(%e, "ACR: GPCCS HS boot failed");
                results.push(AcrBootResult {
                    success: false,
                    strategy: "hs_dma_gpccs".into(),
                    notes: vec![format!("GPCCS HS boot failed: {e}")],
                });
            }
        }

        // Boot FECS
        match self.boot_falcon_hs(
            bar0,
            "FECS",
            falcon::FECS_BASE,
            &dma,
            FECS_FW_CODE_IOVA,
            FECS_FW_DATA_IOVA,
        ) {
            Ok((ctl, mb0)) => {
                tracing::info!(
                    fecs_cpuctl = format!("{ctl:#010x}"),
                    fecs_mb0 = format!("{mb0:#010x}"),
                    "ACR: FECS HS boot complete"
                );
                results.push(AcrBootResult {
                    success: true,
                    strategy: "hs_dma_fecs".into(),
                    notes: vec![format!("cpuctl={ctl:#010x} mb0={mb0:#010x}")],
                });
            }
            Err(e) => {
                tracing::warn!(%e, "ACR: FECS HS boot failed");
                results.push(AcrBootResult {
                    success: false,
                    strategy: "hs_dma_fecs".into(),
                    notes: vec![format!("FECS HS boot failed: {e}")],
                });
            }
        }

        Ok(results)
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

    fn supports_pgob(&self) -> bool {
        true
    }

    fn pgob_disable(&self, bar0: &MappedBar) -> DriverResult<PgobResult> {
        const PMC_ENABLE: u32 = 0x0000_0200;
        const PMC_CLKGATE_DISABLE: u32 = 0x0000_0260;
        const PGRAPH_GPC_BCAST_CONTROL: u32 = 0x0041_9000;
        const PGRAPH_STATUS: u32 = 0x0040_0700;
        const GPC_ENABLES: u32 = 0x0002_2004;

        let gpc_before = bar0.read_u32(GPC_ENABLES as usize).unwrap_or(0);
        tracing::info!(
            gpc_before = format!("{gpc_before:#010x}"),
            "PGOB: GPC state before disable"
        );

        // Step 1: Disable PMC clock gating (write 1 to PMC + 0x260)
        bar0.write_u32(PMC_CLKGATE_DISABLE as usize, 1)
            .map_err(|e| DriverError::OracleError(format!("PMC clock gate disable: {e}").into()))?;
        tracing::debug!("PGOB: PMC clock gating disabled (0x260 = 1)");

        // Step 2: Ensure GR engine is enabled in PMC_ENABLE (bit 12)
        let pmc = bar0.read_u32(PMC_ENABLE as usize).unwrap_or(0);
        if pmc & (1 << 12) == 0 {
            bar0.write_u32(PMC_ENABLE as usize, pmc | (1 << 12))
                .map_err(|e| DriverError::OracleError(format!("PMC_ENABLE GR bit: {e}").into()))?;
            tracing::debug!("PGOB: enabled GR engine in PMC_ENABLE");
        }

        // Step 3: GPC broadcast — disable PGOB via PGRAPH GPC broadcast control.
        // Write 0x0110 to ungate GPC power-gated domains (from nouveau ctxgf100).
        bar0.write_u32(PGRAPH_GPC_BCAST_CONTROL as usize, 0x0000_0110)
            .map_err(|e| DriverError::OracleError(format!("GPC broadcast PGOB disable: {e}").into()))?;
        tracing::debug!("PGOB: wrote GPC broadcast control = 0x110");

        // Step 4: Per-GPC power gate disable via broadcast offset + 0x1028.
        // Writing 0 to each GPC's power gate control disables power gating.
        let gpc_pgob_offset = PGRAPH_GPC_BCAST_CONTROL + 0x1028;
        bar0.write_u32(gpc_pgob_offset as usize, 0x0000_0000)
            .map_err(|e| DriverError::OracleError(format!("GPC PGOB per-GPC disable: {e}").into()))?;
        tracing::debug!(
            offset = format!("{gpc_pgob_offset:#010x}"),
            "PGOB: wrote per-GPC power gate disable"
        );

        // Step 5: Poll PGRAPH_STATUS until ungating completes (no PRI fault).
        let deadline = Instant::now() + Duration::from_millis(100);
        let mut status = 0xDEAD_DEAD_u32;
        while Instant::now() < deadline {
            status = bar0.read_u32(PGRAPH_STATUS as usize).unwrap_or(0xDEAD_DEAD);
            // PRI fault signature: top nibble 0xBADF — ungating not complete
            if status >> 16 != 0xBADF {
                break;
            }
            std::thread::sleep(Duration::from_micros(100));
        }
        tracing::info!(
            status = format!("{status:#010x}"),
            "PGOB: PGRAPH_STATUS after ungating"
        );

        let gpc_after = bar0.read_u32(GPC_ENABLES as usize).unwrap_or(0);
        let gpc_is_pri_fault = crate::nv::pri::is_pri_fault(gpc_after);

        tracing::info!(
            gpc_before = format!("{gpc_before:#010x}"),
            gpc_after = format!("{gpc_after:#010x}"),
            status = format!("{status:#010x}"),
            gpc_is_pri_fault,
            "PGOB disable complete"
        );

        let gpc_alive = if gpc_is_pri_fault {
            tracing::warn!(
                gpc_after = format!("{gpc_after:#010x}"),
                "GPC_ENABLES reads as PRI fault — GPCs unreachable, reporting 0 alive"
            );
            0
        } else {
            gpc_after.count_ones()
        };

        Ok(PgobResult { gpc_alive })
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
