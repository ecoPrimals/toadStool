// SPDX-License-Identifier: AGPL-3.0-or-later
//! Device verification stage for sovereign init.

use crate::error::SovereignStagesError;
use crate::vfio::device::MappedBar;

use super::memory::pramin_sentinel_test;
use super::pmc::{ISOLATE_TIMEOUT, PMC_ENABLE, PTIMER_TIME_0, PTIMER_TIME_1};

pub(crate) fn verify(bar0: &MappedBar) -> Result<String, SovereignStagesError> {
    // PTIMER liveness: both low and high timer registers should be non-zero
    // on a running GPU.
    let ops = vec![
        (PTIMER_TIME_0 as u32, None),
        (PTIMER_TIME_1 as u32, None),
        (PMC_ENABLE as u32, None),
    ];

    let result = bar0.isolated_batch(&ops, ISOLATE_TIMEOUT);
    match result {
        crate::vfio::isolation::IsolationResult::Ok(vals) => {
            let timer_lo = vals.first().copied().unwrap_or(0);
            let timer_hi = vals.get(1).copied().unwrap_or(0);
            let pmc = vals.get(2).copied().unwrap_or(0);

            if timer_lo == 0 && timer_hi == 0 {
                return Err(SovereignStagesError::VerifyPtimerDead { pmc });
            }

            // VRAM sentinel via PRAMIN
            let vram_ok = pramin_sentinel_test(bar0);

            let detail = format!(
                "ptimer=0x{timer_hi:08x}_{timer_lo:08x} pmc=0x{pmc:08x} vram={}",
                if vram_ok { "ok" } else { "FAILED" },
            );

            if vram_ok {
                tracing::info!("sovereign verify: {detail}");
                Ok(detail)
            } else {
                tracing::warn!("sovereign verify: VRAM sentinel failed but PTIMER alive");
                Err(SovereignStagesError::VerifyVramSentinelFailed { detail })
            }
        }
        crate::vfio::isolation::IsolationResult::Timeout => {
            Err(SovereignStagesError::VerifyTimeout)
        }
        crate::vfio::isolation::IsolationResult::ChildFailed { status } => {
            Err(SovereignStagesError::VerifyChildFailed { status })
        }
        crate::vfio::isolation::IsolationResult::ForkError(e) => {
            Err(SovereignStagesError::VerifyFork(e))
        }
    }
}
