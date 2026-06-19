// SPDX-License-Identifier: AGPL-3.0-or-later
//! [`ComputeDevice`] trait implementation for NVIDIA VFIO direct dispatch.

use std::borrow::Cow;

use crate::error::{DriverError, DriverResult};
use crate::{
    BufferHandle, ComputeDevice, DispatchDims, HardwareCapabilities, MemoryDomain, ShaderInfo,
};

use super::NvVfioComputeDevice;
#[cfg(target_os = "linux")]
use super::channel_init::clear_inflight;
use super::{IOVA_LIMIT, PAGE_SIZE};

impl ComputeDevice for NvVfioComputeDevice {
    fn alloc(&mut self, size: u64, _domain: MemoryDomain) -> DriverResult<BufferHandle> {
        if !self.fecs_ready {
            return Err(DriverError::Unsupported(
                "NVIDIA VFIO alloc requires FECS compute context — see GspBridge".into(),
            ));
        }

        #[cfg(target_os = "linux")]
        {
            let state = self.vfio_state.as_mut().ok_or_else(|| {
                DriverError::Unsupported("VFIO not opened — call open_vfio() before alloc".into())
            })?;

            let aligned_size = (size as usize).div_ceil(PAGE_SIZE as usize) * PAGE_SIZE as usize;
            let iova = state.next_iova;
            if iova + aligned_size as u64 > IOVA_LIMIT {
                return Err(DriverError::MmapFailed(Cow::Borrowed(
                    "IOVA space exhausted (2 MiB identity map limit)",
                )));
            }

            let buf =
                crate::vfio::dma::DmaBuffer::new(state.dma_backend.clone(), aligned_size, iova)?;

            let handle_id = state.next_handle;
            state.next_handle += 1;
            state.next_iova = iova + aligned_size as u64;
            state.buffers.insert(handle_id, buf);

            tracing::debug!(
                handle = handle_id,
                iova = format_args!("{iova:#x}"),
                size = aligned_size,
                "NVIDIA VFIO buffer allocated"
            );

            Ok(BufferHandle(handle_id))
        }

        #[cfg(not(target_os = "linux"))]
        Err(DriverError::Unsupported(
            "NVIDIA VFIO dispatch requires Linux".into(),
        ))
    }

    fn free(&mut self, handle: BufferHandle) -> DriverResult<()> {
        #[cfg(target_os = "linux")]
        {
            let state = self
                .vfio_state
                .as_mut()
                .ok_or_else(|| DriverError::Unsupported("VFIO not opened".into()))?;
            state
                .buffers
                .remove(&handle.0)
                .ok_or(DriverError::BufferNotFound(handle))?;
            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = handle;
            Err(DriverError::Unsupported(
                "NVIDIA VFIO requires Linux".into(),
            ))
        }
    }

    fn upload(&mut self, handle: BufferHandle, offset: u64, data: &[u8]) -> DriverResult<()> {
        #[cfg(target_os = "linux")]
        {
            let state = self
                .vfio_state
                .as_mut()
                .ok_or_else(|| DriverError::Unsupported("VFIO not opened".into()))?;
            let buf = state
                .buffers
                .get_mut(&handle.0)
                .ok_or(DriverError::BufferNotFound(handle))?;

            let start = offset as usize;
            let end = start + data.len();
            let slice = buf.as_mut_slice();
            if end > slice.len() {
                return Err(DriverError::MmapFailed(Cow::Owned(format!(
                    "upload out of bounds: offset {start} + len {} > buf size {}",
                    data.len(),
                    slice.len()
                ))));
            }
            slice[start..end].copy_from_slice(data);
            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = (handle, offset, data);
            Err(DriverError::Unsupported(
                "NVIDIA VFIO requires Linux".into(),
            ))
        }
    }

    fn readback(&self, handle: BufferHandle, offset: u64, len: usize) -> DriverResult<Vec<u8>> {
        #[cfg(target_os = "linux")]
        {
            let state = self
                .vfio_state
                .as_ref()
                .ok_or_else(|| DriverError::Unsupported("VFIO not opened".into()))?;
            let buf = state
                .buffers
                .get(&handle.0)
                .ok_or(DriverError::BufferNotFound(handle))?;

            let start = offset as usize;
            let end = start + len;
            let slice = buf.as_slice();
            if end > slice.len() {
                return Err(DriverError::MmapFailed(Cow::Owned(format!(
                    "readback out of bounds: offset {start} + len {len} > buf size {}",
                    slice.len()
                ))));
            }
            Ok(slice[start..end].to_vec())
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = (handle, offset, len);
            Err(DriverError::Unsupported(
                "NVIDIA VFIO requires Linux".into(),
            ))
        }
    }

