// SPDX-License-Identifier: AGPL-3.0-or-later
//! GPFIFO/USERD/GR-ctx buffer allocation and PFIFO channel creation.

use std::collections::HashMap;

use crate::error::DriverResult;
use crate::vfio::channel::VfioChannel;
use crate::vfio::device::{DmaBackend, MappedBar};
use crate::vfio::dma::DmaBuffer;

use super::super::generation::{CompletionStrategy, GenerationProfile};
use super::{
    DoorbellKind, GPFIFO_ENTRIES, GPFIFO_IOVA, GR_CTX_IOVA, GR_CTX_SIZE, PAGE_SIZE,
    USER_BUFFER_BASE_IOVA, USERD_IOVA, VfioDispatchState,
};

/// Result of PFIFO channel + DMA buffer initialization.
pub(crate) struct ChannelInitResult {
    pub gpfifo: DmaBuffer,
    pub userd: DmaBuffer,
    pub gr_ctx: Option<DmaBuffer>,
    pub channel: VfioChannel,
    pub doorbell: DoorbellKind,
}

/// Allocate GPFIFO, USERD, optional GR context, and create PFIFO channel.
pub(crate) fn init_channel_buffers(
    dma_backend: &DmaBackend,
    bar0: &MappedBar,
    profile: &GenerationProfile,
    is_kepler: bool,
    fecs_ready: bool,
    bdf: &str,
    log_context: &str,
) -> DriverResult<ChannelInitResult> {
    let gpfifo = DmaBuffer::new(dma_backend.clone(), 4096, GPFIFO_IOVA)?;
    let userd = DmaBuffer::new(dma_backend.clone(), 4096, USERD_IOVA)?;

    let gr_ctx = if !is_kepler && fecs_ready {
        let ctx = DmaBuffer::new(dma_backend.clone(), GR_CTX_SIZE, GR_CTX_IOVA)?;
        if log_context.is_empty() {
            tracing::info!(
                bdf = %bdf,
                gr_ctx_iova = format_args!("{GR_CTX_IOVA:#x}"),
                gr_ctx_size = GR_CTX_SIZE,
                "GR context buffer allocated for FECS"
            );
        } else {
            tracing::info!(
                bdf = %bdf,
                gr_ctx_iova = format_args!("{GR_CTX_IOVA:#x}"),
                "GR context buffer allocated ({log_context})"
            );
        }
        Some(ctx)
    } else {
        None
    };

    let mut ch = VfioChannel::create_for_profile(
        dma_backend.clone(),
        bar0,
        GPFIFO_IOVA,
        GPFIFO_ENTRIES,
        USERD_IOVA,
        0,
        profile,
        fecs_ready,
    )?;

    let doorbell = if is_kepler {
        DoorbellKind::Gk104 {
            channel_id: ch.id(),
        }
    } else {
        DoorbellKind::Usermode
    };

    if !is_kepler && gr_ctx.is_some() {
        ch.write_gr_context_ptr(GR_CTX_IOVA, 4);
        ch.resubmit_runlist(bar0)?;
    }

    Ok(ChannelInitResult {
        gpfifo,
        userd,
        gr_ctx,
        channel: ch,
        doorbell,
    })
}

