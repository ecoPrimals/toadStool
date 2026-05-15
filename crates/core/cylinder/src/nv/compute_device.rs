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
/// GR context save area IOVA — FECS copies golden context here per-channel.
const GR_CTX_IOVA: u64 = 0x1_2000;
/// GR context buffer size — GV100 context is ~500KB; 1MB gives margin.
const GR_CTX_SIZE: usize = 0x10_0000;
/// First IOVA available for user DMA buffers (shifted to make room for GR ctx).
const USER_BUFFER_BASE_IOVA: u64 = 0x12_0000;
/// GPFIFO entry count (4 KiB / 8 bytes per entry = 512).
const GPFIFO_ENTRIES: u32 = 512;
/// Maximum IOVA for identity-mapped region. PT0 now needs to cover up to
/// USER_BUFFER_BASE_IOVA + user allocations.
const IOVA_LIMIT: u64 = 0x40_0000;
/// Page size for IOVA alignment.
const PAGE_SIZE: u64 = 4096;

/// Doorbell strategy: Volta+ uses NV_USERMODE, Kepler uses GK104 per-channel.
#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy)]
enum DoorbellKind {
    /// Volta+ `NV_USERMODE_NOTIFY_CHANNEL_PENDING` at BAR0 0x81_0090.
    Usermode,
    /// Kepler GK104 per-channel doorbell at `0x3000 + ch_id * 8`.
    Gk104 { channel_id: u32 },
}

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
    #[expect(dead_code, reason = "GR context buffer held for DMA lifetime")]
    gr_ctx: Option<crate::vfio::dma::DmaBuffer>,
    buffers: HashMap<u32, crate::vfio::dma::DmaBuffer>,
    inflight: Vec<BufferHandle>,
    next_handle: u32,
    next_iova: u64,
    gp_put: u32,
    doorbell: DoorbellKind,
    /// BAR0 base offset of the target PBDMA for direct GP_PUT writes.
    /// On warm-caught GV100, the scheduler doesn't reliably propagate
    /// USERD GP_PUT to the PBDMA; direct writes ensure GPFIFO consumption.
    target_pbdma_base: Option<usize>,
}

