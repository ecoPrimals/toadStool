// SPDX-License-Identifier: AGPL-3.0-or-later
//! HBM2/GDDR5 memory training and warm-state detection.

use crate::error::SovereignStagesError;
use crate::vfio::channel::hbm2_training::{self, Hbm2Controller};
use crate::vfio::device::MappedBar;
use crate::vfio::sovereign_types::SovereignInitOptions;

pub(crate) fn run_hbm2_training(
    bar0: &MappedBar,
    bdf: &str,
    fbpa_count: usize,
    opts: &SovereignInitOptions,
) -> Result<crate::vfio::channel::hbm2_training::TrainingLog, SovereignStagesError> {
    let mut ctrl = Hbm2Controller::new(bar0, Some(bdf), fbpa_count);

    if let Some(golden) = &opts.golden_state {
        ctrl = ctrl.with_backend(hbm2_training::TrainingBackend::DifferentialReplay {
            golden_state: golden.clone(),
        });
    } else if let Some(rom) = &opts.vbios_rom {
        ctrl = ctrl
            .with_backend(hbm2_training::TrainingBackend::VbiosInterpreter { rom: rom.clone() });
    }

    let phy = ctrl.enable_phy()?;
    let linked = phy.train_links()?;
    let dram = linked.init_dram()?;

    match dram.verify_vram() {
        Ok(verified) => {
            let log = verified.training_log().clone();
            tracing::info!(
                writes = log.write_count(),
                "HBM2 training complete — VRAM verified"
            );
            Ok(log)
        }
        Err(e) => Err(e.into()),
    }
}

/// GDDR5 memory training for Kepler GPUs.
///
/// Cold K80s return `0xbad0fb0*` from PRAMIN because GDDR5 hasn't been
/// trained after PCI reset.  This function reads the VBIOS, runs the
/// DEVINIT script interpreter (via PMU falcon or host-side fallback),
/// and verifies PRAMIN returns valid data afterward.
///
/// # Why the devinit status register is consulted before PRAMIN
///
/// [`pramin_sentinel_test`] is not a read. It steers `BAR0_WINDOW` and then
/// **writes** `0xCAFEBEEF` into VRAM through the PRAMIN aperture. On a card
/// whose GDDR5 has never been trained, that is a store into memory with no
/// configured timings, through a window register that on a cold K80 lives in
/// a PRI-faulted PBUS ring (`0xbad0011f`).
///
/// This function used to run that probe *first*, to decide whether training
/// was needed — the same circularity as resetting an unresponsive device to
/// make it respond. You cannot probe VRAM to learn whether VRAM works.
///
/// Observed 2026-08-16 on a Tesla K80: the die answered through
/// `boot_state_probe` and was all-ones by the time devinit was consulted a
/// few instructions later. Only a reboot recovered it.
///
/// `DEVINIT_STATUS` is the right question and is answerable on a cold card —
/// measured `0x00000000` on both K80 dies, `0x00000002` on a POSTed Titan V.
/// So: ask the register, run devinit if it says POST is needed, and probe
/// PRAMIN only once the memory it addresses has been brought up.
pub(crate) fn gddr5_training(bar0: &MappedBar, bdf: &str) -> Result<String, SovereignStagesError> {
    use crate::vfio::channel::devinit;
    use crate::vfio::channel::devinit::DevinitStatus;

    let status = DevinitStatus::probe(bar0);

    if !status.readable {
        // Nothing below is answerable, and a PRAMIN write would be a store
        // into a device that is not there.
        return Err(SovereignStagesError::Devinit(
            crate::error::DevinitError::StatusUnreadable {
                devinit_reg: status.devinit_reg,
            },
        ));
    }

    if !status.needs_post {
        // POST is done, so VRAM is trained and the aperture is safe to touch.
        if pramin_sentinel_test(bar0) {
            return Ok("GDDR5 already trained (POST complete, PRAMIN sentinel OK)".into());
        }
        tracing::warn!(
            devinit_reg = format!("{:#010x}", status.devinit_reg),
            "devinit reports POST complete but PRAMIN is dead — re-running devinit"
        );
    } else {
        tracing::info!(
            devinit_reg = format!("{:#010x}", status.devinit_reg),
            "GDDR5 cold per devinit status — skipping PRAMIN probe until memory is up"
        );
    }

    tracing::info!("GDDR5 cold detected — running DEVINIT for memory training");

    match devinit::execute_devinit_with_diagnostics(bar0, Some(bdf)) {
        Ok(true) => {
            if pramin_sentinel_test(bar0) {
                Ok("GDDR5 trained via DEVINIT — PRAMIN verified".into())
            } else {
                Err(SovereignStagesError::Gddr5PraminDeadAfterDevinit)
            }
        }
        Ok(false) => {
            // devinit declined. If we read "POST needed" on entry, the two
            // readings disagree and we do not know the state of memory —
            // so we must not write into it to find out. Probing PRAMIN here
            // is what wedged K80 die 2 on 2026-08-16, after the entry guard
            // above had correctly kept us away from it moments earlier.
            if status.needs_post {
                return Err(SovereignStagesError::Gddr5DevinitDeclinedWhilePostNeeded {
                    devinit_reg: status.devinit_reg,
                });
            }
            if pramin_sentinel_test(bar0) {
                Ok("DEVINIT reports already done — PRAMIN verified".into())
            } else {
                Err(SovereignStagesError::Gddr5PraminDeadDevinitSkipped)
            }
        }
        Err(e) => Err(e.into()),
    }
}

