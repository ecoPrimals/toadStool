// SPDX-License-Identifier: AGPL-3.0-or-later
//! NVIDIA VFIO compute device — sovereign GPU dispatch via BAR0/PBDMA.
//!
//! Implements [`ComputeDevice`] for NVIDIA GPUs bound to `vfio-pci`. This is
//! the direct-dispatch path: toadStool owns the GPU fd, programs PBDMA channels
//! via BAR0 MMIO, and reads back results without any kernel driver intermediary.
//!
//! # FECS gate
//!
//! The full dispatch path (alloc → upload → dispatch → sync → readback)
//! requires a running FECS (Falcon Engine Compute Scheduler):
//!
//! - **Warm path** (nouveau/nvidia-470 preserves FECS state): dispatch works
//! - **Cold path** (FECS needs firmware upload): requires real `GspBridge`
//! - **Stub path** (`StubGspBridge`): FECS boot returns `Unsupported`
//!
//! # PBDMA dispatch
//!
//! After [`open_vfio`](NvVfioComputeDevice::open_vfio), the device holds a live
//! PFIFO channel with GPFIFO ring and USERD page. `alloc`/`upload`/`readback`
//! map through [`DmaBuffer`](crate::vfio::dma::DmaBuffer), and `dispatch`
//! submits a pushbuffer via GPFIFO + doorbell.

use std::borrow::Cow;
use std::collections::HashMap;

use crate::error::{DriverError, DriverResult};
use crate::{BufferHandle, ComputeDevice, DispatchDims, HardwareCapabilities, MemoryDomain, ShaderInfo};

/// GPFIFO ring IOVA — beyond channel infrastructure (0x3000–0xBFFF).
const GPFIFO_IOVA: u64 = 0x1_0000;
/// USERD page IOVA.
const USERD_IOVA: u64 = 0x1_1000;
/// First IOVA available for user DMA buffers.
const USER_BUFFER_BASE_IOVA: u64 = 0x2_0000;
/// GPFIFO entry count (4 KiB / 8 bytes per entry = 512).
const GPFIFO_ENTRIES: u32 = 512;
/// Maximum IOVA for identity-mapped region (PT0 maps 512 × 4 KiB = 2 MiB).
const IOVA_LIMIT: u64 = 0x20_0000;
/// Page size for IOVA alignment.
const PAGE_SIZE: u64 = 4096;

/// Live VFIO state for PBDMA dispatch. Populated by [`NvVfioComputeDevice::open_vfio`].
#[cfg(target_os = "linux")]
struct VfioDispatchState {
    #[expect(dead_code, reason = "VfioDevice held for fd lifetime — dropping closes VFIO")]
    device: crate::vfio::VfioDevice,
    bar0: crate::vfio::device::MappedBar,
    channel: crate::vfio::channel::VfioChannel,
    dma_backend: crate::vfio::device::DmaBackend,
    gpfifo: crate::vfio::dma::DmaBuffer,
    userd: crate::vfio::dma::DmaBuffer,
    buffers: HashMap<u32, crate::vfio::dma::DmaBuffer>,
    inflight: Vec<BufferHandle>,
    next_handle: u32,
    next_iova: u64,
    gp_put: u32,
}

/// NVIDIA GPU compute device via VFIO direct dispatch.
///
/// Created from a PCI BDF. Capabilities are initially `UNKNOWN` until
/// BAR0 is probed for BOOT0 → SM version → generation profile.
pub struct NvVfioComputeDevice {
    bdf: String,
    caps: HardwareCapabilities,
    sm: u32,
    fecs_ready: bool,
    #[cfg(target_os = "linux")]
    vfio_state: Option<VfioDispatchState>,
}

impl NvVfioComputeDevice {
    /// Create a new NVIDIA VFIO compute device for the given BDF.
    ///
    /// Initializes with `HardwareCapabilities::UNKNOWN`. Call
    /// [`probe_capabilities`](Self::probe_capabilities) after BAR0 open
    /// to populate real caps from the BOOT0 register.
    #[must_use]
    pub fn new(bdf: String) -> Self {
        Self {
            bdf,
            caps: HardwareCapabilities::UNKNOWN,
            sm: 0,
            fecs_ready: false,
            #[cfg(target_os = "linux")]
            vfio_state: None,
        }
    }

