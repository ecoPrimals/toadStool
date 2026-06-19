// SPDX-License-Identifier: AGPL-3.0-or-later
//! GR engine init — falcon boot and FECS bring-up.

use std::time::{Duration, Instant};

use crate::error::SovereignStagesError;
use crate::vfio::device::MappedBar;

fn kepler_falcon_boot(
    bar0: &MappedBar,
    sm_version: u32,
    bridge: &dyn crate::nv::gsp_bridge::GspBridge,
) -> Result<String, SovereignStagesError> {
    use crate::nv::falcon_pio::{falcon_upload_dmem, falcon_upload_imem};
    use crate::vfio::channel::registers::falcon;

    let profile = crate::nv::generation::profile_for_sm(sm_version);

    let _ = bar0.write_u32(0x260, 1);
    std::thread::sleep(Duration::from_millis(10));

    // PGOB disable: ungate GPC compute domains before any GR register access.
    // Without this, GPC reads return 0xBADF3000 and FECS boot fails.
    if crate::nv::generation::is_kepler(profile) {
        tracing::info!("Kepler falcon boot: running PGOB disable before GR init");
        bridge.pgob_diagnostic(bar0, "sovereign_stages::pre-PGOB");
        match bridge.pgob_disable(bar0) {
            Ok(out) => tracing::info!(gpc_alive = out.gpc_alive, "sovereign PGOB succeeded"),
            Err(e) => tracing::warn!(%e, "sovereign PGOB failed — GPCs may be gated"),
        }
        bridge.pgob_diagnostic(bar0, "sovereign_stages::post-PGOB");
    }

    let _ = bridge.apply_gr_bar0_init(bar0, sm_version);

    let fw_dir = format!("/lib/firmware/nvidia/{}", profile.firmware_chip);
    let load = |name: &str| -> Result<Vec<u8>, SovereignStagesError> {
        let path = format!("{fw_dir}/{name}");
        std::fs::read(&path).map_err(|e| SovereignStagesError::KeplerFirmwareRead {
            path: path.clone(),
            source: e,
        })
    };

    let gpccs_inst = load("gpccs_inst.bin")?;
    let gpccs_data = load("gpccs_data.bin")?;
    let fecs_inst = load("fecs_inst.bin")?;
    let fecs_data = load("fecs_data.bin")?;

    tracing::info!(
        fecs_inst = fecs_inst.len(),
        fecs_data = fecs_data.len(),
        gpccs_inst = gpccs_inst.len(),
        gpccs_data = gpccs_data.len(),
        "Kepler falcon boot: firmware loaded from {fw_dir}"
    );

    let boot_falcon = |name: &'static str,
                       base: usize,
                       inst: &[u8],
                       data: &[u8]|
     -> Result<(u32, u32), SovereignStagesError> {
        let cpuctl = bar0.read_u32(base + falcon::CPUCTL).unwrap_or(0xDEAD);
        tracing::info!(
            name,
            cpuctl = format!("{cpuctl:#010x}"),
            "Kepler {name}: starting PIO upload"
        );

        let _ = bar0.write_u32(base + falcon::CPUCTL, falcon::CPUCTL_HRESET);
        std::thread::sleep(Duration::from_millis(5));

        falcon_upload_dmem(bar0, base, 0, data);
        falcon_upload_imem(bar0, base, 0, inst, false);

        let _ = bar0.write_u32(base + falcon::BOOTVEC, 0);
        let _ = bar0.write_u32(base + falcon::MAILBOX0, 0);
        let _ = bar0.write_u32(base + falcon::MAILBOX1, 0);
        let _ = bar0.write_u32(base + falcon::CPUCTL, falcon::CPUCTL_IINVAL);
        std::thread::sleep(Duration::from_millis(1));
        let _ = bar0.write_u32(base + falcon::CPUCTL, falcon::CPUCTL_STARTCPU);

        let start = Instant::now();
        let timeout = Duration::from_secs(2);
        loop {
            std::thread::sleep(Duration::from_millis(5));
            let ctl = bar0.read_u32(base + falcon::CPUCTL).unwrap_or(0xDEAD);
            let mb0 = bar0.read_u32(base + falcon::MAILBOX0).unwrap_or(0);

            if mb0 != 0 {
                tracing::info!(
                    name,
                    cpuctl = format!("{ctl:#010x}"),
                    mb0 = format!("{mb0:#010x}"),
                    "mailbox response"
                );
                return Ok((ctl, mb0));
            }
            if ctl & falcon::CPUCTL_HALTED != 0 && ctl & falcon::CPUCTL_HRESET == 0 {
                tracing::warn!(
                    name,
                    cpuctl = format!("{ctl:#010x}"),
                    "halted without mailbox"
                );
                return Ok((ctl, 0));
            }
            if start.elapsed() > timeout {
                tracing::error!(name, cpuctl = format!("{ctl:#010x}"), "timeout");
                return Err(SovereignStagesError::KeplerFalconBootTimeout { name, cpuctl: ctl });
            }
        }
    };

    let (gpccs_ctl, gpccs_mb0) =
        boot_falcon("GPCCS", falcon::GPCCS_BASE, &gpccs_inst, &gpccs_data)?;
    let (fecs_ctl, fecs_mb0) = boot_falcon("FECS", falcon::FECS_BASE, &fecs_inst, &fecs_data)?;

    let fecs_running = fecs_ctl & falcon::CPUCTL_HALTED == 0 && fecs_mb0 != 0;

    let detail = format!(
        "Kepler PIO: FECS cpuctl={fecs_ctl:#010x} mb0={fecs_mb0:#010x} | \
         GPCCS cpuctl={gpccs_ctl:#010x} mb0={gpccs_mb0:#010x} | running={fecs_running}"
    );

    if fecs_running {
        Ok(detail)
    } else {
        Err(SovereignStagesError::KeplerFalconNotRunning { detail })
    }
}

