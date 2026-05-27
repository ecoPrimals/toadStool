// SPDX-License-Identifier: AGPL-3.0-or-later
//! CE (Copy Engine) validation — proves the sovereign DMA pipeline.
//!
//! Independent of the GR engine and GPCs. If CE dispatch works after a warm
//! handoff, it confirms that VFIO DMA mapping, PFIFO scheduling, PBDMA
//! command delivery, and buffer read-back are all functional.
//!
//! The only missing piece for full compute is GPC power (the PGRAPH wall).

use std::time::{Duration, Instant};

use crate::nv::pushbuf::PushBuf;
use crate::vfio::channel::VfioChannel;
use crate::vfio::channel::pfifo;
use crate::vfio::channel::registers::{pbdma, usermode};
use crate::vfio::device::{DmaBackend, MappedBar};
use crate::vfio::dma::DmaBuffer;

// CE validation uses IOVAs in the user buffer region (0x12_0000+)
// to avoid conflicting with existing dispatch channel infrastructure.
const CE_GPFIFO_IOVA: u64 = 0x20_0000;
const CE_USERD_IOVA: u64 = 0x20_1000;
const CE_SRC_IOVA: u64 = 0x20_2000;
const CE_DST_IOVA: u64 = 0x20_3000;
const CE_PB_IOVA: u64 = 0x20_4000;
const CE_GPFIFO_ENTRIES: u32 = 512;
const CE_BUF_SIZE: usize = 4096;

const MAGIC_PATTERN: u32 = 0xDEAD_BEEF;

/// Result of a CE validation attempt.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CeValidationResult {
    pub ce_runlist: Option<u32>,
    pub ce_pbdma: Option<usize>,
    pub channel_created: bool,
    pub pushbuf_submitted: bool,
    pub gp_get_advanced: bool,
    pub readback_correct: bool,
    pub readback_sample: Vec<u32>,
    pub src_sample: Vec<u32>,
    pub elapsed_ms: u64,
    pub error: Option<String>,
    pub pbdma_diagnostics: Option<PbdmaDiagnostics>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PbdmaDiagnostics {
    pub intr_0: u32,
    pub gp_get: u32,
    pub gp_put: u32,
    pub pb_header: u32,
    pub method0: u32,
    pub status: u32,
}

/// Run CE validation: discover CE runlist, create channel, submit DMA copy, readback.
///
/// Uses the CE DMA class from `profile` when provided, falling back to
/// `VOLTA_DMA_COPY_A` (0xC3B5) for backward compatibility.
pub fn validate_ce(
    bar0: &MappedBar,
    dma_backend: DmaBackend,
) -> CeValidationResult {
    validate_ce_with_profile(bar0, dma_backend, None)
}

