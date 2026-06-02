// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::{Duration, Instant};

use crate::error::{DriverError, DriverResult};
use crate::nv::gsp_bridge::{AcrBootResult, FalconBootResult, GspBridge, PgobResult};
use crate::vfio::channel::registers::falcon;
use crate::vfio::device::MappedBar;

use super::{
    FECS_FW_CODE_IOVA, FECS_FW_DATA_IOVA, GPCCS_FW_CODE_IOVA, GPCCS_FW_DATA_IOVA, NvGspBridge,
};

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