pub(crate) fn falcon_boot(
    bar0: &MappedBar,
    sm_version: u32,
    dma: Option<&crate::vfio::device::DmaBackend>,
    warm_state: crate::vfio::sovereign_strategy::FalconWarmState,
    bridge: &dyn crate::nv::gsp_bridge::GspBridge,
    boot_style: crate::vfio::sovereign_strategy::FalconBootStyle,
) -> Result<String, SovereignStagesError> {
    use crate::vfio::channel::registers::falcon;
    use crate::vfio::sovereign_strategy::{FalconBootStyle, FalconWarmState};

    match boot_style {
        FalconBootStyle::DirectPio => {
            tracing::info!(
                sm = sm_version,
                "DirectPio falcon boot — using PIO firmware upload (no ACR)"
            );
            return kepler_falcon_boot(bar0, sm_version, bridge);
        }
        FalconBootStyle::NoFalcons => {
            tracing::info!("No falcon engines on this hardware — skipping falcon boot");
            return Ok("no-falcons: hardware has no falcon microcontrollers".into());
        }
        FalconBootStyle::AcrDmaHs => {}
    }

    // ── FECS warm-preservation dispatch ──────────────────────────────
    //
    // The strategy has already classified the falcon thermal state via
    // `detect_falcon_warm_state()` — dispatch on the enum rather than
    // reading BAR0 registers inline.
    tracing::info!(warm_state = ?warm_state, "falcon warm-state detection result");

    match warm_state {
        FalconWarmState::WarmPreserved { cpuctl, mailbox0 } => {
            tracing::info!(
                "FECS warm-preserved (HALTED + mb0={mailbox0:#010x}) — skipping ACR/PIO"
            );
            return Ok(format!(
                "warm-preserved: FECS cpuctl={cpuctl:#010x} mb0={mailbox0:#010x}"
            ));
        }
        FalconWarmState::WarmRunning {
            cpuctl,
            pc,
            mailbox0,
        } => {
            tracing::info!(
                fecs_pc = format!("{pc:#010x}"),
                "FECS warm-running (active firmware, PC advancing) — skipping boot"
            );
            return Ok(format!(
                "warm-running: FECS cpuctl={cpuctl:#010x} pc={pc:#010x} mb0={mailbox0:#010x}"
            ));
        }
        FalconWarmState::Inconsistent { cpuctl } => {
            tracing::warn!(
                cpuctl = format!("{cpuctl:#010x}"),
                "FECS inconsistent teardown state — attempting PIO re-bootstrap"
            );
            let chip = crate::nv::identity::chip_name(sm_version);
            match bridge.boot_gr_falcons(bar0, chip) {
                Ok(result) if result.running => {
                    return Ok(format!(
                        "warm re-bootstrap OK: FECS cpuctl=0x{:08x} mb0=0x{:08x}",
                        result.cpuctl_after, result.mailbox0,
                    ));
                }
                Ok(result) => {
                    tracing::warn!(
                        cpuctl = format!("0x{:08x}", result.cpuctl_after),
                        "PIO re-bootstrap: FECS not running — falling through to cold path"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "PIO re-bootstrap failed — falling through to cold path"
                    );
                }
            }
        }
        FalconWarmState::Cold => {}
    }

    let chip = crate::nv::identity::chip_name(sm_version);

    let _ = bridge.apply_gr_bar0_init(bar0, sm_version);

    // Exp 173 proved nvidia-535 closed does NOT configure WPR on GV100 (pre-GSP).
    // WPR is a Turing+/Ampere+ concept for GSP-RM protection. On Volta, the RM
    // runs on the CPU and doesn't use WPR hardware boundaries. The ACR chain's
    // requirement for WPR cannot be satisfied on GV100 through vendor drivers.
    // The SEC2→HS→FECS bootstrap path requires a different approach for Volta.

    let wpr1_beg = bar0.read_u32(0x100CE4).unwrap_or(0xDEAD);
    let wpr1_end = bar0.read_u32(0x100CE8).unwrap_or(0xDEAD);
    let wpr2_beg = bar0.read_u32(0x100CEC).unwrap_or(0xDEAD);
    let wpr2_end = bar0.read_u32(0x100CF0).unwrap_or(0xDEAD);
    let wpr_configured = wpr2_beg != 0 && wpr2_end != 0 && wpr2_end > wpr2_beg;
    tracing::info!(
        wpr1_beg = format!("{wpr1_beg:#x}"),
        wpr1_end = format!("{wpr1_end:#x}"),
        wpr2_beg = format!("{wpr2_beg:#x}"),
        wpr2_end = format!("{wpr2_end:#x}"),
        wpr_configured,
        "pre-ACR WPR state"
    );

    tracing::info!(
        chip,
        dma_available = dma.is_some(),
        "falcon boot: trying ACR boot solver..."
    );

    let acr_detail = match bridge.acr_boot(bar0, sm_version, chip, dma.cloned()) {
        Ok(results) => {
            if results.iter().any(|r| r.success) {
                let cpuctl = bar0
                    .read_u32(falcon::FECS_BASE + falcon::CPUCTL)
                    .unwrap_or(0xDEAD_DEAD);
                let mb0 = bar0
                    .read_u32(falcon::FECS_BASE + falcon::MAILBOX0)
                    .unwrap_or(0);
                return Ok(format!(
                    "ACR boot OK: FECS cpuctl=0x{cpuctl:08x} mb0=0x{mb0:08x} ({} strategies)",
                    results.len()
                ));
            }
            let summary: Vec<String> = results
                .iter()
                .enumerate()
                .map(|(i, r)| {
                    let tail: Vec<&str> =
                        r.notes.iter().rev().take(40).map(|s| s.as_str()).collect();
                    format!("S{i}:{} [{}]", r.strategy, tail.join("; "))
                })
                .collect();
            summary.join(" | ")
        }
        Err(e) => format!("solver_err:{e}"),
    };

    tracing::info!(chip, "ACR failed, trying direct PIO upload...");
    match bridge.boot_gr_falcons(bar0, chip) {
        Ok(result) => {
            let detail = format!(
                "direct boot: FECS cpuctl=0x{:08x} mb0=0x{:08x} running={} | acr:[{}]",
                result.cpuctl_after, result.mailbox0, result.running, acr_detail,
            );
            if result.running {
                Ok(detail)
            } else {
                Err(SovereignStagesError::FalconBootNotRunning { detail })
            }
        }
        Err(e) => Err(SovereignStagesError::FalconBootPathsExhausted {
            detail: format!("{e} | acr:[{acr_detail}]"),
        }),
    }
}

pub(crate) fn gr_init(
    bar0: &MappedBar,
    sm_version: u32,
    bridge: &dyn crate::nv::gsp_bridge::GspBridge,
) -> Result<String, SovereignStagesError> {
    let chip = crate::nv::identity::chip_name(sm_version);

    match bridge.boot_fecs(bar0, chip) {
        Ok(result) if result.running => Ok(format!(
            "GR ready: FECS mb0=0x{:08x} mb1=0x{:08x}",
            result.mailbox0, result.mailbox1,
        )),
        Ok(result) => Err(SovereignStagesError::GrFecsNotRunning {
            cpuctl: result.cpuctl_after,
        }),
        Err(e) => Err(SovereignStagesError::VfioCompute(Box::new(e))),
    }
}