/// Strategy for memory training, keyed by `MemoryType`.
///
/// Each GPU generation's `GenerationProfile::memory_type` maps to a
/// training strategy. The dispatch function runs the appropriate path
/// or returns a skip reason for types that don't need explicit training.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryTrainingStrategy {
    /// GDDR5 (Kepler): DEVINIT script interpreter.
    Gddr5Devinit,
    /// HBM2/HBM2e (Volta, Ampere datacenter): typestate training controller.
    Hbm2Controller,
    /// Memory type exists but no sovereign training path implemented yet.
    Unsupported(&'static str),
}

impl MemoryTrainingStrategy {
    /// Determine the training strategy for a given memory type.
    #[must_use]
    pub fn for_memory_type(mem: crate::hardware::MemoryType) -> Self {
        use crate::hardware::MemoryType;
        match mem {
            MemoryType::Gddr5 => Self::Gddr5Devinit,
            MemoryType::Hbm2 => Self::Hbm2Controller,
            MemoryType::Hbm3 => Self::Unsupported("HBM3"),
            MemoryType::Gddr6 => Self::Unsupported("GDDR6"),
            MemoryType::Gddr6x => Self::Unsupported("GDDR6X"),
            MemoryType::Gddr7 => Self::Unsupported("GDDR7"),
        }
    }
}

/// Result of memory training dispatch.
pub enum MemoryTrainingResult {
    /// Training ran or was already warm — detail string.
    Ok(String),
    /// HBM2 training produced a full training log.
    OkWithLog(crate::vfio::channel::hbm2_training::TrainingLog),
    /// Skipped (already warm or unsupported type) — reason string.
    Skipped(String),
    /// Training failed.
    Failed(SovereignStagesError),
}

/// Dispatch memory training based on strategy and warm state.
///
/// Centralizes the branching logic that was previously inlined in
/// `sovereign_init`. Callers provide the strategy (from
/// `MemoryTrainingStrategy::for_memory_type`) and warm detection flag.
pub(crate) fn dispatch_memory_training(
    strategy: MemoryTrainingStrategy,
    bar0: &MappedBar,
    bdf: &str,
    warm_detected: bool,
    pmc_before: u32,
    opts: &SovereignInitOptions,
) -> MemoryTrainingResult {
    match strategy {
        MemoryTrainingStrategy::Gddr5Devinit => {
            if warm_detected {
                tracing::info!(
                    pmc_enable = format!("0x{pmc_before:08x}"),
                    "GDDR5 GPU warm — skipping memory training"
                );
                return MemoryTrainingResult::Skipped(format!(
                    "GDDR5 warm (pmc=0x{pmc_before:08x}, PRAMIN sentinel ok)"
                ));
            }
            match gddr5_training(bar0, bdf) {
                Ok(detail) => MemoryTrainingResult::Ok(detail),
                Err(e) => MemoryTrainingResult::Failed(e),
            }
        }
        MemoryTrainingStrategy::Hbm2Controller => {
            if warm_detected {
                tracing::info!(
                    pmc_enable = format!("0x{pmc_before:08x}"),
                    "warm GPU detected — skipping HBM2 training"
                );
                return MemoryTrainingResult::Skipped(format!(
                    "warm detected (pmc=0x{pmc_before:08x}, PRAMIN sentinel ok)"
                ));
            }
            // After early falcon boot, the PMU may have completed HBM2 training
            // autonomously.  Check PRAMIN before attempting the full controller path.
            if pramin_sentinel_test(bar0) {
                tracing::info!(
                    pmc_enable = format!("0x{pmc_before:08x}"),
                    "VRAM alive (PMU devinit completed) — skipping HBM2 training"
                );
                return MemoryTrainingResult::Skipped(format!(
                    "VRAM alive after falcon boot (pmc=0x{pmc_before:08x})"
                ));
            }

            // On cold secure-boot GPUs, try PMU FALCON devinit first.
            // The PMU firmware in the VBIOS ROM includes HBM2 training
            // sequences that the host-side interpreter cannot replicate.
            tracing::info!("HBM2 cold: trying PMU FALCON devinit before controller path");
            match crate::vfio::channel::devinit::execute_devinit_with_diagnostics(bar0, Some(bdf)) {
                Ok(true) => {
                    tracing::info!("PMU FALCON devinit trained HBM2 — VRAM alive");
                    return MemoryTrainingResult::Ok("HBM2 trained via PMU FALCON devinit".into());
                }
                Ok(false) => {
                    tracing::info!("PMU FALCON devinit: not needed or VRAM still dead");
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "PMU FALCON devinit failed — falling through to HBM2 controller"
                    );
                }
            }

            // Check PRAMIN again in case PMU devinit worked asynchronously
            if pramin_sentinel_test(bar0) {
                tracing::info!("VRAM alive after PMU devinit attempt");
                return MemoryTrainingResult::Ok(
                    "HBM2 trained via PMU FALCON devinit (delayed)".into(),
                );
            }

            let fbpa_count = opts.fbpa_count.unwrap_or(4);
            match run_hbm2_training(bar0, bdf, fbpa_count, opts) {
                Ok(log) => MemoryTrainingResult::OkWithLog(log),
                Err(e) => MemoryTrainingResult::Failed(e),
            }
        }
        MemoryTrainingStrategy::Unsupported(name) => {
            MemoryTrainingResult::Skipped(format!("memory_type={name} (pmc=0x{pmc_before:08x})"))
        }
    }
}

