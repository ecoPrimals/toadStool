// SPDX-License-Identifier: AGPL-3.0-or-later
//! Kepler (GK110/GK210) `InitPipeline` + `BootPipeline` implementation.
//!
//! Encodes the Tesla K80 initialization path:
//! - VBIOS interpreter for GDDR5 devinit (no PMU firmware table in BIT I)
//! - Direct PIO falcon upload (unsigned firmware, no ACR/HS)
//! - GrInitSequence replay for PGRAPH ungating
//!
//! Implements both the NVIDIA-specific `InitPipeline` (takes `&MappedBar`) and
//! the vendor-agnostic `BootPipeline` (takes `&dyn RegisterAccess`).

use crate::error::DriverError;
use crate::hardware::{BootInitInfo, BootPipeline, BootProbeInfo, Vendor};
use crate::nv::gr_init::ChipFamily;
use crate::vfio::device::{MappedBar, RegisterAccess};
use crate::vfio::init_pipeline::*;
use crate::vfio::sovereign_stages;

const NV_PMC_ENABLE: u32 = 0x0000_0200;

/// Kepler-family init pipeline.
///
/// Handles GK110/GK210 GPUs (Tesla K80, K40). These use unsigned
/// firmware, host-side VBIOS interpretation for GDDR5 training, and
/// direct PIO falcon upload for FECS/GPCCS.
#[derive(Debug)]
pub struct KeplerInit {
    bdf: Option<String>,
}

impl Default for KeplerInit {
    fn default() -> Self {
        Self::new()
    }
}

impl KeplerInit {
    /// Create a Kepler pipeline with no BDF (uses default/unknown).
    pub fn new() -> Self {
        Self { bdf: None }
    }

    /// Create a Kepler pipeline targeting a specific PCI BDF address.
    pub fn with_bdf(bdf: impl Into<String>) -> Self {
        Self {
            bdf: Some(bdf.into()),
        }
    }
}

impl InitPipeline for KeplerInit {
    fn chip_family(&self) -> ChipFamily {
        ChipFamily::Kepler
    }

    fn probe(&self, bar0: &MappedBar) -> Result<ProbeResult, DriverError> {
        let (boot0, chip_id) = sovereign_stages::bar0_probe(bar0)
            .map_err(|e| DriverError::Unsupported(format!("bar0_probe: {e}").into()))?;

        let sm = sovereign_stages::chip_id_to_sm(chip_id);
        let pmc = bar0
            .read_u32(sovereign_stages::PMC_ENABLE)
            .unwrap_or(0);
        let warm = sovereign_stages::is_warm_gpu(pmc, bar0);

        Ok(ProbeResult {
            boot0,
            chip_id,
            sm_version: sm,
            warm,
            pmc_enable: pmc,
        })
    }

    fn devinit(
        &self,
        bar0: &MappedBar,
        probe: &ProbeResult,
    ) -> Result<DevinitResult, DriverError> {
        if probe.warm {
            return Ok(DevinitResult {
                vram_alive: true,
                method: DevinitMethod::WarmSkip,
                writes_applied: 0,
            });
        }

        let bdf = self.bdf.as_deref().unwrap_or("unknown");
        match sovereign_stages::gddr5_training(bar0, bdf) {
            Ok(detail) => {
                tracing::info!(detail, "Kepler GDDR5 devinit complete");
                Ok(DevinitResult {
                    vram_alive: true,
                    method: DevinitMethod::VbiosInterpreter,
                    writes_applied: 0,
                })
            }
            Err(e) => Err(DriverError::Unsupported(
                format!("GDDR5 devinit failed: {e}").into(),
            )),
        }
    }

    fn engine_init(
        &self,
        bar0: &MappedBar,
        probe: &ProbeResult,
    ) -> Result<EngineResult, DriverError> {
        use crate::nv::gsp_bridge::NoopGspBridge;

        let bridge = NoopGspBridge::default();

        match sovereign_stages::falcon_boot(
            bar0,
            probe.sm_version,
            None,
            crate::vfio::sovereign_strategy::FalconWarmState::Cold,
            &bridge,
            crate::vfio::sovereign_strategy::FalconBootStyle::DirectPio,
        ) {
            Ok(detail) => {
                let preserved = detail.contains("warm-preserved");
                Ok(EngineResult {
                    fecs_running: !preserved,
                    fecs_cpuctl: 0,
                    fecs_mailbox0: 0,
                    method: if preserved {
                        EngineInitMethod::WarmPreserved
                    } else {
                        EngineInitMethod::PioUpload
                    },
                })
            }
            Err(e) => {
                if probe.warm {
                    Ok(EngineResult {
                        fecs_running: false,
                        fecs_cpuctl: 0,
                        fecs_mailbox0: 0,
                        method: EngineInitMethod::WarmGated,
                    })
                } else {
                    Err(DriverError::Unsupported(
                        format!("falcon boot: {e}").into(),
                    ))
                }
            }
        }
    }