    fn dispatch(
        &mut self,
        shader: &[u8],
        buffers: &[BufferHandle],
        dims: DispatchDims,
        info: &ShaderInfo,
    ) -> DriverResult<()> {
        if !self.fecs_ready {
            return Err(DriverError::Unsupported(
                "NVIDIA VFIO dispatch requires FECS compute context — firmware loads but \
                 compute context never becomes ready. Production path: warm-handoff from \
                 nouveau/nvidia-470, or real GspBridge (shader compiler IPC or local absorption)"
                    .into(),
            ));
        }

        #[cfg(target_os = "linux")]
        {
            let state = self.vfio_state.as_mut().ok_or_else(|| {
                DriverError::Unsupported(
                    "VFIO not opened — call open_vfio() before dispatch".into(),
                )
            })?;

            if shader.is_empty() {
                return Err(DriverError::SubmitFailed(
                    "shader binary must be non-empty".into(),
                ));
            }

            let sm = self.sm;
            let profile = super::super::generation::profile_for_sm(sm);

            // 1. Upload shader binary
            let mut shader_buf = state.alloc_next_dma(shader.len(), "shader binary")?;
            let shader_iova = shader_buf.iova();
            shader_buf.as_mut_slice()[..shader.len()].copy_from_slice(shader);

            // 2. Build CBUF descriptor table (16-byte stride per binding)
            let desc_entry_size: usize = 16;
            let desc_size = buffers.len() * desc_entry_size;
            let mut desc_buf =
                state.alloc_next_dma(desc_size.max(PAGE_SIZE as usize), "descriptor table")?;
            let desc_iova = desc_buf.iova();
            for (i, handle) in buffers.iter().enumerate() {
                let buf = state
                    .buffers
                    .get(&handle.0)
                    .ok_or(DriverError::BufferNotFound(*handle))?;
                let offset = i * desc_entry_size;
                desc_buf.as_mut_slice()[offset..offset + 4]
                    .copy_from_slice(&(buf.iova() as u32).to_le_bytes());
                desc_buf.as_mut_slice()[offset + 4..offset + 8]
                    .copy_from_slice(&((buf.iova() >> 32) as u32).to_le_bytes());
                desc_buf.as_mut_slice()[offset + 8..offset + 12]
                    .copy_from_slice(&(buf.size() as u32).to_le_bytes());
            }

            // 3. Driver constants (grid dims for `@builtin(num_workgroups)`)
            let driver_const_data = super::super::qmd::encode_driver_constants(&dims);
            let mut dc_buf = state.alloc_next_dma(PAGE_SIZE as usize, "driver constants")?;
            let dc_iova = dc_buf.iova();
            dc_buf.as_mut_slice()[..driver_const_data.len()].copy_from_slice(&driver_const_data);

            // 4. Build QMD
            let workgroup = if info.workgroup.iter().any(|&d| d > 0) {
                info.workgroup
            } else {
                [64, 1, 1]
            };
            let cbufs = super::super::qmd::build_standard_cbufs(
                desc_iova,
                desc_size.max(64) as u32,
                dc_iova,
                super::super::qmd::DRIVER_CONST_SIZE,
            );
            let qmd_params = super::super::qmd::QmdParams {
                shader_va: shader_iova,
                grid: dims,
                workgroup,
                gpr_count: info.gpr_count.max(4),
                shared_mem_bytes: info.shared_mem_bytes,
                barrier_count: info.barrier_count,
                local_mem_low_bytes: info.local_mem_bytes.unwrap_or(0),
                cbufs,
            };
            let qmd_words = super::super::qmd::build_qmd(profile, &qmd_params);
            let qmd_bytes: &[u8] = bytemuck::cast_slice(&qmd_words);
            let mut qmd_buf = state.alloc_next_dma(qmd_bytes.len(), "QMD")?;
            let qmd_iova = qmd_buf.iova();
            qmd_buf.as_mut_slice()[..qmd_bytes.len()].copy_from_slice(qmd_bytes);

            // 5. Build + submit pushbuffer (compute init + dispatch + optional semaphore)
            let mut init_pb = super::super::pushbuf::PushBuf::compute_init(
                profile.compute_class,
                profile.local_mem_window,
                0,
                0,
            );
            init_pb.append(
                &super::super::pushbuf::PushBuf::compute_dispatch_with_launch(
                    profile.launch_method,
                    qmd_iova,
                ),
            );

            if matches!(
                state.completion,
                super::super::generation::CompletionStrategy::SemaphoreFence
            ) && let Some(sem) = &state.semaphore
            {
                state.semaphore_value = state.semaphore_value.wrapping_add(1);
                let release_pb = super::super::pushbuf::PushBuf::semaphore_release(
                    sem.iova(),
                    state.semaphore_value,
                    0,
                );
                init_pb.append(&release_pb);
            }

            state.submit_pushbuffer(init_pb.as_bytes())?;

            // Track transient DMA allocations for cleanup after sync.
            for dma in [shader_buf, desc_buf, dc_buf, qmd_buf] {
                state.track_inflight(dma);
            }

            tracing::debug!(
                sm,
                shader_iova = format_args!("{shader_iova:#x}"),
                qmd_iova = format_args!("{qmd_iova:#x}"),
                grid = format_args!("[{},{},{}]", dims.x, dims.y, dims.z),
                gp_put = state.gp_put,
                "NVIDIA VFIO: QMD-based compute dispatch submitted via GPFIFO"
            );

            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = (shader, buffers, dims, info);
            Err(DriverError::Unsupported(
                "NVIDIA VFIO requires Linux".into(),
            ))
        }
    }

