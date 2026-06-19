// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::{Duration, Instant};

use crate::error::{DriverError, DriverResult};
use crate::nv::falcon_pio::{falcon_upload_dmem, falcon_upload_imem};
use crate::vfio::channel::registers::falcon;
use crate::vfio::device::MappedBar;

use super::{ACR_UCODE_IOVA, NvGspBridge};

impl NvGspBridge {
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
        let mut code_buf = crate::vfio::dma::DmaBuffer::new(dma.clone(), code_pages, code_iova)?;
        code_buf.as_mut_slice()[..ucode.len()].copy_from_slice(&ucode);

        // Hold PMU in HRESET
        let _ = bar0.write_u32(base + falcon::CPUCTL_ALIAS, falcon::CPUCTL_HRESET);
        std::thread::sleep(Duration::from_millis(10));

        // Enable DMA engine
        let _ = bar0.write_u32(base + falcon::DMACTL, 0x01);

        // Configure PMU FBIF (at base + 0xE00, not 0x600)
        let fbif_base = base + 0xE00;
        let _ = bar0.write_u32(
            fbif_base + 4 * falcon::FBIF_DMAIDX_UCODE,
            falcon::FBIF_TARGET_PHYS_SYS_COH,
        );
        let _ = bar0.write_u32(
            fbif_base + 4 * falcon::FBIF_DMAIDX_VIRT,
            falcon::FBIF_TARGET_VIRT,
        );
        let _ = bar0.write_u32(
            fbif_base + 4 * falcon::FBIF_DMAIDX_PHYS_VID,
            falcon::FBIF_TARGET_PHYS_VID,
        );
        let _ = bar0.write_u32(
            fbif_base + 4 * falcon::FBIF_DMAIDX_PHYS_SYS_COH,
            falcon::FBIF_TARGET_PHYS_SYS_COH,
        );
        let _ = bar0.write_u32(
            fbif_base + 4 * falcon::FBIF_DMAIDX_PHYS_SYS_NCOH,
            falcon::FBIF_TARGET_PHYS_SYS_NCOH,
        );

        let fbif0 = bar0
            .read_u32(fbif_base + 4 * falcon::FBIF_DMAIDX_UCODE)
            .unwrap_or(0xDEAD);
        let fbif3 = bar0
            .read_u32(fbif_base + 4 * falcon::FBIF_DMAIDX_PHYS_SYS_COH)
            .unwrap_or(0xDEAD);
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

    pub(super) fn boot_falcon_pio(
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
                tracing::error!(name, cpuctl = format!("{ctl:#010x}"), "PIO boot: timeout");
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
        let bl = self.load_gr_blob(if name == "FECS" {
            "fecs_bl.bin"
        } else {
            "gpccs_bl.bin"
        })?;
        let inst = self.load_gr_blob(if name == "FECS" {
            "fecs_inst.bin"
        } else {
            "gpccs_inst.bin"
        })?;
        let data = self.load_gr_blob(if name == "FECS" {
            "fecs_data.bin"
        } else {
            "gpccs_data.bin"
        })?;
        let sig = self
            .load_gr_blob(if name == "FECS" {
                "fecs_sig.bin"
            } else {
                "gpccs_sig.bin"
            })
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

        let mut code_buf = crate::vfio::dma::DmaBuffer::new(dma.clone(), code_pages, code_iova)?;
        code_buf.as_mut_slice()[..inst.len()].copy_from_slice(&inst);

        let mut data_buf = crate::vfio::dma::DmaBuffer::new(dma.clone(), data_pages, data_iova)?;
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
        let _ = bar0.write_u32(
            fbif_base + 4 * falcon::FBIF_DMAIDX_UCODE,
            falcon::FBIF_TARGET_PHYS_VID,
        );
        let _ = bar0.write_u32(
            fbif_base + 4 * falcon::FBIF_DMAIDX_VIRT,
            falcon::FBIF_TARGET_VIRT,
        );
        let _ = bar0.write_u32(
            fbif_base + 4 * falcon::FBIF_DMAIDX_PHYS_VID,
            falcon::FBIF_TARGET_PHYS_VID,
        );
        let _ = bar0.write_u32(
            fbif_base + 4 * falcon::FBIF_DMAIDX_PHYS_SYS_COH,
            falcon::FBIF_TARGET_PHYS_SYS_COH,
        );
        let _ = bar0.write_u32(
            fbif_base + 4 * falcon::FBIF_DMAIDX_PHYS_SYS_NCOH,
            falcon::FBIF_TARGET_PHYS_SYS_NCOH,
        );

        // Read back FBIF to verify
        let fbif0 = bar0
            .read_u32(fbif_base + 4 * falcon::FBIF_DMAIDX_UCODE)
            .unwrap_or(0xDEAD);
        let fbif3 = bar0
            .read_u32(fbif_base + 4 * falcon::FBIF_DMAIDX_PHYS_SYS_COH)
            .unwrap_or(0xDEAD);
        let fbif4 = bar0
            .read_u32(fbif_base + 4 * falcon::FBIF_DMAIDX_PHYS_SYS_NCOH)
            .unwrap_or(0xDEAD);
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