    fn verify(&self, bar0: &MappedBar) -> Result<VerifyResult, DriverError> {
        match sovereign_stages::verify(bar0) {
            Ok(detail) => Ok(VerifyResult {
                ptimer_alive: true,
                vram_ok: true,
                pmc_enable: bar0
                    .read_u32(sovereign_stages::PMC_ENABLE)
                    .unwrap_or(0),
                detail,
            }),
            Err(e) => Err(DriverError::Unsupported(
                format!("verify: {e}").into(),
            )),
        }
    }
}

// ── Vendor-Agnostic BootPipeline ──────────────────────────────────────

impl BootPipeline for KeplerInit {
    type ProbeResult = ProbeResult;
    type InitResult = DevinitResult;

    fn device_family(&self) -> &'static str {
        "Kepler"
    }

    fn probe(
        &self,
        bar: &dyn RegisterAccess,
    ) -> Result<ProbeResult, DriverError> {
        let boot0 = bar
            .read_u32(0x0000_0000)
            .map_err(|e| DriverError::Unsupported(format!("BOOT0 read: {e}").into()))?;
        let chip_id = (boot0 >> 20) & 0x1FF;
        let sm = sovereign_stages::chip_id_to_sm(chip_id);
        let pmc = bar.read_u32(NV_PMC_ENABLE).unwrap_or(0);

        // Match is_warm_gpu() threshold (>= 8) and add PRAMIN window
        // read as lightweight VRAM accessibility check.
        let pmc_warm = pmc.count_ones() >= 8;
        let pramin_accessible = bar.read_u32(0x0070_0000).unwrap_or(0) != 0;
        let warm = pmc_warm && pramin_accessible;

        Ok(ProbeResult {
            boot0,
            chip_id,
            sm_version: sm,
            warm,
            pmc_enable: pmc,
        })
    }

    fn is_warm(&self, probe: &ProbeResult) -> bool {
        probe.warm
    }

    fn probe_summary(&self, probe: &ProbeResult) -> BootProbeInfo {
        BootProbeInfo {
            vendor: Vendor::Nvidia,
            family: "Kepler".to_string(),
            warm: probe.warm,
            identity_raw: probe.boot0,
        }
    }

    fn devinit(
        &self,
        _bar: &dyn RegisterAccess,
        probe: &ProbeResult,
    ) -> Result<DevinitResult, DriverError> {
        if probe.warm {
            return Ok(DevinitResult {
                vram_alive: true,
                method: DevinitMethod::WarmSkip,
                writes_applied: 0,
            });
        }
        Err(DriverError::Unsupported(
            "Kepler cold devinit requires MappedBar (use InitPipeline)".into(),
        ))
    }

    fn init_summary(&self, init: &DevinitResult) -> BootInitInfo {
        BootInitInfo {
            memory_alive: init.vram_alive,
            writes_applied: init.writes_applied,
            method: format!("{:?}", init.method),
        }
    }

    fn engine_init(
        &self,
        _bar: &dyn RegisterAccess,
        probe: &ProbeResult,
    ) -> Result<(), DriverError> {
        if probe.warm {
            return Ok(());
        }
        Err(DriverError::Unsupported(
            "Kepler engine_init requires MappedBar (use InitPipeline)".into(),
        ))
    }

    fn verify(
        &self,
        bar: &dyn RegisterAccess,
    ) -> Result<bool, DriverError> {
        let ptimer_lo = bar.read_u32(0x0000_9400).unwrap_or(0);
        let ptimer_hi = bar.read_u32(0x0000_9410).unwrap_or(0);
        let pmc = bar.read_u32(NV_PMC_ENABLE).unwrap_or(0);
        let ptimer_alive = (ptimer_lo | ptimer_hi) != 0;
        Ok(ptimer_alive && pmc.count_ones() >= 8)
    }
}