    /// Create a device with known SM version (from prior BAR0 probe or
    /// warm handoff detection).
    #[must_use]
    pub fn with_sm(bdf: String, sm: u32) -> Self {
        let profile = super::generation::profile_for_sm(sm);
        Self {
            bdf,
            caps: profile.to_capabilities(),
            sm,
            fecs_ready: false,
            #[cfg(target_os = "linux")]
            vfio_state: None,
        }
    }

    /// Probe capabilities from BOOT0 register if BAR0 is accessible.
    ///
    /// On success, updates the internal capabilities from the GPU's
    /// generation profile. Requires sysfs BAR0 access (VFIO feature).
    #[cfg(feature = "vfio")]
    pub fn probe_capabilities(&mut self) -> DriverResult<()> {
        const BAR0_MIN_SIZE: usize = 0x1000;
        let bar0 = crate::vfio::sysfs_bar0::SysfsBar0::open(&self.bdf, BAR0_MIN_SIZE)
            .map_err(|e| DriverError::Unsupported(format!("BAR0 open failed: {e}").into()))?;
        let boot0 = bar0.read_u32(0);
        if let Some(sm) = super::identity::boot0_to_sm(boot0) {
            let profile = super::generation::profile_for_sm(sm);
            self.caps = profile.to_capabilities();
            self.sm = sm;
            tracing::info!(
                bdf = %self.bdf, sm, chip = super::identity::chip_name(sm),
                "NVIDIA VFIO: probed capabilities from BOOT0"
            );
        }
        Ok(())
    }

    /// Probe BAR0 for warm-preserved FECS state.
    ///
    /// After a nouveau → vfio-pci warm handoff, FECS may be halted with
    /// firmware still resident in IMEM/DMEM. This reads FECS CPUCTL and
    /// MAILBOX0 to detect the warm-preserved state:
    ///
    /// - **HALTED (bit 5) + MAILBOX0 ≠ 0** → warm-preserved, compute-ready
    /// - Otherwise → cold or inconsistent, FECS not ready
    ///
    /// Also probes BOOT0 for chip identification if capabilities are unknown.
    /// Returns `true` if warm FECS was detected and the device is compute-ready.
    #[cfg(target_os = "linux")]
    pub fn probe_warm_fecs(&mut self) -> bool {
        use crate::vfio::channel::registers::falcon;

        const BAR0_MIN_SIZE: usize = 0x41_A000;
        let bar0 = match crate::vfio::sysfs_bar0::SysfsBar0::open(&self.bdf, BAR0_MIN_SIZE) {
            Ok(b) => b,
            Err(e) => {
                tracing::debug!(bdf = %self.bdf, error = %e, "BAR0 open failed for warm FECS probe");
                return false;
            }
        };

        // Probe BOOT0 for chip identity if not already known.
        if self.caps.vendor == crate::hardware::Vendor::Unknown {
            let boot0 = bar0.read_u32(0);
            if let Some(sm) = super::identity::boot0_to_sm(boot0) {
                let profile = super::generation::profile_for_sm(sm);
                self.caps = profile.to_capabilities();
                self.sm = sm;
                tracing::info!(
                    bdf = %self.bdf, sm,
                    chip = super::identity::chip_name(sm),
                    "warm probe: identified NVIDIA GPU from BOOT0"
                );
            }
        }

        // Check PMC_ENABLE popcount — warm GPUs have ≥8 engines enabled.
        let pmc_enable = bar0.read_u32(0x200);
        if pmc_enable.count_ones() < 8 {
            tracing::debug!(
                bdf = %self.bdf,
                pmc_enable = format!("{pmc_enable:#010x}"),
                popcount = pmc_enable.count_ones(),
                "cold GPU: PMC_ENABLE popcount < 8"
            );
            return false;
        }

        let fecs_cpuctl = bar0.read_u32(falcon::FECS_BASE + falcon::CPUCTL);
        let fecs_mb0 = bar0.read_u32(falcon::FECS_BASE + falcon::MAILBOX0);

        let halted = fecs_cpuctl & falcon::CPUCTL_HALTED != 0;

        tracing::info!(
            bdf = %self.bdf,
            fecs_cpuctl = format!("{fecs_cpuctl:#010x}"),
            fecs_mb0 = format!("{fecs_mb0:#010x}"),
            halted,
            pmc_popcount = pmc_enable.count_ones(),
            "FECS warm-state probe"
        );

        if halted && fecs_mb0 != 0 {
            tracing::info!(
                bdf = %self.bdf,
                "FECS warm-preserved detected — compute context ready"
            );
            self.fecs_ready = true;
            return true;
        }

        tracing::debug!(
            bdf = %self.bdf,
            "FECS not warm-preserved (halted={halted}, mb0={fecs_mb0:#x})"
        );
        false
    }

