// SPDX-License-Identifier: AGPL-3.0-or-later
//! Page table and engine register capture (BAR0 / PRAMIN).

mod bar0;
mod types;
mod walk;

pub use types::{
    ChannelCapture, ChannelInfo, EntryFlags, InstanceBlock, PageDirectory, PageEntry, PageTable,
    PageTableDump, Pd0Directory, Pd0Entry, decode_entry_addr,
};
pub use types::EngineRegisters;

pub(crate) use bar0::Bar0Rw;

use crate::error::ChannelError;

use super::super::registers::misc;
use walk::{channel_info_from_scan, scan_channels, walk_channel_page_tables};

/// Detect the currently bound driver for a BDF.
pub fn detect_driver(bdf: &str) -> String {
    let sysfs = crate::linux_paths::sysfs_root();
    let link = format!("{sysfs}/bus/pci/devices/{bdf}/driver");
    match std::fs::read_link(&link) {
        Ok(p) => p
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unknown".into()),
        Err(_) => "unbound".into(),
    }
}

/// Capture the full page table state from a GPU at the given BDF.
///
/// Works regardless of which driver is currently bound (nouveau, nvidia,
/// vfio-pci, or unbound with BAR0 still accessible). The capture includes:
/// - All active channels found in PCCSR (0-511)
/// - For each channel: full PD3→PD2→PD1→PD0→PT walk (all non-zero entries)
/// - Engine register state (PFIFO, PMU, FECS, GPCCS, SEC2, MMU)
///
/// Set `max_channels` to limit how many channels are walked (0 = all found).
/// Capture using an existing VFIO `MappedBar` — no sysfs resource0 open needed.
///
/// Used by the glowplug daemon to perform oracle captures on VFIO-bound devices
/// through the daemon's existing bar0 mapping, avoiding the sysfs mmap that
/// hangs when vfio-pci owns the device.
pub fn capture_page_tables_via_mapped_bar(
    bdf: &str,
    mapped_bar: &crate::vfio::device::MappedBar,
    max_channels: usize,
) -> Result<PageTableDump, ChannelError> {
    // SAFETY: `mapped_bar` is a live VFIO BAR0 mapping; `base_ptr`/`size` describe the
    // full mapped region for the borrow of `mapped_bar`, satisfying `Bar0Rw::from_raw`.
    let bar0 = unsafe { Bar0Rw::from_raw(mapped_bar.base_ptr(), mapped_bar.size())? };
    capture_page_tables_inner(bdf, &bar0, max_channels)
}

/// A `Send`-safe handle to a BAR0 mapping for use across thread boundaries.
///
/// Wraps a raw pointer + size so it can be moved into `spawn_blocking` tasks.
/// The caller must ensure the underlying mapping outlives this handle.
///
/// ## Thread safety (`Send`)
///
/// The underlying BAR0 window is process-global MMIO: volatile reads are defined
/// for any thread once the mapping is established. This handle does **not**
/// implement [`Sync`]: sharing `&Bar0Handle` across threads without external
/// synchronization would duplicate the same unsafety as sharing raw pointers.
/// Use one handle per task or wrap externally if shared access is required.
pub struct Bar0Handle {
    ptr: *mut u8,
    size: usize,
}

impl std::fmt::Debug for Bar0Handle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bar0Handle")
            .field("size", &self.size)
            .field("ptr_nonnull", &!self.ptr.is_null())
            .finish()
    }
}

// SAFETY: Raw `*mut u8`/`usize` pair borrowed from an existing BAR0 mmap.
// `Send` allows moving the handle into worker threads (e.g. `spawn_blocking`);
// the caller must keep the underlying mapping alive for the handle's lifetime.
// Intentionally not `Sync`: sharing `&Bar0Handle` would duplicate unsynchronized
// raw-pointer access.
unsafe impl Send for Bar0Handle {}

impl Bar0Handle {
    /// Create a handle from a `MappedBar` reference.
    ///
    /// The handle borrows the mapping's address — the `MappedBar` (and its
    /// owning `VfioHolder`) must outlive any task using this handle.
    pub fn from_mapped_bar(bar: &crate::vfio::device::MappedBar) -> Self {
        Self {
            ptr: bar.base_ptr(),
            size: bar.size(),
        }
    }

    /// Perform an oracle page table capture using this BAR0 mapping.
    pub fn capture_page_tables(
        &self,
        bdf: &str,
        max_channels: usize,
    ) -> Result<PageTableDump, ChannelError> {
        // SAFETY: `Bar0Handle` is only constructed from a live `MappedBar`; the caller
        // keeps that mapping alive, so `ptr`/`size` remain valid for `Bar0Rw::from_raw`.
        let bar0 = unsafe { Bar0Rw::from_raw(self.ptr, self.size)? };
        capture_page_tables_inner(bdf, &bar0, max_channels)
    }

    /// Read a 32-bit BAR0 register with proper error handling.
    pub fn try_read_u32(&self, offset: usize) -> Result<u32, ChannelError> {
        // SAFETY: Same invariants as `capture_page_tables`: underlying BAR0 mapping outlives
        // this handle, so `ptr`/`size` satisfy `Bar0Rw::from_raw`.
        let bar0 = unsafe { Bar0Rw::from_raw(self.ptr, self.size)? };
        bar0.try_read_u32(offset)
    }

    /// Write a 32-bit BAR0 register with proper error handling.
    pub fn try_write_u32(&self, offset: usize, val: u32) -> Result<(), ChannelError> {
        // SAFETY: Same invariants as `capture_page_tables`: underlying BAR0 mapping outlives
        // this handle, so `ptr`/`size` satisfy `Bar0Rw::from_raw`.
        let bar0 = unsafe { Bar0Rw::from_raw(self.ptr, self.size)? };
        bar0.try_write_u32(offset, val)
    }

    /// BAR0 mapping size in bytes.
    #[must_use]
    pub const fn size(&self) -> usize {
        self.size
    }
}

pub fn capture_page_tables(bdf: &str, max_channels: usize) -> Result<PageTableDump, ChannelError> {
    let bar0 = Bar0Rw::open(bdf)?;
    capture_page_tables_inner(bdf, &bar0, max_channels)
}

fn capture_page_tables_inner(
    bdf: &str,
    bar0: &Bar0Rw,
    max_channels: usize,
) -> Result<PageTableDump, ChannelError> {
    let driver = detect_driver(bdf);

    let boot0 = bar0.read_u32(misc::BOOT0);
    if boot0 == 0xFFFF_FFFF {
        return Err(ChannelError::Bar0ReadsAllOnes);
    }

    let saved_window = bar0.read_u32(misc::BAR0_WINDOW);
    let raw_channels = scan_channels(bar0);

    let limit = if max_channels == 0 {
        raw_channels.len()
    } else {
        max_channels.min(raw_channels.len())
    };

    let mut channels = Vec::new();
    for &(id, inst_reg, chan_reg) in raw_channels.iter().take(limit) {
        let info = channel_info_from_scan(bar0, id, inst_reg, chan_reg);
        let capture = walk_channel_page_tables(bar0, &info);
        channels.push(capture);
    }

    let engine_registers = super::engine_regs::capture_engine_registers(bar0);

    // Restore BAR0 window
    bar0.set_window((saved_window as u64) << 16);

    let timestamp = {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        format!("{}s", now.as_secs())
    };

    Ok(PageTableDump {
        bdf: bdf.to_string(),
        driver,
        boot0,
        timestamp,
        channels,
        engine_registers,
    })
}
