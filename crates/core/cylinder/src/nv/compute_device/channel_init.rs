// SPDX-License-Identifier: AGPL-3.0-or-later
//! GPFIFO/USERD/GR-ctx buffer allocation and PFIFO channel creation.

use std::collections::HashMap;

use crate::error::DriverResult;
use crate::vfio::channel::VfioChannel;
use crate::vfio::device::{DmaBackend, MappedBar};
use crate::vfio::dma::DmaBuffer;

use super::super::generation::{CompletionStrategy, GenerationProfile};
use super::{DoorbellKind, GPFIFO_ENTRIES, GPFIFO_IOVA, GR_CTX_IOVA, GR_CTX_SIZE, PAGE_SIZE, USER_BUFFER_BASE_IOVA, USERD_IOVA, VfioDispatchState};

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
    let sem_offset = if semaphore.is_some() {
        PAGE_SIZE
    } else {
        0
    };

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

/// Free inflight pushbuffers after sync (used by compute.rs).
pub(crate) fn clear_inflight(state: &mut VfioDispatchState) {
    let inflight = std::mem::take(&mut state.inflight);
    for handle in inflight {
        state.buffers.remove(&handle.0);
    }
}
