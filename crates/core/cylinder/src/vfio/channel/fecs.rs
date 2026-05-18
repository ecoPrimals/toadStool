// SPDX-License-Identifier: AGPL-3.0-or-later
//! FECS (Front-End Command Scheduler) falcon method protocol.
//!
//! FECS manages GR context switching — allocating, loading, and saving the
//! per-channel GR register state. Communication with FECS happens via a
//! mailbox protocol: write method + data to FECS registers, trigger an
//! interrupt, poll for completion.
//!
//! This module implements the host-side of that protocol, enabling:
//! - `INIT_CTXSW`: Initialize context switching for FECS/GPCCS pair
//! - `BIND_CHANNEL`: Bind a new channel + GR context buffer to FECS
//! - `SET_WATCHDOG_TIMEOUT`: Configure FECS watchdog
//!
//! # Register protocol (from nouveau `gf100_gr_fecs_set_watchdog_timeout`)
//!
//! ```text
//! 1. Write data  → FECS_BASE + GR_FECS_MAILBOX0 (0x840)
//! 2. Write method → FECS_BASE + MTHD_DATA (0x500) + MTHD_CMD (0x504)
//! 3. Poll          FECS_BASE + MTHD_CMD until bit 0 == 0
//! 4. Read result → FECS_BASE + GR_FECS_MAILBOX0 (0x840)
//! ```
//!
//! # Nouveau source references
//!
//! - `drivers/gpu/drm/nouveau/nvkm/engine/gr/ctxgv100.c`
//! - `drivers/gpu/drm/nouveau/nvkm/engine/gr/gf100.c` (`gr_fecs_method`)

use crate::error::{DriverError, DriverResult};
use crate::vfio::device::MappedBar;

use super::registers::falcon;

/// FECS method IDs from nouveau and open-gpu-kernel-modules.
///
/// These are written to `FECS_BASE + MTHD_CMD` (0x504) to invoke
/// specific FECS firmware functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum FecsMethod {
    /// Initialize context switching for the GR engine.
    /// Triggers FECS to scan for GPCCS, allocate internal structures,
    /// and prepare for channel scheduling.
    /// nouveau: `GRCTX_CMD_CTXSW_INIT`
    InitCtxsw = 0x10,
    /// Halt context switching. FECS stops scheduling channels.
    /// nouveau: `GRCTX_CMD_CTXSW_HALT`
    HaltCtxsw = 0x11,
    /// Bind a channel to FECS for context switching.
    /// Data: instance block address >> 12 in MAILBOX0.
    /// nouveau: `GRCTX_CMD_BIND_CHANNEL`
    BindChannel = 0x21,
    /// Set the watchdog timeout value.
    /// Data: timeout in MAILBOX0.
    /// nouveau: `GRCTX_CMD_SET_WATCHDOG_TIMEOUT`
    SetWatchdogTimeout = 0x30,
    /// Commit the GR context buffer for a channel.
    /// Data: instance block address >> 12 in MAILBOX0.
    /// nouveau: `GRCTX_CMD_COMMIT`
    Commit = 0x22,
    /// Discover the GR context size (golden context size in bytes).
    /// Returns size in MAILBOX0 after completion.
    /// nouveau: `GRCTX_CMD_DISCOVER_IMAGE_SIZE`
    DiscoverImageSize = 0x40,
    /// Discover the GR ZCULL context size.
    /// nouveau: `GRCTX_CMD_DISCOVER_ZCULL_IMAGE_SIZE`
    DiscoverZcullImageSize = 0x41,
    /// Discover the PM context size.
    /// nouveau: `GRCTX_CMD_DISCOVER_PM_IMAGE_SIZE`
    DiscoverPmImageSize = 0x42,
}

/// Result of a FECS method invocation.
#[derive(Debug, Clone)]
pub struct FecsMethodResult {
    /// Whether the method completed successfully.
    pub success: bool,
    /// Value in MAILBOX0 after completion (return data for queries).
    pub mailbox0: u32,
    /// Value in MTHD_STATUS after completion.
    pub status: u32,
    /// Number of poll iterations before completion.
    pub poll_count: u32,
}