/// Run CE validation with an explicit generation profile for CE class selection.
pub fn validate_ce_with_profile(
    bar0: &MappedBar,
    dma_backend: DmaBackend,
    profile: Option<&crate::nv::generation::GenerationProfile>,
) -> CeValidationResult {
    let start = Instant::now();
    let mut result = CeValidationResult {
        ce_runlist: None,
        ce_pbdma: None,
        channel_created: false,
        pushbuf_submitted: false,
        gp_get_advanced: false,
        readback_correct: false,
        readback_sample: Vec::new(),
        src_sample: Vec::new(),
        elapsed_ms: 0,
        error: None,
        pbdma_diagnostics: None,
    };

    // Step 1: Discover CE runlist from engine topology table.
    let ce_rl = match pfifo::discover_ce_runlist(bar0) {
        Some(rl) => {
            tracing::info!(ce_runlist = rl, "CE runlist discovered");
            result.ce_runlist = Some(rl);
            rl
        }
        None => {
            result.error = Some("no CE engine found in topology table".into());
            result.elapsed_ms = start.elapsed().as_millis() as u64;
            return result;
        }
    };

    // Find the PBDMA serving this runlist from RUNLIST_PBDMA_MAP.
    let ce_pbdma = pfifo::find_pbdma_for_runlist(bar0, ce_rl);
    result.ce_pbdma = ce_pbdma;
    tracing::info!(
        ce_pbdma = ?ce_pbdma,
        ce_pbdma_base = ce_pbdma.map(|p| format!("{:#x}", 0x0004_0000 + p * 0x2000)),
        "CE PBDMA from RUNLIST_PBDMA_MAP"
    );

    // Step 2: Create a channel on the CE runlist.
    let channel_id = 1_u32;
    let gpfifo = match DmaBuffer::new(dma_backend.clone(), 4096, CE_GPFIFO_IOVA) {
        Ok(b) => b,
        Err(e) => {
            result.error = Some(format!("GPFIFO alloc: {e}"));
            result.elapsed_ms = start.elapsed().as_millis() as u64;
            return result;
        }
    };
    let userd = match DmaBuffer::new(dma_backend.clone(), 4096, CE_USERD_IOVA) {
        Ok(b) => b,
        Err(e) => {
            result.error = Some(format!("USERD alloc: {e}"));
            result.elapsed_ms = start.elapsed().as_millis() as u64;
            return result;
        }
    };
    let _ = gpfifo; // held for DMA lifetime
    let _ = userd;

    let chan = match VfioChannel::create_on_runlist(
        dma_backend.clone(),
        bar0,
        CE_GPFIFO_IOVA,
        CE_GPFIFO_ENTRIES,
        CE_USERD_IOVA,
        channel_id,
        ce_rl,
    ) {
        Ok(c) => c,
        Err(e) => {
            result.error = Some(format!("channel create: {e}"));
            result.elapsed_ms = start.elapsed().as_millis() as u64;
            return result;
        }
    };
    result.channel_created = true;
    tracing::info!(channel_id = chan.id(), runlist = ce_rl, "CE channel created");

    // Force-program the CE PBDMA with channel state.
    // The channel was created via the GR runlist PFIFO path (which programs
    // the GR PBDMA). The CE PBDMA needs the GPFIFO/USERD/signature loaded.
    if let Some(pid) = ce_pbdma {
        let pb_base = 0x0004_0000 + pid * 0x2000;

        // Clear any stale interrupts on the CE PBDMA.
        let _ = bar0.write_u32(pb_base + 0x100, 0xFFFF_FFFF); // INTR_0 W1C
        let _ = bar0.write_u32(pb_base + 0x108, 0xFFFF_FFFF); // INTR_STALL W1C
        let _ = bar0.write_u32(pb_base + 0x148, 0xFFFF_FFFF); // HCE_INTR W1C

        // Program GPFIFO base.
        let _ = bar0.write_u32(pb_base + pbdma::GP_BASE_LO, (CE_GPFIFO_IOVA & 0xFFFF_FFFF) as u32);
        let _ = bar0.write_u32(pb_base + pbdma::GP_BASE_HI, (CE_GPFIFO_IOVA >> 32) as u32);
        // GP_PUT = 0, GP_FETCH = 0 (fresh channel).
        let _ = bar0.write_u32(pb_base + pbdma::GP_PUT, 0);
        let _ = bar0.write_u32(pb_base + pbdma::CTX_GP_FETCH, 0);

        // Program USERD pointer.
        let userd_lo = (CE_USERD_IOVA & 0xFFFF_FFFC) as u32 | 0x2; // valid + aperture=sysmem
        let _ = bar0.write_u32(pb_base + pbdma::USERD_LO, userd_lo);
        let _ = bar0.write_u32(pb_base + pbdma::USERD_HI, (CE_USERD_IOVA >> 32) as u32);

        // Set signature from channel (nouveau uses 0x3ACE).
        let _ = bar0.write_u32(pb_base + pbdma::SIGNATURE, 0x0000_3ACE);

        tracing::info!(
            pbdma = pid,
            pb_base = format_args!("{pb_base:#x}"),
            gpfifo = format_args!("{CE_GPFIFO_IOVA:#x}"),
            userd = format_args!("{CE_USERD_IOVA:#x}"),
            "CE PBDMA force-programmed"
        );
    }

    // Step 3: Allocate source and destination DMA buffers.
    let mut src_buf = match DmaBuffer::new(dma_backend.clone(), CE_BUF_SIZE, CE_SRC_IOVA) {
        Ok(b) => b,
        Err(e) => {
            result.error = Some(format!("src buf alloc: {e}"));
            result.elapsed_ms = start.elapsed().as_millis() as u64;
            return result;
        }
    };
    let mut dst_buf = match DmaBuffer::new(dma_backend.clone(), CE_BUF_SIZE, CE_DST_IOVA) {
        Ok(b) => b,
        Err(e) => {
            result.error = Some(format!("dst buf alloc: {e}"));
            result.elapsed_ms = start.elapsed().as_millis() as u64;
            return result;
        }
    };

    // Fill source with known pattern, zero destination.
    let src_words = bytemuck::cast_slice_mut::<u8, u32>(src_buf.as_mut_slice());
    for w in src_words.iter_mut() {
        *w = MAGIC_PATTERN;
    }
    dst_buf.as_mut_slice().fill(0);

    result.src_sample = bytemuck::cast_slice::<u8, u32>(&src_buf.as_slice()[..16]).to_vec();

    // Step 4: Build CE pushbuffer (init + DMA copy).
    let ce_class = profile
        .map(|p| p.ce_class)
        .unwrap_or(crate::nv::pushbuf::ce::VOLTA_DMA_COPY_A);
    let mut pb = PushBuf::ce_init(ce_class);
    let copy_pb = PushBuf::ce_dma_copy(CE_SRC_IOVA, CE_DST_IOVA, CE_BUF_SIZE as u32);
    pb.append(&copy_pb);
    let pb_bytes = pb.as_bytes();

    // Allocate PB DMA buffer and copy the pushbuffer into it.
    let mut pb_buf = match DmaBuffer::new(dma_backend.clone(), pb_bytes.len().max(4096), CE_PB_IOVA) {
        Ok(b) => b,
        Err(e) => {
            result.error = Some(format!("pb buf alloc: {e}"));
            result.elapsed_ms = start.elapsed().as_millis() as u64;
            return result;
        }
    };
    pb_buf.as_mut_slice()[..pb_bytes.len()].copy_from_slice(pb_bytes);

    // Step 5: Submit via GPFIFO.
    let dword_count = (pb_bytes.len() / 4) as u64;
    let gp_entry_lo = (CE_PB_IOVA & 0xFFFF_FFFC) as u32;
    let gp_entry_hi = (dword_count as u32) << 10;
    let gp_entry = (gp_entry_lo as u64) | ((gp_entry_hi as u64) << 32);

    // Write GP entry at slot 0.
    let gpfifo_buf = match DmaBuffer::new(dma_backend.clone(), 4096, CE_GPFIFO_IOVA) {
        Ok(mut b) => {
            b.as_mut_slice()[0..8].copy_from_slice(&gp_entry.to_le_bytes());
            b
        }
        Err(e) => {
            result.error = Some(format!("gpfifo re-map: {e}"));
            result.elapsed_ms = start.elapsed().as_millis() as u64;
            return result;
        }
    };

    // Write GP_PUT = 1 to USERD (offset 35*4 = 0x8C).
    const USERD_GP_PUT: usize = 35 * 4;
    let userd_buf = match DmaBuffer::new(dma_backend.clone(), 4096, CE_USERD_IOVA) {
        Ok(b) => {
            b.volatile_write_u32(USERD_GP_PUT, 1);
            b
        }
        Err(e) => {
            result.error = Some(format!("userd re-map: {e}"));
            result.elapsed_ms = start.elapsed().as_millis() as u64;
            return result;
        }
    };
    std::sync::atomic::fence(std::sync::atomic::Ordering::SeqCst);

    // Direct PBDMA GP_PUT write for warm-caught GPUs.
    if let Some(pid) = result.ce_pbdma {
        let pb_base = 0x0004_0000 + pid * 0x2000;
        let _ = bar0.write_u32(pb_base + pbdma::GP_PUT, 1);
        tracing::info!(pbdma = pid, pb_base = format_args!("{pb_base:#x}"), "direct CE PBDMA GP_PUT=1");
    }

    // Ring doorbell (Volta+ USERMODE).
    let _ = bar0.write_u32(usermode::NOTIFY_CHANNEL_PENDING, chan.id());

    result.pushbuf_submitted = true;
    tracing::info!("CE pushbuffer submitted, polling GP_GET");

    // Step 6: Poll GP_GET for consumption.
    let poll_start = Instant::now();
    let timeout = Duration::from_millis(500);
    let mut gp_get_val = 0_u32;

    while poll_start.elapsed() < timeout {
        // Read GP_FETCH (GP_GET equivalent) from PBDMA context area.
        if let Some(pid) = result.ce_pbdma {
            let pb_base = 0x0004_0000 + pid * 0x2000;
            gp_get_val = bar0.read_u32(pb_base + pbdma::CTX_GP_FETCH).unwrap_or(0);
        }
        if gp_get_val >= 1 {
            result.gp_get_advanced = true;
            tracing::info!(gp_get = gp_get_val, elapsed_us = poll_start.elapsed().as_micros(), "GP_GET advanced — CE consumed pushbuffer");
            break;
        }
        std::thread::sleep(Duration::from_micros(100));
    }

    if !result.gp_get_advanced {
        // Capture PBDMA diagnostics.
        if let Some(pid) = result.ce_pbdma {
            let pb_base = 0x0004_0000 + pid * 0x2000;
            result.pbdma_diagnostics = Some(PbdmaDiagnostics {
                intr_0: bar0.read_u32(pb_base + 0x100).unwrap_or(0xDEAD),
                gp_get: bar0.read_u32(pb_base + pbdma::CTX_GP_FETCH).unwrap_or(0xDEAD),
                gp_put: bar0.read_u32(pb_base + pbdma::GP_PUT).unwrap_or(0xDEAD),
                pb_header: bar0.read_u32(pb_base + 0x084).unwrap_or(0xDEAD),
                method0: bar0.read_u32(pb_base + 0x064).unwrap_or(0xDEAD),
                status: bar0.read_u32(pb_base + 0x068).unwrap_or(0xDEAD),
            });
            if let Some(diag) = &result.pbdma_diagnostics {
                tracing::warn!(
                    intr_0 = format_args!("{:#010x}", diag.intr_0),
                    "CE PBDMA GP_GET did not advance — PBDMA diagnostics captured"
                );
            }
        }
        result.error = Some("GP_GET did not advance within timeout".into());
    }

    // Step 7: Read back destination buffer.
    let dst_words = bytemuck::cast_slice::<u8, u32>(dst_buf.as_slice());
    result.readback_sample = dst_words[..4.min(dst_words.len())].to_vec();

    if result.gp_get_advanced {
        let correct = dst_words.iter().all(|&w| w == MAGIC_PATTERN);
        result.readback_correct = correct;
        if correct {
            tracing::info!("CE DMA copy VERIFIED — sovereign DMA pipeline functional");
        } else {
            let nonzero = dst_words.iter().filter(|&&w| w != 0).count();
            tracing::warn!(
                nonzero_words = nonzero,
                total_words = dst_words.len(),
                sample_0 = format_args!("{:#010x}", dst_words.first().copied().unwrap_or(0)),
                "CE DMA copy completed but readback mismatch"
            );
        }
    }

    // Hold DMA buffers in scope until validation is complete.
    drop(gpfifo_buf);
    drop(userd_buf);
    drop(pb_buf);
    drop(src_buf);
    drop(dst_buf);

    result.elapsed_ms = start.elapsed().as_millis() as u64;
    result
}