/// Like [`init_channel_buffers`] but allows overriding the PFIFO warm_handoff
/// independently from `fecs_ready`. Use after a PMC PFIFO reset when FECS is
/// still alive but PFIFO needs cold initialization.
#[expect(
    clippy::too_many_arguments,
    reason = "hardware init requires explicit state flags"
)]
pub(crate) fn init_channel_buffers_with_pfifo_config(
    dma_backend: &DmaBackend,
    bar0: &MappedBar,
    profile: &GenerationProfile,
    is_kepler: bool,
    fecs_ready: bool,
    pfifo_warm: bool,
    bdf: &str,
    log_context: &str,
) -> DriverResult<ChannelInitResult> {
    let gpfifo = DmaBuffer::new(dma_backend.clone(), 4096, GPFIFO_IOVA)?;
    let userd = DmaBuffer::new(dma_backend.clone(), 4096, USERD_IOVA)?;

    let gr_ctx = if !is_kepler && fecs_ready {
        let ctx = DmaBuffer::new(dma_backend.clone(), GR_CTX_SIZE, GR_CTX_IOVA)?;
        tracing::info!(
            bdf = %bdf,
            gr_ctx_iova = format_args!("{GR_CTX_IOVA:#x}"),
            "GR context buffer allocated ({log_context})"
        );
        Some(ctx)
    } else {
        None
    };

    let mut ch = VfioChannel::create_for_profile(
        dma_backend.clone(),
        bar0,
        GPFIFO_IOVA,
        GPFIFO_ENTRIES,
        USERD_IOVA,
        0,
        profile,
        pfifo_warm,
    )?;

    let doorbell = if is_kepler {
        DoorbellKind::Gk104 {
            channel_id: ch.id(),
        }
    } else {
        DoorbellKind::Usermode
    };

    if !is_kepler && gr_ctx.is_some() {
        ch.write_gr_context_ptr(GR_CTX_IOVA, 4);
        ch.resubmit_runlist(bar0)?;
    }

    Ok(ChannelInitResult {
        gpfifo,
        userd,
        gr_ctx,
        channel: ch,
        doorbell,
    })
}

/// Allocate optional semaphore buffer for Blackwell+ completion signaling.
pub(crate) fn alloc_semaphore_buffer(
    dma_backend: &DmaBackend,
    completion: CompletionStrategy,
    bdf: &str,
    log_context: &str,
) -> DriverResult<Option<DmaBuffer>> {
    if !matches!(completion, CompletionStrategy::SemaphoreFence) {
        return Ok(None);
    }

    let mut sem = DmaBuffer::new(dma_backend.clone(), 4096, USER_BUFFER_BASE_IOVA)?;
    sem.as_mut_slice()[..4].copy_from_slice(&0u32.to_le_bytes());
    if log_context.is_empty() {
        tracing::info!(
            bdf = %bdf,
            sem_iova = format_args!("{USER_BUFFER_BASE_IOVA:#x}"),
            "semaphore buffer allocated for SemaphoreFence completion"
        );
    } else {
        tracing::info!(
            bdf = %bdf,
            sem_iova = format_args!("{USER_BUFFER_BASE_IOVA:#x}"),
            "semaphore buffer allocated ({log_context})"
        );
    }
    Ok(Some(sem))
}

/// Build final [`VfioDispatchState`] from initialized components.
pub(crate) fn build_dispatch_state(
    device: crate::vfio::VfioDevice,
    bar0: MappedBar,
    init: ChannelInitResult,
    dma_backend: DmaBackend,
    semaphore: Option<DmaBuffer>,
    completion: CompletionStrategy,
    target_pbdma_base: Option<usize>,
) -> VfioDispatchState {
    let sem_offset = if semaphore.is_some() { PAGE_SIZE } else { 0 };

    VfioDispatchState {
        device,
        bar0,
        channel: init.channel,
        dma_backend,
        gpfifo: init.gpfifo,
        userd: init.userd,
        gr_ctx: init.gr_ctx,
        semaphore,
        semaphore_value: 0,
        buffers: HashMap::new(),
        inflight: Vec::new(),
        next_handle: 1,
        next_iova: USER_BUFFER_BASE_IOVA + sem_offset,
        gp_put: 0,
        doorbell: init.doorbell,
        completion,
        target_pbdma_base,
    }
}