    /// Mark FECS as ready (warm-preserved or firmware booted).
    pub fn set_fecs_ready(&mut self, ready: bool) {
        self.fecs_ready = ready;
    }

    /// BDF address of this device.
    #[must_use]
    pub fn bdf(&self) -> &str {
        &self.bdf
    }

    /// Whether FECS compute context is available for dispatch.
    #[must_use]
    pub fn is_fecs_ready(&self) -> bool {
        self.fecs_ready
    }

    /// Open the VFIO device and create a PFIFO channel for PBDMA dispatch.
    ///
    /// After this call, `alloc`/`upload`/`readback`/`dispatch`/`sync` use
    /// real DMA buffers and GPFIFO submission instead of returning
    /// `Unsupported`.
    ///
    /// Uses warm handoff channel creation if FECS is already ready
    /// (preserves falcon engine state from nouveau/nvidia-470).
    ///
    /// # Errors
    ///
    /// Returns error if VFIO device open, BAR0 map, DMA buffer allocation,
    /// or channel creation fails.
    #[cfg(target_os = "linux")]
    pub fn open_vfio(&mut self) -> DriverResult<()> {
        use crate::vfio::channel::VfioChannel;
        use crate::vfio::dma::DmaBuffer;
        use crate::vfio::VfioDevice;

        let device = VfioDevice::open(&self.bdf)?;
        let bar0 = device.map_bar(0)?;
        let dma_backend = device.dma_backend();

        let gpfifo = DmaBuffer::new(dma_backend.clone(), 4096, GPFIFO_IOVA)?;
        let userd = DmaBuffer::new(dma_backend.clone(), 4096, USERD_IOVA)?;

        let channel = if self.fecs_ready {
            VfioChannel::create_warm(
                dma_backend.clone(),
                &bar0,
                GPFIFO_IOVA,
                GPFIFO_ENTRIES,
                USERD_IOVA,
                0,
            )?
        } else {
            VfioChannel::create(
                dma_backend.clone(),
                &bar0,
                GPFIFO_IOVA,
                GPFIFO_ENTRIES,
                USERD_IOVA,
                0,
            )?
        };

        tracing::info!(
            bdf = %self.bdf,
            channel_id = channel.id(),
            fecs_ready = self.fecs_ready,
            "VFIO PBDMA dispatch state initialized"
        );

        self.vfio_state = Some(VfioDispatchState {
            device,
            bar0,
            channel,
            dma_backend,
            gpfifo,
            userd,
            buffers: HashMap::new(),
            inflight: Vec::new(),
            next_handle: 1,
            next_iova: USER_BUFFER_BASE_IOVA,
            gp_put: 0,
        });

        Ok(())
    }