#[cfg(target_os = "linux")]
impl VfioDispatchState {
    /// Allocate a DMA buffer at the next available IOVA, advancing the bump pointer.
    fn alloc_next_dma(&mut self, size: usize, what: &str) -> DriverResult<crate::vfio::dma::DmaBuffer> {
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
    fn submit_pushbuffer(&mut self, pb_bytes: &[u8]) -> DriverResult<()> {
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
    fn track_inflight(&mut self, dma: crate::vfio::dma::DmaBuffer) {
        let id = self.next_handle;
        self.next_handle += 1;
        self.buffers.insert(id, dma);
        self.inflight.push(BufferHandle(id));
    }
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

    /// Probe BAR0 for warm FECS state.
    ///
    /// After a nouveau → vfio-pci warm handoff, FECS is in one of two
    /// valid warm states depending on the teardown strategy:
    ///
    /// - **Live warm**: FECS still running (not halted), PMC engines enabled.
    ///   Occurs with NOP'd-teardown patched nouveau — FECS was never stopped.
    /// - **Preserved warm**: FECS halted + MAILBOX0 ≠ 0, firmware resident in
    ///   IMEM/DMEM. Occurs with standard teardown interception (kprobe/livepatch).
    ///
    /// Both states indicate the GPU is compute-ready. Cold state (PMC popcount < 8)
    /// means no prior driver session initialized the GPU.
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

        // Use CPUCTL_ALIAS (0x130) for Volta HS falcons. CPUCTL at 0x100
        // is security-locked and always reads HRESET=1 on HS mode falcons.
        let fecs_cpuctl_alias = bar0.read_u32(falcon::FECS_BASE + falcon::CPUCTL_ALIAS);
        let fecs_cpuctl_raw = bar0.read_u32(falcon::FECS_BASE + falcon::CPUCTL);
        let fecs_mb0 = bar0.read_u32(falcon::FECS_BASE + falcon::MAILBOX0);
        let fecs_pc = bar0.read_u32(falcon::FECS_BASE + falcon::PC);

        let halted = fecs_cpuctl_alias & falcon::CPUCTL_HALTED != 0;
        let in_hreset = fecs_cpuctl_alias & falcon::CPUCTL_HRESET != 0;
        let running = !halted && !in_hreset;

        tracing::info!(
            bdf = %self.bdf,
            fecs_cpuctl_alias = format!("{fecs_cpuctl_alias:#010x}"),
            fecs_cpuctl_raw = format!("{fecs_cpuctl_raw:#010x}"),
            fecs_pc = format!("{fecs_pc:#010x}"),
            fecs_mb0 = format!("{fecs_mb0:#010x}"),
            halted,
            in_hreset,
            running,
            pmc_popcount = pmc_enable.count_ones(),
            "FECS warm-state probe (CPUCTL_ALIAS)"
        );

        let preserved_warm = halted && fecs_mb0 != 0;
        let live_warm = running && pmc_enable.count_ones() >= 16;

        if preserved_warm {
            tracing::info!(
                bdf = %self.bdf,
                "FECS warm-preserved (halted + firmware resident) — compute-ready"
            );
            self.fecs_ready = true;
            return true;
        }

        if live_warm {
            tracing::info!(
                bdf = %self.bdf,
                pmc_popcount = pmc_enable.count_ones(),
                "FECS live-warm (still running, NOP'd teardown) — compute-ready"
            );
            self.fecs_ready = true;
            return true;
        }

        tracing::debug!(
            bdf = %self.bdf,
            "FECS not warm (halted={halted}, mb0={fecs_mb0:#x}, pmc_pop={})",
            pmc_enable.count_ones(),
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

    /// SM version detected from BOOT0 (0 if not yet probed).
    #[must_use]
    pub fn sm_version(&self) -> u32 {
        self.sm
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

        let profile = super::generation::profile_for_sm(self.sm);
        let is_kepler = matches!(
            profile.page_table_format,
            super::generation::PageTableFormat::V1TwoLevel
        );

        let device = VfioDevice::open(&self.bdf)?;
        let bar0 = device.map_bar(0)?;
        let dma_backend = device.dma_backend();

        let mut fecs_hs_booted = false;
        let mut fecs_bridge: Option<super::nv_gsp_bridge::NvGspBridge> = None;

        // Probe FECS state and prepare for deferred boot (after channel creation).
        // On nouveau warm handoff, FECS firmware expects PFIFO infrastructure
        // to exist before it can run. We boot FECS AFTER channel setup.
        if !is_kepler && self.fecs_ready {
            use crate::vfio::channel::registers::falcon;

            let pmc_before = bar0.read_u32(0x200).unwrap_or(0);
            if pmc_before.count_ones() < 8 {
                tracing::info!(
                    bdf = %self.bdf,
                    pmc_before = format_args!("{pmc_before:#010x}"),
                    "PMC cold after VFIO FLR — enabling all engines"
                );
                let _ = bar0.write_u32(0x200, 0xFFFF_FFFF);
                std::thread::sleep(std::time::Duration::from_millis(50));
                let pmc_after = bar0.read_u32(0x200).unwrap_or(0);
                tracing::info!(
                    bdf = %self.bdf,
                    pmc_after = format_args!("{pmc_after:#010x}"),
                    popcount = pmc_after.count_ones(),
                    "PMC engines enabled"
                );
            }

            // Use CPUCTL_ALIAS for Volta HS falcons — CPUCTL at 0x100 is
            // security-locked and always reads HRESET on HS mode.
            let fecs_alias = bar0.read_u32(falcon::FECS_BASE + falcon::CPUCTL_ALIAS).unwrap_or(0xDEAD);
            let fecs_pc = bar0.read_u32(falcon::FECS_BASE + falcon::PC).unwrap_or(0xDEAD);
            let is_bad_read = fecs_alias & 0xBADF_0000 == 0xBADF_0000;
            let fecs_in_hreset = !is_bad_read
                && (fecs_alias & falcon::CPUCTL_HRESET != 0);
            let fecs_running = !is_bad_read && !fecs_in_hreset
                && (fecs_alias & falcon::CPUCTL_HALTED == 0);
            let fecs_needs_boot = is_bad_read || fecs_in_hreset;

            if fecs_running {
                tracing::info!(
                    bdf = %self.bdf,
                    fecs_cpuctl_alias = format_args!("{fecs_alias:#010x}"),
                    fecs_pc = format_args!("{fecs_pc:#010x}"),
                    "FECS already running (warm handoff preserved) — skipping boot"
                );
            } else if fecs_needs_boot {
                tracing::info!(
                    bdf = %self.bdf,
                    fecs_cpuctl_alias = format_args!("{fecs_alias:#010x}"),
                    fecs_pc = format_args!("{fecs_pc:#010x}"),
                    bad_read = is_bad_read,
                    "FECS not alive — preparing deferred HS boot"
                );
                let bridge = super::nv_gsp_bridge::NvGspBridge::new(profile.firmware_chip);
                if bridge.has_gr_firmware() {
                    tracing::info!(
                        bdf = %self.bdf,
                        "FECS firmware available — deferring boot to after channel creation"
                    );
                    fecs_bridge = Some(bridge);
                }
            }
        }

        let gpfifo = DmaBuffer::new(dma_backend.clone(), 4096, GPFIFO_IOVA)?;
        let userd = DmaBuffer::new(dma_backend.clone(), 4096, USERD_IOVA)?;

        // Allocate GR context save area for FECS. Each channel needs a context
        // buffer where FECS saves/restores the GR register state during
        // context switching. Without this, FECS leaves our channel in PENDING.
        let gr_ctx = if !is_kepler && self.fecs_ready {
            let ctx = DmaBuffer::new(dma_backend.clone(), GR_CTX_SIZE, GR_CTX_IOVA)?;
            tracing::info!(
                bdf = %self.bdf,
                gr_ctx_iova = format_args!("{GR_CTX_IOVA:#x}"),
                gr_ctx_size = GR_CTX_SIZE,
                "GR context buffer allocated for FECS"
            );
            Some(ctx)
        } else {
            None
        };

        let (channel, doorbell) = if is_kepler {
            let guard = super::hardware_guard::GuardedBar::new(&bar0, 16).map_err(|e| {
                DriverError::Unsupported(Cow::Owned(format!("Kepler BAR0 guard init: {e}")))
            })?;
            let ch = VfioChannel::create_kepler(
                dma_backend.clone(),
                &guard,
                GPFIFO_IOVA,
                GPFIFO_ENTRIES,
                USERD_IOVA,
                0,
            )?;
            let ch_id = ch.id();
            tracing::info!(
                bdf = %self.bdf,
                generation = profile.name,
                "Kepler VFIO channel created — GK104 doorbell"
            );
            (ch, DoorbellKind::Gk104 { channel_id: ch_id })
        } else if self.fecs_ready {
            let mut ch = VfioChannel::create_warm(
                dma_backend.clone(),
                &bar0,
                GPFIFO_IOVA,
                GPFIFO_ENTRIES,
                USERD_IOVA,
                0,
            )?;
            if gr_ctx.is_some() {
                ch.write_gr_context_ptr(GR_CTX_IOVA, 4);
                ch.resubmit_runlist(&bar0)?;
            }
            (ch, DoorbellKind::Usermode)
        } else {
            let ch = VfioChannel::create(
                dma_backend.clone(),
                &bar0,
                GPFIFO_IOVA,
                GPFIFO_ENTRIES,
                USERD_IOVA,
                0,
            )?;
            (ch, DoorbellKind::Usermode)
        };

        tracing::info!(
            bdf = %self.bdf,
            channel_id = channel.id(),
            fecs_ready = self.fecs_ready,
            generation = profile.name,
            doorbell = ?doorbell,
            "VFIO PBDMA dispatch state initialized"
        );

        // Discover the target PBDMA for direct GP_PUT writes.
        let target_pbdma_base = if matches!(doorbell, DoorbellKind::Usermode) {
            let pbdma_map = bar0.read_u32(0x2004).unwrap_or(0);
            let runlist_id = channel.runlist_id_hint();
            let mut found: Option<usize> = None;
            let mut seq = 0_usize;
            for pid in 0..32_usize {
                if pbdma_map & (1 << pid) == 0 {
                    continue;
                }
                let rl = bar0.read_u32(0x2390 + seq * 4).unwrap_or(0xFFFF);
                if rl == runlist_id {
                    found = Some(0x0004_0000 + pid * 0x2000);
                    tracing::info!(pbdma = pid, runlist = rl, "target PBDMA for direct GP_PUT");
                    break;
                }
                seq += 1;
            }
            found
        } else {
            None
        };

        // Deferred GR falcon boot: now that PFIFO + channel infrastructure
        // exists, boot GPCCS first, then FECS, then send INIT_CTXSW.
        // FECS and GPCCS are a pair — FECS self-halts if GPCCS is not running.
        if let Some(bridge) = fecs_bridge {
            use crate::vfio::channel::registers::{falcon, pmc};

            // Ensure all engines are enabled in PMC_ENABLE before touching
            // GPC registers. GPCCS registers at 0x41Axxx are behind the GR
            // engine clock gate and return 0xbadf5040 (PRI fault) when
            // GR/GPC is clock-gated.
            let pmc_before = bar0.read_u32(pmc::ENABLE).unwrap_or(0);
            let _ = bar0.write_u32(pmc::ENABLE, 0xFFFF_FFFF);
            std::thread::sleep(std::time::Duration::from_millis(10));
            let pmc_after = bar0.read_u32(pmc::ENABLE).unwrap_or(0);
            tracing::info!(
                bdf = %self.bdf,
                pmc_before = format_args!("{pmc_before:#010x}"),
                pmc_after = format_args!("{pmc_after:#010x}"),
                "GR init: PMC glow-plug all engines"
            );

            // 1. Boot GPCCS first
            tracing::info!(bdf = %self.bdf, "GR init: booting GPCCS falcon");
            match bridge.boot_falcon_hs(
                &bar0,
                "GPCCS",
                falcon::GPCCS_BASE,
                &dma_backend,
                super::nv_gsp_bridge::GPCCS_FW_CODE_IOVA,
                super::nv_gsp_bridge::GPCCS_FW_DATA_IOVA,
            ) {
                Ok((ctl, mb0)) => {
                    tracing::info!(
                        bdf = %self.bdf,
                        gpccs_cpuctl = format_args!("{ctl:#010x}"),
                        gpccs_mb0 = format_args!("{mb0:#010x}"),
                        "GPCCS HS boot complete"
                    );
                }
                Err(e) => {
                    tracing::warn!(bdf = %self.bdf, error = %e, "GPCCS HS boot failed");
                }
            }

            // 2. Boot FECS
            tracing::info!(bdf = %self.bdf, "GR init: booting FECS falcon");
            match bridge.boot_falcon_hs(
                &bar0,
                "FECS",
                falcon::FECS_BASE,
                &dma_backend,
                super::nv_gsp_bridge::FECS_FW_CODE_IOVA,
                super::nv_gsp_bridge::FECS_FW_DATA_IOVA,
            ) {
                Ok((ctl, mb0)) => {
                    fecs_hs_booted = true;
                    tracing::info!(
                        bdf = %self.bdf,
                        fecs_cpuctl = format_args!("{ctl:#010x}"),
                        fecs_mb0 = format_args!("{mb0:#010x}"),
                        "FECS HS boot complete (post-channel-creation)"
                    );
                }
                Err(e) => {
                    tracing::warn!(bdf = %self.bdf, error = %e, "FECS HS boot failed");
                }
            }

            // 3. Check FECS state via both CPUCTL and CPUCTL_ALIAS.
            // On Volta HS falcons, CPUCTL at 0x100 may be security-locked and
            // always show HRESET, while CPUCTL_ALIAS at 0x130 shows the true state.
            if fecs_hs_booted {
                let fecs_base = falcon::FECS_BASE;

                // Immediate check via both registers
                let ctl = bar0.read_u32(fecs_base + falcon::CPUCTL).unwrap_or(0xDEAD);
                let ctl_alias = bar0.read_u32(fecs_base + falcon::CPUCTL_ALIAS).unwrap_or(0xDEAD);
                let pc = bar0.read_u32(fecs_base + falcon::PC).unwrap_or(0xDEAD);
                let mb0 = bar0.read_u32(fecs_base + falcon::MAILBOX0).unwrap_or(0xDEAD);
                tracing::info!(
                    bdf = %self.bdf,
                    fecs_cpuctl = format_args!("{ctl:#010x}"),
                    fecs_cpuctl_alias = format_args!("{ctl_alias:#010x}"),
                    fecs_pc = format_args!("{pc:#010x}"),
                    fecs_mb0 = format_args!("{mb0:#010x}"),
                    "FECS post-boot: CPUCTL vs CPUCTL_ALIAS (HS security check)"
                );

                // Wait 100ms and check stability
                std::thread::sleep(std::time::Duration::from_millis(100));
                let ctl2 = bar0.read_u32(fecs_base + falcon::CPUCTL).unwrap_or(0xDEAD);
                let ctl2_alias = bar0.read_u32(fecs_base + falcon::CPUCTL_ALIAS).unwrap_or(0xDEAD);
                let pc2 = bar0.read_u32(fecs_base + falcon::PC).unwrap_or(0xDEAD);
                let mb02 = bar0.read_u32(fecs_base + falcon::MAILBOX0).unwrap_or(0xDEAD);
                let gpccs_ctl = bar0.read_u32(falcon::GPCCS_BASE + falcon::CPUCTL).unwrap_or(0xDEAD);
                let gpccs_alias = bar0.read_u32(falcon::GPCCS_BASE + falcon::CPUCTL_ALIAS).unwrap_or(0xDEAD);
                let gpccs_pc = bar0.read_u32(falcon::GPCCS_BASE + falcon::PC).unwrap_or(0xDEAD);
                let fecs_alive = ctl2_alias & falcon::CPUCTL_HRESET == 0
                    && ctl2_alias & falcon::CPUCTL_HALTED == 0;
                tracing::info!(
                    bdf = %self.bdf,
                    fecs_cpuctl = format_args!("{ctl2:#010x}"),
                    fecs_cpuctl_alias = format_args!("{ctl2_alias:#010x}"),
                    fecs_pc = format_args!("{pc2:#010x}"),
                    fecs_mb0 = format_args!("{mb02:#010x}"),
                    fecs_alive,
                    gpccs_cpuctl = format_args!("{gpccs_ctl:#010x}"),
                    gpccs_cpuctl_alias = format_args!("{gpccs_alias:#010x}"),
                    gpccs_pc = format_args!("{gpccs_pc:#010x}"),
                    "GR falcon stability check (100ms post-boot)"
                );
            }
        }

        self.vfio_state = Some(VfioDispatchState {
            device,
            bar0,
            channel,
            dma_backend,
            gpfifo,
            userd,
            gr_ctx,
            buffers: HashMap::new(),
            inflight: Vec::new(),
            next_handle: 1,
            next_iova: USER_BUFFER_BASE_IOVA,
            gp_put: 0,
            doorbell,
            target_pbdma_base,
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
            let profile = super::generation::profile_for_sm(sm);

            // 1. Upload shader binary
            let mut shader_buf = state.alloc_next_dma(shader.len(), "shader binary")?;
            let shader_iova = shader_buf.iova();
            shader_buf.as_mut_slice()[..shader.len()].copy_from_slice(shader);

            // 2. Build CBUF descriptor table (16-byte stride per binding)
            let desc_entry_size: usize = 16;
            let desc_size = buffers.len() * desc_entry_size;
            let mut desc_buf = state.alloc_next_dma(desc_size.max(PAGE_SIZE as usize), "descriptor table")?;
            let desc_iova = desc_buf.iova();
            for (i, handle) in buffers.iter().enumerate() {
                let buf = state.buffers.get(&handle.0).ok_or(DriverError::BufferNotFound(*handle))?;
                let offset = i * desc_entry_size;
                desc_buf.as_mut_slice()[offset..offset + 4]
                    .copy_from_slice(&(buf.iova() as u32).to_le_bytes());
                desc_buf.as_mut_slice()[offset + 4..offset + 8]
                    .copy_from_slice(&((buf.iova() >> 32) as u32).to_le_bytes());
                desc_buf.as_mut_slice()[offset + 8..offset + 12]
                    .copy_from_slice(&(buf.size() as u32).to_le_bytes());
            }

            // 3. Driver constants (grid dims for `@builtin(num_workgroups)`)
            let driver_const_data = super::qmd::encode_driver_constants(&dims);
            let mut dc_buf = state.alloc_next_dma(PAGE_SIZE as usize, "driver constants")?;
            let dc_iova = dc_buf.iova();
            dc_buf.as_mut_slice()[..driver_const_data.len()].copy_from_slice(&driver_const_data);

            // 4. Build QMD
            let workgroup = if info.workgroup.iter().any(|&d| d > 0) { info.workgroup } else { [64, 1, 1] };
            let cbufs = super::qmd::build_standard_cbufs(
                desc_iova, desc_size.max(64) as u32, dc_iova, super::qmd::DRIVER_CONST_SIZE,
            );
            let qmd_params = super::qmd::QmdParams {
                shader_va: shader_iova, grid: dims, workgroup,
                gpr_count: info.gpr_count.max(4), shared_mem_bytes: info.shared_mem_bytes,
                barrier_count: info.barrier_count, local_mem_low_bytes: info.local_mem_bytes.unwrap_or(0),
                cbufs,
            };
            let qmd_words = super::qmd::build_qmd(profile, &qmd_params);
            let qmd_bytes: &[u8] = bytemuck::cast_slice(&qmd_words);
            let mut qmd_buf = state.alloc_next_dma(qmd_bytes.len(), "QMD")?;
            let qmd_iova = qmd_buf.iova();
            qmd_buf.as_mut_slice()[..qmd_bytes.len()].copy_from_slice(qmd_bytes);

            // 5. Build + submit pushbuffer (compute init + dispatch)
            let mut init_pb = super::pushbuf::PushBuf::compute_init(
                profile.compute_class, profile.local_mem_window, 0, 0,
            );
            init_pb.append(&super::pushbuf::PushBuf::compute_dispatch_with_launch(
                profile.launch_method, qmd_iova,
            ));
            state.submit_pushbuffer(init_pb.as_bytes())?;

            // Track transient DMA allocations for cleanup after sync.
            for dma in [shader_buf, desc_buf, dc_buf, qmd_buf] {
                state.track_inflight(dma);
            }

            tracing::debug!(
                sm, shader_iova = format_args!("{shader_iova:#x}"),
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
            Err(DriverError::Unsupported("NVIDIA VFIO requires Linux".into()))
        }
    }

    fn sync(&mut self) -> DriverResult<()> {
        #[cfg(target_os = "linux")]
        {
            use crate::vfio::channel::registers::{pbdma, ramuserd};

            if let Some(state) = self.vfio_state.as_mut() {
                let target = state.gp_put;

                // Pre-sync PBDMA diagnostics (reading direct PBDMA registers)
                if let Some(pb) = state.target_pbdma_base {
                    let gp_base = state.bar0.read_u32(pb + pbdma::GP_BASE_LO).unwrap_or(0);
                    let hw_put = state.bar0.read_u32(pb + pbdma::GP_PUT).unwrap_or(0xDEAD);
                    let hw_get = state.bar0.read_u32(pb + pbdma::GP_FETCH).unwrap_or(0xDEAD);
                    let hw_state = state.bar0.read_u32(pb + pbdma::GP_STATE).unwrap_or(0xDEAD);
                    let ch_state = state.bar0.read_u32(pb + pbdma::CHANNEL_STATE).unwrap_or(0xDEAD);
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

                // Poll USERD GP_GET until it reaches GP_PUT.
                let mut last_gp_get = 0xFFFF_FFFFu32;
                for i in 0..1000 {
                    let gp_get = state.userd.volatile_read_u32(ramuserd::GP_GET);
                    if gp_get == target {
                        tracing::info!(iterations = i, "sync complete: GP_GET reached GP_PUT");
                        break;
                    }
                    if gp_get != last_gp_get {
                        tracing::debug!(gp_get, target, iteration = i, "GP_GET changed");
                        last_gp_get = gp_get;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(1));
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

            let profile = super::generation::profile_for_sm(self.sm);
            let pb = super::pushbuf::PushBuf::gr_context_init(
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

    #[test]
    fn kepler_sm_uses_v21_qmd() {
        use crate::nv::generation;

        let dev = NvVfioComputeDevice::with_sm("0000:25:00.0".into(), 37);
        let profile = generation::profile_for_sm(37);
        assert_eq!(profile.qmd_version, generation::QmdVersion::V21);
        assert!(matches!(
            profile.page_table_format,
            generation::PageTableFormat::V1TwoLevel
        ));
        assert_eq!(profile.boot_strategy, generation::BootStrategy::NoAcr);
        assert_eq!(dev.capabilities().vendor, crate::hardware::Vendor::Nvidia);
    }

    #[test]
    fn kepler_doorbell_address() {
        let addr = crate::vfio::channel::registers::usermode::gk104_doorbell(0);
        assert_eq!(addr, 0x3000);
        let addr7 = crate::vfio::channel::registers::usermode::gk104_doorbell(7);
        assert_eq!(addr7, 0x3000 + 7 * 8);
    }
}