/// Phase A (Exp 229): Adopt an RM-created channel instead of creating a new one.
///
/// Scans PCCSR for channels that RM left in an ACTIVE state (status >= 5).
/// If found, reads the instance block pointer to extract GPFIFO base and
/// USERD base from RAMFC, then maps DMA buffers at RM's addresses.
///
/// Returns `None` if no adoptable RM channel is found (all IDLE/PENDING).
pub(crate) fn adopt_rm_channel(
    dma_backend: &DmaBackend,
    bar0: &MappedBar,
    _profile: &GenerationProfile,
    bdf: &str,
    rm_channel_id: Option<u32>,
) -> DriverResult<Option<ChannelInitResult>> {
    use crate::vfio::channel::registers::pccsr;

    // If we have a specific RM channel ID, check that one first
    let scan_range = if let Some(id) = rm_channel_id {
        vec![id]
    } else {
        (0..64u32).collect()
    };

    let mut best_channel: Option<(u32, u32)> = None; // (channel_id, pccsr_val)

    for ch_id in &scan_range {
        let pccsr_val = bar0.read_u32(pccsr::channel(*ch_id)).unwrap_or(0);
        let status = pccsr::status(pccsr_val);
        let enabled = pccsr_val & 1;

        if enabled != 0 && status >= 5 {
            tracing::info!(
                bdf = %bdf,
                ch_id,
                status,
                pccsr = format_args!("{pccsr_val:#010x}"),
                status_name = pccsr::status_name(pccsr_val),
                "adopt_rm_channel: found ACTIVE RM channel"
            );
            best_channel = Some((*ch_id, pccsr_val));
            break;
        } else if enabled != 0 {
            tracing::debug!(
                bdf = %bdf,
                ch_id,
                status,
                pccsr = format_args!("{pccsr_val:#010x}"),
                "adopt_rm_channel: skipping non-ACTIVE channel"
            );
        }
    }

    let (channel_id, pccsr_val) = match best_channel {
        Some(c) => c,
        None => {
            tracing::info!(
                bdf = %bdf,
                "adopt_rm_channel: no ACTIVE RM channels found in PCCSR — Phase A not available"
            );
            return Ok(None);
        }
    };

    // Read the instance block pointer from PCCSR INST register
    let inst_val = bar0.read_u32(pccsr::inst(channel_id)).unwrap_or(0);
    let inst_ptr = (inst_val & 0x0FFF_FFFF) as u64;
    let inst_target = (inst_val >> 28) & 0x3;
    let inst_iova = inst_ptr << 12; // INST_PTR is in 4K pages

    tracing::info!(
        bdf = %bdf,
        channel_id,
        inst_ptr = format_args!("{inst_ptr:#010x}"),
        inst_target,
        inst_iova = format_args!("{inst_iova:#018x}"),
        pccsr = format_args!("{pccsr_val:#010x}"),
        "adopt_rm_channel: reading RM channel instance block"
    );

    // Allocate our own DMA buffers at sovereign IOVAs — we can't use RM's
    // addresses directly since we don't know their host virtual mappings.
    // Instead, we create the channel with our own buffers and skip the
    // channel creation in PFIFO (it already exists).
    let gpfifo = DmaBuffer::new(dma_backend.clone(), 4096, GPFIFO_IOVA)?;
    let userd = DmaBuffer::new(dma_backend.clone(), 4096, USERD_IOVA)?;

    let ctx = DmaBuffer::new(dma_backend.clone(), GR_CTX_SIZE, GR_CTX_IOVA)?;
    tracing::info!(
        bdf = %bdf,
        gr_ctx_iova = format_args!("{GR_CTX_IOVA:#x}"),
        "adopt_rm_channel: GR context buffer allocated for adopted channel"
    );
    let gr_ctx = Some(ctx);

    // Build an adopted VfioChannel that wraps the existing RM channel
    // without creating a new one in PFIFO hardware.
    let channel = VfioChannel::adopt_existing(
        dma_backend.clone(),
        bar0,
        channel_id,
        GPFIFO_IOVA,
        GPFIFO_ENTRIES,
        USERD_IOVA,
    )?;

    tracing::info!(
        bdf = %bdf,
        channel_id = channel.id(),
        "adopt_rm_channel: RM channel adopted for sovereign dispatch"
    );

    Ok(Some(ChannelInitResult {
        gpfifo,
        userd,
        gr_ctx,
        channel,
        doorbell: DoorbellKind::Usermode,
    }))
}

/// Free inflight pushbuffers after sync (used by compute.rs).
pub(crate) fn clear_inflight(state: &mut VfioDispatchState) {
    let inflight = std::mem::take(&mut state.inflight);
    for handle in inflight {
        state.buffers.remove(&handle.0);
    }
}
