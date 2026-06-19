// SPDX-License-Identifier: AGPL-3.0-or-later
//! Live VFIO dispatch state: doorbell strategy, DMA buffers, GPFIFO submission.

use std::borrow::Cow;
use std::collections::HashMap;

use crate::BufferHandle;
use crate::error::{DriverError, DriverResult};

use super::{GPFIFO_ENTRIES, IOVA_LIMIT, PAGE_SIZE};

/// Doorbell strategy: Volta+ uses NV_USERMODE, Kepler uses GK104 per-channel.
#[derive(Debug, Clone, Copy)]
pub(crate) enum DoorbellKind {
    /// Volta+ `NV_USERMODE_NOTIFY_CHANNEL_PENDING` at BAR0 0x81_0090.
    Usermode,
    /// Kepler GK104 per-channel doorbell at `0x3000 + ch_id * 8`.
    Gk104 { channel_id: u32 },
}

/// Live VFIO state for PBDMA dispatch. Populated by [`super::NvVfioComputeDevice::open_vfio`].
pub(crate) struct VfioDispatchState {
    pub(crate) device: crate::vfio::VfioDevice,
    pub(crate) bar0: crate::vfio::device::MappedBar,
    pub(crate) channel: crate::vfio::channel::VfioChannel,
    pub(crate) dma_backend: crate::vfio::device::DmaBackend,
    pub(crate) gpfifo: crate::vfio::dma::DmaBuffer,
    pub(crate) userd: crate::vfio::dma::DmaBuffer,
    #[expect(dead_code, reason = "GR context buffer held for DMA lifetime")]
    pub(crate) gr_ctx: Option<crate::vfio::dma::DmaBuffer>,
    /// Semaphore buffer for Blackwell+ completion signaling (GP_GET removed from USERD).
    pub(crate) semaphore: Option<crate::vfio::dma::DmaBuffer>,
    /// Expected semaphore payload value for the next sync.
    pub(crate) semaphore_value: u32,
    pub(crate) buffers: HashMap<u32, crate::vfio::dma::DmaBuffer>,
    pub(crate) inflight: Vec<BufferHandle>,
    pub(crate) next_handle: u32,
    pub(crate) next_iova: u64,
    pub(crate) gp_put: u32,
    pub(crate) doorbell: DoorbellKind,
    /// Completion strategy for this GPU generation.
    pub(crate) completion: super::super::generation::CompletionStrategy,
    /// BAR0 base offset of the target PBDMA for direct GP_PUT writes.
    /// On warm-caught GV100, the scheduler doesn't reliably propagate
    /// USERD GP_PUT to the PBDMA; direct writes ensure GPFIFO consumption.
    pub(crate) target_pbdma_base: Option<usize>,
}

impl VfioDispatchState {
    /// Allocate a DMA buffer at the next available IOVA, advancing the bump pointer.
    pub(crate) fn alloc_next_dma(
        &mut self,
        size: usize,
        what: &str,
    ) -> DriverResult<crate::vfio::dma::DmaBuffer> {
        let aligned = size.div_ceil(PAGE_SIZE as usize) * PAGE_SIZE as usize;
        let iova = self.next_iova;
        if iova + aligned as u64 > IOVA_LIMIT {
            return Err(DriverError::MmapFailed(Cow::Owned(format!(
                "IOVA space exhausted for {what}"
            ))));
        }
        let buf = crate::vfio::dma::DmaBuffer::new(self.dma_backend.clone(), aligned, iova)?;
        self.next_iova = iova + aligned as u64;
        Ok(buf)
    }

    /// Submit a pushbuffer via GPFIFO + doorbell.
    pub(crate) fn submit_pushbuffer(&mut self, pb_bytes: &[u8]) -> DriverResult<()> {
        use crate::vfio::channel::registers::{pbdma, ramuserd};

        let dword_count = (pb_bytes.len() / 4) as u64;
        let mut pb_buf = self.alloc_next_dma(pb_bytes.len(), "pushbuffer")?;
        pb_buf.as_mut_slice()[..pb_bytes.len()].copy_from_slice(pb_bytes);

        let gp_entry_lo = (pb_buf.iova() & 0xFFFF_FFFC) as u32;
        let gp_entry_hi = (dword_count as u32) << 10;
        let gp_entry = (gp_entry_lo as u64) | ((gp_entry_hi as u64) << 32);

        let gp_offset = (self.gp_put as usize) * 8;
        self.gpfifo.as_mut_slice()[gp_offset..gp_offset + 8]
            .copy_from_slice(&gp_entry.to_le_bytes());

        let new_put = (self.gp_put + 1) % GPFIFO_ENTRIES;
        self.userd.volatile_write_u32(ramuserd::GP_PUT, new_put);
        std::sync::atomic::fence(std::sync::atomic::Ordering::SeqCst);

        // On warm-caught GV100, also write GP_PUT directly to the PBDMA's
        // direct register (0x054) and CTX register (same offset). The
        // scheduler doesn't propagate USERD GP_PUT to PBDMAs after warm
        // handoff — direct register write ensures the PBDMA sees pending
        // GPFIFO entries immediately.
        if let Some(pb) = self.target_pbdma_base {
            let _ = self.bar0.write_u32(pb + pbdma::GP_PUT, new_put);
        }

        let doorbell_addr = match self.doorbell {
            DoorbellKind::Usermode => {
                crate::vfio::channel::registers::usermode::NOTIFY_CHANNEL_PENDING
            }
            DoorbellKind::Gk104 { channel_id } => {
                crate::vfio::channel::registers::usermode::gk104_doorbell(channel_id)
            }
        };
        self.bar0
            .write_u32(doorbell_addr, self.channel.id())
            .map_err(|e| DriverError::SubmitFailed(Cow::Owned(format!("doorbell write: {e}"))))?;

        self.gp_put = new_put;
        self.track_inflight(pb_buf);
        Ok(())
    }

    /// Track a transient DMA buffer for cleanup after sync.
    pub(crate) fn track_inflight(&mut self, dma: crate::vfio::dma::DmaBuffer) {
        let id = self.next_handle;
        self.next_handle += 1;
        self.buffers.insert(id, dma);
        self.inflight.push(BufferHandle(id));
    }
}