    fn sync(&mut self) -> DriverResult<()> {
        #[cfg(target_os = "linux")]
        {
            use super::super::generation::CompletionStrategy;
            use crate::vfio::channel::registers::{pbdma, ramuserd};

            if let Some(state) = self.vfio_state.as_mut() {
                let target = state.gp_put;

                // Pre-sync PBDMA diagnostics (reading direct PBDMA registers)
                if let Some(pb) = state.target_pbdma_base {
                    let gp_base = state.bar0.read_u32(pb + pbdma::GP_BASE_LO).unwrap_or(0);
                    let hw_put = state.bar0.read_u32(pb + pbdma::GP_PUT).unwrap_or(0xDEAD);
                    let hw_get = state.bar0.read_u32(pb + pbdma::GP_FETCH).unwrap_or(0xDEAD);
                    let hw_state = state.bar0.read_u32(pb + pbdma::GP_STATE).unwrap_or(0xDEAD);
                    let ch_state = state
                        .bar0
                        .read_u32(pb + pbdma::CHANNEL_STATE)
                        .unwrap_or(0xDEAD);
                    let userd_lo = state.bar0.read_u32(pb + pbdma::USERD_LO).unwrap_or(0xDEAD);
                    let sig = state.bar0.read_u32(pb + pbdma::SIGNATURE).unwrap_or(0xDEAD);
                    tracing::info!(
                        target_gp_put = target,
                        gp_base = format_args!("{gp_base:#010x}"),
                        hw_put = format_args!("{hw_put:#010x}"),
                        hw_get = format_args!("{hw_get:#010x}"),
                        gp_state = format_args!("{hw_state:#010x}"),
                        ch_state = format_args!("{ch_state:#010x}"),
                        userd_lo = format_args!("{userd_lo:#010x}"),
                        signature = format_args!("{sig:#010x}"),
                        "pre-sync PBDMA diagnostics"
                    );
                }

                match state.completion {
                    CompletionStrategy::GpGetPoll => {
                        let mut last_gp_get = 0xFFFF_FFFFu32;
                        for i in 0..1000 {
                            let gp_get = state.userd.volatile_read_u32(ramuserd::GP_GET);
                            if gp_get == target {
                                tracing::info!(
                                    iterations = i,
                                    "sync complete: GP_GET reached GP_PUT"
                                );
                                break;
                            }
                            if gp_get != last_gp_get {
                                tracing::debug!(gp_get, target, iteration = i, "GP_GET changed");
                                last_gp_get = gp_get;
                            }
                            std::thread::sleep(std::time::Duration::from_millis(1));
                        }
                    }

                    CompletionStrategy::SemaphoreFence => {
                        let expected = state.semaphore_value;
                        if let Some(sem) = &state.semaphore {
                            for i in 0..2000 {
                                let val = sem.volatile_read_u32(0);
                                if val == expected {
                                    tracing::info!(
                                        iterations = i,
                                        payload = expected,
                                        "sync complete: semaphore reached expected value"
                                    );
                                    break;
                                }
                                if i % 100 == 0 {
                                    tracing::debug!(
                                        val,
                                        expected,
                                        iteration = i,
                                        "semaphore poll in progress"
                                    );
                                }
                                std::thread::sleep(std::time::Duration::from_millis(1));
                            }
                        } else {
                            tracing::warn!(
                                "SemaphoreFence strategy but no semaphore buffer allocated"
                            );
                        }
                    }
                }

                // Post-sync PBDMA diagnostics
                if let Some(pb) = state.target_pbdma_base {
                    let hw_put = state.bar0.read_u32(pb + pbdma::GP_PUT).unwrap_or(0xDEAD);
                    let hw_get = state.bar0.read_u32(pb + pbdma::GP_FETCH).unwrap_or(0xDEAD);
                    let userd_gp_get = state.userd.volatile_read_u32(ramuserd::GP_GET);
                    let userd_gp_put = state.userd.volatile_read_u32(ramuserd::GP_PUT);
                    tracing::info!(
                        hw_put = format_args!("{hw_put:#010x}"),
                        hw_get = format_args!("{hw_get:#010x}"),
                        userd_gp_get,
                        userd_gp_put,
                        target,
                        "post-sync PBDMA diagnostics"
                    );
                }

                clear_inflight(state);
            }
            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        Ok(())
    }

    fn capabilities(&self) -> &HardwareCapabilities {
        &self.caps
    }

    fn init_gr_context(&mut self, method_entries: &[(u32, u32)]) -> DriverResult<()> {
        #[cfg(target_os = "linux")]
        {
            let state = self.vfio_state.as_mut().ok_or_else(|| {
                DriverError::Unsupported("VFIO not opened — call open_vfio() first".into())
            })?;

            if method_entries.is_empty() {
                tracing::debug!(bdf = %self.bdf, "GR context init: no method entries to submit");
                return Ok(());
            }

            let profile = super::super::generation::profile_for_sm(self.sm);
            let pb = super::super::pushbuf::PushBuf::gr_context_init(
                profile.compute_class,
                method_entries,
            );
            state.submit_pushbuffer(pb.as_bytes())?;

            tracing::info!(
                bdf = %self.bdf,
                entries = method_entries.len(),
                compute_class = format_args!("{:#06x}", profile.compute_class),
                "GR context init pushbuffer submitted"
            );

            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = method_entries;
            Err(DriverError::Unsupported(
                "GR context init requires Linux VFIO".into(),
            ))
        }
    }

    #[cfg(target_os = "linux")]
    fn bar0(&self) -> Option<&crate::vfio::device::MappedBar> {
        self.vfio_state.as_ref().map(|s| &s.bar0)
    }

    #[cfg(target_os = "linux")]
    fn dma_backend(&self) -> Option<&crate::vfio::device::DmaBackend> {
        self.vfio_state.as_ref().map(|s| &s.dma_backend)
    }

    #[cfg(target_os = "linux")]
    fn dup_anchor_fds(&self) -> Option<crate::vfio::DupAnchorFds> {
        self.vfio_state
            .as_ref()
            .and_then(|s| s.device.dup_anchor_fds().ok())
    }

    #[cfg(target_os = "linux")]
    fn adopt_anchor_fds(&mut self, fds: crate::vfio::ReceivedVfioFds) -> DriverResult<()> {
        self.open_vfio_from_received(fds)
    }
}

#[cfg(target_os = "linux")]
impl crate::VfioDeviceExt for NvVfioComputeDevice {
    fn vfio_bar0(&self) -> Option<&crate::vfio::device::MappedBar> {
        self.vfio_state.as_ref().map(|s| &s.bar0)
    }

    fn vfio_dma_backend(&self) -> Option<&crate::vfio::device::DmaBackend> {
        self.vfio_state.as_ref().map(|s| &s.dma_backend)
    }
}