/// Default timeout for FECS method completion polling.
const FECS_METHOD_TIMEOUT_MS: u64 = 2000;
/// Poll interval between FECS method status checks.
const FECS_METHOD_POLL_INTERVAL_MS: u64 = 1;

/// Send a method to FECS and wait for completion.
///
/// Implements the FECS mailbox protocol used by nouveau for GR context
/// management. The protocol:
///
/// 1. Write `data` to `MAILBOX0`
/// 2. Clear `MAILBOX1` (old completion flag)
/// 3. Write 0 to `MTHD_DATA`, method to `MTHD_CMD`
/// 4. Write 1 to `MAILBOX1` (trigger)
/// 5. Poll `MAILBOX1` until FECS clears it to 0
/// 6. Read `MTHD_STATUS` for result code
///
/// # Errors
///
/// Returns error if BAR0 writes fail or the method times out.
pub fn fecs_method(
    bar0: &MappedBar,
    method: FecsMethod,
    data: u32,
) -> DriverResult<FecsMethodResult> {
    fecs_method_on(bar0, falcon::FECS_BASE, method, data)
}

/// Send a method to a falcon at a given base address.
///
/// Uses the GR FECS method protocol from nouveau `gf100_gr_fecs_method`:
///   1. Write data to GR_FECS_MAILBOX0 (base + 0x840)
///   2. Write 0 to MTHD_DATA (base + 0x500)
///   3. Write method to MTHD_CMD (base + 0x504) — triggers execution
///   4. Poll MTHD_CMD until bit 0 clears (FECS clears when done)
///   5. Read result from GR_FECS_MAILBOX0 (base + 0x840)
pub fn fecs_method_on(
    bar0: &MappedBar,
    falcon_base: usize,
    method: FecsMethod,
    data: u32,
) -> DriverResult<FecsMethodResult> {
    use std::borrow::Cow;

    // 1. Write method data to GR FECS MAILBOX0 (PGRAPH-wrapped, not falcon core)
    bar0.write_u32(falcon_base + falcon::GR_FECS_MAILBOX0, data)
        .map_err(|e| DriverError::SubmitFailed(Cow::Owned(
            format!("FECS GR_MAILBOX0 write: {e}")
        )))?;

    // 2. Write method (MTHD_DATA=0, MTHD_CMD=method — writing CMD triggers)
    bar0.write_u32(falcon_base + falcon::MTHD_DATA, 0)
        .map_err(|e| DriverError::SubmitFailed(Cow::Owned(
            format!("FECS MTHD_DATA write: {e}")
        )))?;
    bar0.write_u32(falcon_base + falcon::MTHD_CMD, method as u32)
        .map_err(|e| DriverError::SubmitFailed(Cow::Owned(
            format!("FECS MTHD_CMD write: {e}")
        )))?;

    // 3. Poll MTHD_CMD until bit 0 clears (FECS firmware clears it on completion)
    let deadline = std::time::Instant::now()
        + std::time::Duration::from_millis(FECS_METHOD_TIMEOUT_MS);
    let mut poll_count = 0u32;

    loop {
        let cmd = bar0.read_u32(falcon_base + falcon::MTHD_CMD).unwrap_or(0xDEAD);
        if cmd & 1 == 0 {
            break;
        }
        poll_count += 1;

        if std::time::Instant::now() > deadline {
            let pc = bar0.read_u32(falcon_base + falcon::PC).unwrap_or(0xDEAD);
            let alias = bar0.read_u32(falcon_base + falcon::CPUCTL_ALIAS).unwrap_or(0xDEAD);
            let mbox = bar0.read_u32(falcon_base + falcon::GR_FECS_MAILBOX0).unwrap_or(0xDEAD);
            tracing::warn!(
                method = ?method,
                cmd = format_args!("{cmd:#010x}"),
                pc = format_args!("{pc:#010x}"),
                cpuctl_alias = format_args!("{alias:#010x}"),
                gr_mailbox0 = format_args!("{mbox:#010x}"),
                poll_count,
                "FECS method timeout (MTHD_CMD bit 0 not cleared)"
            );
            return Err(DriverError::Unsupported(
                format!(
                    "FECS method {method:?} timed out after {FECS_METHOD_TIMEOUT_MS}ms \
                     (cmd={cmd:#x}, pc={pc:#x})"
                ).into()
            ));
        }

        std::thread::sleep(std::time::Duration::from_millis(FECS_METHOD_POLL_INTERVAL_MS));
    }

    // 4. Read result from GR FECS MAILBOX0
    let mailbox0 = bar0.read_u32(falcon_base + falcon::GR_FECS_MAILBOX0).unwrap_or(0);
    let status = bar0.read_u32(falcon_base + falcon::MTHD_STATUS).unwrap_or(0xDEAD);

    tracing::info!(
        method = ?method,
        data = format_args!("{data:#010x}"),
        mailbox0 = format_args!("{mailbox0:#010x}"),
        status = format_args!("{status:#010x}"),
        poll_count,
        "FECS method completed"
    );

    Ok(FecsMethodResult {
        success: status == 0 || status == 0x10,
        mailbox0,
        status,
        poll_count,
    })
}