/// # This is a destructive probe, despite the name
///
/// It steers `BAR0_WINDOW` and **writes** `0xCAFEBEEF` into VRAM, then reads
/// it back. It is not safe to call on a device whose memory controller has
/// not been brought up: on a cold Kepler that is a store into untrained
/// GDDR5 through a window register in a PRI-faulted PBUS ring, and it wedges
/// the die until the next reboot. Both K80 dies were lost this way on
/// 2026-08-16.
///
/// Establish that memory is up — `DEVINIT_STATUS` bit 1, or a successful
/// devinit — *before* calling this. Never call it to decide whether memory
/// needs training; that is the question it cannot survive asking.
pub(crate) fn pramin_sentinel_test(bar0: &MappedBar) -> bool {
    use crate::vfio::memory::{MemoryRegion, PraminRegion};

    match PraminRegion::new(bar0, 0x0002_6000, 8) {
        Ok(mut region) => region.probe_sentinel(0, 0xCAFE_BEEF).is_working(),
        Err(_) => false,
    }
}

/// Map chip_id → SM version.
///
/// Delegates to the authoritative [`boot0_to_sm`](crate::nv::identity::boot0_to_sm)
/// by reconstructing a synthetic BOOT0 from the chip_id. Falls back to Volta
/// (SM 70) for unrecognized chipsets with a warning.
pub(crate) fn chip_id_to_sm(chip_id: u32) -> u32 {
    let synthetic_boot0 = chip_id << 20;
    match crate::nv::identity::boot0_to_sm(synthetic_boot0) {
        Some(sm) => sm,
        None => {
            tracing::warn!(
                chip_id = format!("0x{chip_id:03x}"),
                "unknown chip — defaulting to SM 70 (Volta)"
            );
            70
        }
    }
}

/// Heuristic to detect if the GPU has already been trained.
///
/// A "warm" GPU has most engines enabled (high popcount in PMC_ENABLE)
/// and accessible VRAM (PRAMIN sentinel test passes). A cold GPU after
/// PCI reset typically has PMC_ENABLE = 0x0 or very few bits set.
pub(crate) fn is_warm_gpu(pmc_enable: u32, bar0: &MappedBar) -> bool {
    let popcount = pmc_enable.count_ones();
    if popcount < 8 {
        return false;
    }
    pramin_sentinel_test(bar0)
}

#[cfg(test)]
mod gddr5_ordering_tests {
    use crate::nv::register_read::RegisterRead;

    /// Mirror of the ordering rule in `gddr5_training`: may the PRAMIN
    /// aperture be touched, given only the devinit status register?
    fn pramin_probe_allowed(devinit_reg: u32) -> bool {
        let read = RegisterRead::classify(devinit_reg);
        match read.valid() {
            None => false,           // unreadable: the device is not there
            Some(v) => (v & 2) != 0, // POST complete: VRAM is up
        }
    }

    /// The K80 wedge. A cold die reads 0x0 — POST needed, memory untrained.
    /// Writing 0xCAFEBEEF through PRAMIN here is what killed both dies.
    #[test]
    fn cold_kepler_must_not_probe_pramin() {
        assert!(
            !pramin_probe_allowed(0x0000_0000),
            "untrained GDDR5 must not be written to in order to test it"
        );
    }

    /// A POSTed Titan V reads 0x2 — the aperture is backed by live memory.
    #[test]
    fn posted_device_may_probe_pramin() {
        assert!(pramin_probe_allowed(0x0000_0002));
    }

    /// An unresponsive device is the worst case: a store into nothing.
    #[test]
    fn unreadable_status_must_not_probe_pramin() {
        assert!(!pramin_probe_allowed(0xFFFF_FFFF));
        assert!(!pramin_probe_allowed(0xBADF_5040));
    }
}