    /// Whether the VFIO dispatch path is initialized.
    #[must_use]
    pub fn is_vfio_open(&self) -> bool {
        #[cfg(target_os = "linux")]
        {
            self.vfio_state.is_some()
        }
        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    }
}

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
                DriverError::Unsupported(
                    "VFIO not opened — call open_vfio() before alloc".into(),
                )
            })?;

            let aligned_size = (size as usize).div_ceil(PAGE_SIZE as usize) * PAGE_SIZE as usize;
            let iova = state.next_iova;
            if iova + aligned_size as u64 > IOVA_LIMIT {
                return Err(DriverError::MmapFailed(Cow::Borrowed(
                    "IOVA space exhausted (2 MiB identity map limit)",
                )));
            }

            let buf = crate::vfio::dma::DmaBuffer::new(
                state.dma_backend.clone(),
                aligned_size,
                iova,
            )?;

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
            let state = self.vfio_state.as_mut().ok_or_else(|| {
                DriverError::Unsupported("VFIO not opened".into())
            })?;
            state
                .buffers
                .remove(&handle.0)
                .ok_or(DriverError::BufferNotFound(handle))?;
            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = handle;
            Err(DriverError::Unsupported("NVIDIA VFIO requires Linux".into()))
        }
    }

    fn upload(&mut self, handle: BufferHandle, offset: u64, data: &[u8]) -> DriverResult<()> {
        #[cfg(target_os = "linux")]
        {
            let state = self.vfio_state.as_mut().ok_or_else(|| {
                DriverError::Unsupported("VFIO not opened".into())
            })?;
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
            Err(DriverError::Unsupported("NVIDIA VFIO requires Linux".into()))
        }
    }

    fn readback(&self, handle: BufferHandle, offset: u64, len: usize) -> DriverResult<Vec<u8>> {
        #[cfg(target_os = "linux")]
        {
            let state = self.vfio_state.as_ref().ok_or_else(|| {
                DriverError::Unsupported("VFIO not opened".into())
            })?;
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
            Err(DriverError::Unsupported("NVIDIA VFIO requires Linux".into()))
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
                 nouveau/nvidia-470, or real GspBridge (coralReef IPC or local absorption)"
                    .into(),
            ));
        }

        #[cfg(target_os = "linux")]
        {
            use crate::vfio::channel::registers::{ramuserd, usermode};

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
            let profile = super::generation::profile_for_sm(sm);

            // --- 1. Upload shader binary to GPU-visible DMA buffer ---
            let shader_aligned =
                shader.len().div_ceil(PAGE_SIZE as usize) * PAGE_SIZE as usize;
            let shader_iova = state.next_iova;
            if shader_iova + shader_aligned as u64 > IOVA_LIMIT {
                return Err(DriverError::MmapFailed(Cow::Borrowed(
                    "IOVA space exhausted for shader binary",
                )));
            }
            let mut shader_buf = crate::vfio::dma::DmaBuffer::new(
                state.dma_backend.clone(),
                shader_aligned,
                shader_iova,
            )?;
            shader_buf.as_mut_slice()[..shader.len()].copy_from_slice(shader);
            state.next_iova = shader_iova + shader_aligned as u64;

            // --- 2. Build CBUF descriptor table for user buffers ---
            let desc_entry_size: u64 = 16;
            let desc_size = (buffers.len() as u64) * desc_entry_size;
            let desc_aligned = (desc_size as usize)
                .max(PAGE_SIZE as usize)
                .div_ceil(PAGE_SIZE as usize)
                * PAGE_SIZE as usize;
            let desc_iova = state.next_iova;
            if desc_iova + desc_aligned as u64 > IOVA_LIMIT {
                return Err(DriverError::MmapFailed(Cow::Borrowed(
                    "IOVA space exhausted for descriptor table",
                )));
            }
            let mut desc_buf = crate::vfio::dma::DmaBuffer::new(
                state.dma_backend.clone(),
                desc_aligned,
                desc_iova,
            )?;

            for (i, handle) in buffers.iter().enumerate() {
                let buf = state.buffers.get(&handle.0).ok_or(
                    DriverError::BufferNotFound(*handle),
                )?;
                let buf_iova = buf.iova();
                let buf_size = buf.size() as u32;
                let offset = i * desc_entry_size as usize;
                desc_buf.as_mut_slice()[offset..offset + 4]
                    .copy_from_slice(&(buf_iova as u32).to_le_bytes());
                desc_buf.as_mut_slice()[offset + 4..offset + 8]
                    .copy_from_slice(&((buf_iova >> 32) as u32).to_le_bytes());
                desc_buf.as_mut_slice()[offset + 8..offset + 12]
                    .copy_from_slice(&buf_size.to_le_bytes());
            }
            state.next_iova = desc_iova + desc_aligned as u64;

            // --- 3. Build driver constants (grid dims for `num_workgroups`) ---
            let driver_const_data =
                super::qmd::encode_driver_constants(&dims);
            let dc_iova = state.next_iova;
            if dc_iova + PAGE_SIZE > IOVA_LIMIT {
                return Err(DriverError::MmapFailed(Cow::Borrowed(
                    "IOVA space exhausted for driver constants",
                )));
            }
            let mut dc_buf = crate::vfio::dma::DmaBuffer::new(
                state.dma_backend.clone(),
                PAGE_SIZE as usize,
                dc_iova,
            )?;
            dc_buf.as_mut_slice()[..driver_const_data.len()]
                .copy_from_slice(&driver_const_data);
            state.next_iova = dc_iova + PAGE_SIZE;

            // --- 4. Build QMD ---
            let workgroup = if info.workgroup.iter().any(|&d| d > 0) {
                info.workgroup
            } else {
                [64, 1, 1]
            };

            let cbufs = super::qmd::build_standard_cbufs(
                desc_iova,
                desc_size.max(64) as u32,
                dc_iova,
                super::qmd::DRIVER_CONST_SIZE,
            );

            let qmd_params = super::qmd::QmdParams {
                shader_va: shader_iova,
                grid: dims,
                workgroup,
                gpr_count: info.gpr_count.max(4),
                shared_mem_bytes: info.shared_mem_bytes,
                barrier_count: info.barrier_count,
                local_mem_low_bytes: info.local_mem_bytes.unwrap_or(0),
                cbufs,
            };

            let qmd_words = super::qmd::build_qmd(profile, &qmd_params);
            let qmd_bytes: &[u8] = bytemuck::cast_slice(&qmd_words);
            let qmd_aligned = qmd_bytes
                .len()
                .div_ceil(PAGE_SIZE as usize)
                * PAGE_SIZE as usize;
            let qmd_iova = state.next_iova;
            if qmd_iova + qmd_aligned as u64 > IOVA_LIMIT {
                return Err(DriverError::MmapFailed(Cow::Borrowed(
                    "IOVA space exhausted for QMD",
                )));
            }
            let mut qmd_buf = crate::vfio::dma::DmaBuffer::new(
                state.dma_backend.clone(),
                qmd_aligned,
                qmd_iova,
            )?;
            qmd_buf.as_mut_slice()[..qmd_bytes.len()].copy_from_slice(qmd_bytes);
            state.next_iova = qmd_iova + qmd_aligned as u64;

            // --- 5. Build pushbuffer (compute init + dispatch) ---
            let mut init_pb = super::pushbuf::PushBuf::compute_init(
                profile.compute_class,
                profile.local_mem_window,
                0,
                0,
            );
            let dispatch_pb = super::pushbuf::PushBuf::compute_dispatch_with_launch(
                profile.launch_method,
                qmd_iova,
            );
            init_pb.append(&dispatch_pb);
            let pb_bytes = init_pb.as_bytes();

            let pb_aligned = pb_bytes
                .len()
                .div_ceil(PAGE_SIZE as usize)
                * PAGE_SIZE as usize;
            let pb_iova = state.next_iova;
            if pb_iova + pb_aligned as u64 > IOVA_LIMIT {
                return Err(DriverError::MmapFailed(Cow::Borrowed(
                    "IOVA space exhausted for pushbuffer",
                )));
            }
            let mut pb_buf = crate::vfio::dma::DmaBuffer::new(
                state.dma_backend.clone(),
                pb_aligned,
                pb_iova,
            )?;
            pb_buf.as_mut_slice()[..pb_bytes.len()].copy_from_slice(pb_bytes);
            state.next_iova = pb_iova + pb_aligned as u64;

            // --- 6. Submit via GPFIFO ---
            let dword_count = (pb_bytes.len() / 4) as u64;
            let gp_entry_lo = (pb_iova & 0xFFFF_FFFC) as u32;
            let gp_entry_hi = (dword_count as u32) << 10;
            let gp_entry = (gp_entry_lo as u64) | ((gp_entry_hi as u64) << 32);

            let gp_offset = (state.gp_put as usize) * 8;
            state.gpfifo.as_mut_slice()[gp_offset..gp_offset + 8]
                .copy_from_slice(&gp_entry.to_le_bytes());

            let new_put = (state.gp_put + 1) % GPFIFO_ENTRIES;

            state.userd.volatile_write_u32(ramuserd::GP_PUT, new_put);
            std::sync::atomic::fence(std::sync::atomic::Ordering::SeqCst);

            state
                .bar0
                .write_u32(usermode::NOTIFY_CHANNEL_PENDING, state.channel.id())
                .map_err(|e| {
                    DriverError::SubmitFailed(Cow::Owned(format!("doorbell write: {e}")))
                })?;

            state.gp_put = new_put;

            // Track transient DMA allocations for cleanup after sync.
            for dma in [shader_buf, desc_buf, dc_buf, qmd_buf, pb_buf] {
                let id = state.next_handle;
                state.next_handle += 1;
                state.buffers.insert(id, dma);
                state.inflight.push(BufferHandle(id));
            }

            tracing::debug!(
                sm,
                shader_iova = format_args!("{shader_iova:#x}"),
                qmd_iova = format_args!("{qmd_iova:#x}"),
                pb_iova = format_args!("{pb_iova:#x}"),
                dwords = dword_count,
                grid = format_args!("[{},{},{}]", dims.x, dims.y, dims.z),
                gp_put = new_put,
                "NVIDIA VFIO: QMD-based compute dispatch submitted via GPFIFO"
            );

            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = (shader, buffers, dims, info);
            Err(DriverError::Unsupported("NVIDIA VFIO requires Linux".into()))
        }
    }

    fn sync(&mut self) -> DriverResult<()> {
        #[cfg(target_os = "linux")]
        {
            use crate::vfio::channel::registers::ramuserd;

            if let Some(state) = self.vfio_state.as_mut() {
                // Poll USERD GP_GET until it reaches GP_PUT.
                let target = state.gp_put;
                for _ in 0..1000 {
                    let gp_get = state.userd.volatile_read_u32(ramuserd::GP_GET);
                    if gp_get == target {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }

                // Free inflight pushbuffers.
                let inflight = std::mem::take(&mut state.inflight);
                for handle in inflight {
                    state.buffers.remove(&handle.0);
                }
            }
            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        Ok(())
    }

    fn capabilities(&self) -> &HardwareCapabilities {
        &self.caps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cold_dispatch_returns_fecs_error() {
        let mut dev = NvVfioComputeDevice::new("0000:01:00.0".into());
        let result = dev.dispatch(
            &[0u8; 64],
            &[],
            DispatchDims::new(1, 1, 1),
            &ShaderInfo::default(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("FECS"));
    }

    #[test]
    fn cold_alloc_returns_unsupported() {
        let mut dev = NvVfioComputeDevice::new("0000:01:00.0".into());
        assert!(dev.alloc(4096, MemoryDomain::Vram).is_err());
    }

    #[test]
    fn with_sm_populates_caps() {
        let dev = NvVfioComputeDevice::with_sm("0000:25:00.0".into(), 70);
        let caps = dev.capabilities();
        assert_eq!(caps.vendor, crate::hardware::Vendor::Nvidia);
        assert_ne!(caps.device_name, "unknown");
    }

    #[test]
    fn new_has_unknown_caps() {
        let dev = NvVfioComputeDevice::new("0000:01:00.0".into());
        assert_eq!(dev.capabilities().vendor, crate::hardware::Vendor::Unknown);
    }

    #[test]
    fn fecs_ready_flag() {
        let mut dev = NvVfioComputeDevice::new("0000:01:00.0".into());
        assert!(!dev.is_fecs_ready());
        dev.set_fecs_ready(true);
        assert!(dev.is_fecs_ready());
    }

    #[test]
    fn warm_fecs_enables_alloc_gate() {
        let mut dev = NvVfioComputeDevice::new("0000:01:00.0".into());
        assert!(dev.alloc(4096, MemoryDomain::Vram).is_err());
        let err = dev.alloc(4096, MemoryDomain::Vram).unwrap_err();
        assert!(err.to_string().contains("FECS"));

        dev.set_fecs_ready(true);
        let err = dev.alloc(4096, MemoryDomain::Vram).unwrap_err();
        assert!(
            err.to_string().contains("VFIO not opened"),
            "with FECS ready but no VFIO, should hit VFIO gate: {err}"
        );
    }

    #[test]
    fn warm_fecs_enables_dispatch_gate() {
        let mut dev = NvVfioComputeDevice::with_sm("0000:01:00.0".into(), 70);
        dev.set_fecs_ready(true);
        let err = dev
            .dispatch(
                &[0u8; 64],
                &[],
                DispatchDims::new(1, 1, 1),
                &ShaderInfo::default(),
            )
            .unwrap_err();
        assert!(
            err.to_string().contains("VFIO not opened"),
            "with FECS ready but no VFIO, should hit VFIO gate: {err}"
        );
    }

    #[test]
    fn dispatch_rejects_empty_shader() {
        let mut dev = NvVfioComputeDevice::new("0000:01:00.0".into());
        dev.set_fecs_ready(true);
        let err = dev
            .dispatch(
                &[],
                &[],
                DispatchDims::new(1, 1, 1),
                &ShaderInfo::default(),
            )
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("VFIO not opened") || msg.contains("non-empty"),
            "empty shader binary should fail: {msg}"
        );
    }

    #[test]
    fn free_unknown_handle_returns_not_found() {
        let mut dev = NvVfioComputeDevice::new("0000:01:00.0".into());
        let err = dev.free(BufferHandle(999)).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("VFIO") || msg.contains("not found") || msg.contains("999"),
            "unknown handle should error: {msg}"
        );
    }

    #[test]
    fn is_vfio_open_default_false() {
        let dev = NvVfioComputeDevice::new("0000:01:00.0".into());
        assert!(!dev.is_vfio_open());
    }
}