/// Initialize FECS context switching.
///
/// Must be called before any channels can be scheduled on the GR engine.
/// On warm handoff from nouveau, this may already be done — but calling
/// it again is safe (FECS re-initializes its internal tables).
pub fn fecs_init_ctxsw(bar0: &MappedBar) -> DriverResult<FecsMethodResult> {
    tracing::info!("sending FECS INIT_CTXSW");
    fecs_method(bar0, FecsMethod::InitCtxsw, 0)
}

/// Bind a channel's instance block to FECS for context switching.
///
/// `inst_ptr` is the physical/IOVA address of the channel's instance block.
/// FECS uses this to locate the channel's GR context buffer pointer
/// (at instance block offset 0x210/0x214) and manage context save/restore.
pub fn fecs_bind_channel(bar0: &MappedBar, inst_ptr: u64) -> DriverResult<FecsMethodResult> {
    let inst_shifted = (inst_ptr >> 12) as u32;
    tracing::info!(
        inst_ptr = format_args!("{inst_ptr:#010x}"),
        inst_shifted = format_args!("{inst_shifted:#010x}"),
        "sending FECS BIND_CHANNEL"
    );
    fecs_method(bar0, FecsMethod::BindChannel, inst_shifted)
}

/// Commit a channel's GR context to FECS.
///
/// After binding, this tells FECS to copy the golden context into the
/// channel's GR context buffer and mark the channel as ready for scheduling.
pub fn fecs_commit(bar0: &MappedBar, inst_ptr: u64) -> DriverResult<FecsMethodResult> {
    let inst_shifted = (inst_ptr >> 12) as u32;
    tracing::info!(
        inst_ptr = format_args!("{inst_ptr:#010x}"),
        inst_shifted = format_args!("{inst_shifted:#010x}"),
        "sending FECS COMMIT"
    );
    fecs_method(bar0, FecsMethod::Commit, inst_shifted)
}

/// Query the GR context image size from FECS.
///
/// Returns the size in bytes that FECS expects for each channel's GR
/// context buffer. The golden context template is this size.
pub fn fecs_discover_image_size(bar0: &MappedBar) -> DriverResult<u32> {
    let result = fecs_method(bar0, FecsMethod::DiscoverImageSize, 0)?;
    tracing::info!(
        gr_ctx_size = result.mailbox0,
        gr_ctx_size_hex = format_args!("{:#x}", result.mailbox0),
        "FECS reports GR context image size"
    );
    Ok(result.mailbox0)
}

/// Set the FECS watchdog timeout.
///
/// nouveau sets this to `0x7FFFFFFF` (max) during GR init.
pub fn fecs_set_watchdog_timeout(bar0: &MappedBar, timeout: u32) -> DriverResult<FecsMethodResult> {
    fecs_method(bar0, FecsMethod::SetWatchdogTimeout, timeout)
}

/// Check if FECS is alive and responsive via CPUCTL_ALIAS.
///
/// Returns `true` if FECS is running (not in HRESET, not halted).
/// Uses CPUCTL_ALIAS (0x130) which bypasses the HS security lock
/// on Volta+ falcons.
pub fn fecs_is_alive(bar0: &MappedBar) -> bool {
    let alias = bar0.read_u32(falcon::FECS_BASE + falcon::CPUCTL_ALIAS).unwrap_or(0xDEAD);
    let is_bad = alias & 0xBADF_0000 == 0xBADF_0000 || alias & 0xBAD0_0000 == 0xBAD0_0000;
    if is_bad {
        return false;
    }
    let in_hreset = alias & falcon::CPUCTL_HRESET != 0;
    let halted = alias & falcon::CPUCTL_HALTED != 0;
    !in_hreset && !halted
}

/// Read a block of VRAM via the BAR0 PRAMIN window (0x700000–0x7FFFFF).
///
/// The PRAMIN window provides a 1 MiB "porthole" into VRAM, controlled by
/// BAR0_WINDOW (0x1700). To read VRAM address `vram_addr`:
///   1. Set BAR0_WINDOW = `vram_addr >> 16`
///   2. Read BAR0 + 0x700000 + `(vram_addr & 0xFFFF)`
///
/// # Limitations
///
/// Reads up to `len` bytes (must be 4-byte aligned). Returns the data
/// or an error if BAR0 reads fail. Window changes for each 64 KiB block.
pub fn pramin_read(
    bar0: &MappedBar,
    vram_addr: u64,
    len: usize,
) -> DriverResult<Vec<u8>> {
    use std::borrow::Cow;

    const PRAMIN_BASE: usize = 0x0070_0000;
    const PRAMIN_SIZE: usize = 0x0001_0000; // 64 KiB window
    const BAR0_WINDOW: usize = 0x0000_1700;

    if !len.is_multiple_of(4) {
        return Err(DriverError::MmapFailed(Cow::Borrowed(
            "PRAMIN read length must be 4-byte aligned"
        )));
    }

    let mut result = vec![0u8; len];
    let mut offset = 0usize;
    let mut current_window = u64::MAX; // force first window set

    while offset < len {
        let addr = vram_addr + offset as u64;
        let window = addr >> 16;
        let within = (addr & 0xFFFF) as usize;

        if window != current_window {
            bar0.write_u32(BAR0_WINDOW, window as u32)
                .map_err(|e| DriverError::MmapFailed(Cow::Owned(
                    format!("PRAMIN window set: {e}")
                )))?;
            current_window = window;
        }

        let remaining = len - offset;
        let window_remaining = PRAMIN_SIZE - within;
        let chunk = remaining.min(window_remaining);

        for i in (0..chunk).step_by(4) {
            let val = bar0.read_u32(PRAMIN_BASE + within + i)
                .map_err(|e| DriverError::MmapFailed(Cow::Owned(
                    format!("PRAMIN read at {:#x}: {e}", addr + i as u64)
                )))?;
            result[offset + i..offset + i + 4].copy_from_slice(&val.to_le_bytes());
        }

        offset += chunk;
    }

    Ok(result)
}

/// Write a block of data to VRAM via the BAR0 PRAMIN window.
///
/// Inverse of `pramin_read`. Writes system memory data into VRAM.
pub fn pramin_write(
    bar0: &MappedBar,
    vram_addr: u64,
    data: &[u8],
) -> DriverResult<()> {
    use std::borrow::Cow;

    const PRAMIN_BASE: usize = 0x0070_0000;
    const PRAMIN_SIZE: usize = 0x0001_0000;
    const BAR0_WINDOW: usize = 0x0000_1700;

    if !data.len().is_multiple_of(4) {
        return Err(DriverError::MmapFailed(Cow::Borrowed(
            "PRAMIN write length must be 4-byte aligned"
        )));
    }

    let mut offset = 0usize;
    let mut current_window = u64::MAX;

    while offset < data.len() {
        let addr = vram_addr + offset as u64;
        let window = addr >> 16;
        let within = (addr & 0xFFFF) as usize;

        if window != current_window {
            bar0.write_u32(BAR0_WINDOW, window as u32)
                .map_err(|e| DriverError::MmapFailed(Cow::Owned(
                    format!("PRAMIN window set: {e}")
                )))?;
            current_window = window;
        }

        let remaining = data.len() - offset;
        let window_remaining = PRAMIN_SIZE - within;
        let chunk = remaining.min(window_remaining);

        for i in (0..chunk).step_by(4) {
            let val = u32::from_le_bytes([
                data[offset + i],
                data[offset + i + 1],
                data[offset + i + 2],
                data[offset + i + 3],
            ]);
            bar0.write_u32(PRAMIN_BASE + within + i, val)
                .map_err(|e| DriverError::MmapFailed(Cow::Owned(
                    format!("PRAMIN write at {:#x}: {e}", addr + i as u64)
                )))?;
        }

        offset += chunk;
    }

    Ok(())
}

/// Probe nouveau's golden context location by reading the FECS golden
/// context pointer from VRAM via PRAMIN.
///
/// After nouveau initializes GR, FECS stores a golden context template
/// at a VRAM address. We can discover this by reading FECS internal state.
/// Returns the VRAM address of the golden context if found, or 0 if unknown.
pub fn probe_golden_context_vram_addr(bar0: &MappedBar) -> u64 {
    // FECS stores the golden context VRAM address in its DMEM at a
    // firmware-version-dependent offset. For now, we try to discover
    // it via the FECS DISCOVER_IMAGE_SIZE method and heuristics.
    //
    // On a warm handoff where nouveau has initialized GR, the golden
    // context typically lives near the start of VRAM (within the first
    // few MiB allocated by nouveau's VMM).
    //
    // Probe strategy: read the first 4 bytes at known VRAM offsets
    // that nouveau uses for GR context allocation. If we find a
    // non-zero pattern that looks like valid GR context header data,
    // that's likely the golden context.
    const PROBE_OFFSETS: &[u64] = &[
        0x0000_0000, // base of VRAM
        0x0002_0000, // 128 KiB
        0x0010_0000, // 1 MiB
        0x0020_0000, // 2 MiB
    ];

    for &offset in PROBE_OFFSETS {
        match pramin_read(bar0, offset, 16) {
            Ok(data) => {
                let word0 = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                let word1 = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
                if word0 != 0 && word0 != 0xFFFF_FFFF
                    && word0 & 0xBAD0_0000 != 0xBAD0_0000
                {
                    tracing::info!(
                        vram_offset = format_args!("{offset:#010x}"),
                        word0 = format_args!("{word0:#010x}"),
                        word1 = format_args!("{word1:#010x}"),
                        "PRAMIN probe: non-zero data at VRAM offset"
                    );
                }
            }
            Err(e) => {
                tracing::debug!(
                    vram_offset = format_args!("{offset:#010x}"),
                    error = %e,
                    "PRAMIN probe failed at offset"
                );
            }
        }
    }

    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fecs_method_values() {
        assert_eq!(FecsMethod::InitCtxsw as u32, 0x10);
        assert_eq!(FecsMethod::HaltCtxsw as u32, 0x11);
        assert_eq!(FecsMethod::BindChannel as u32, 0x21);
        assert_eq!(FecsMethod::Commit as u32, 0x22);
        assert_eq!(FecsMethod::SetWatchdogTimeout as u32, 0x30);
        assert_eq!(FecsMethod::DiscoverImageSize as u32, 0x40);
    }

    #[test]
    fn fecs_method_result_debug() {
        let result = FecsMethodResult {
            success: true,
            mailbox0: 0x80000,
            status: 0,
            poll_count: 5,
        };
        let debug = format!("{result:?}");
        assert!(debug.contains("success: true"));
        assert!(debug.contains("524288"));
    }
}
