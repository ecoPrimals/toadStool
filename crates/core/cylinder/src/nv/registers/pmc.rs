// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(dead_code, reason = "hardware register constants — comprehensive coverage for evolving absorption")]

//! PMC (Power Management Controller) — engine enables and boot identity.

/// Chip identity and strap configuration read at boot.
pub const BOOT0: u32 = 0x0000_0000;
/// PMC master interrupt status.
pub const INTR: u32 = 0x0000_0100;
/// PMC interrupt enable mask (engine 0).
pub const INTR_EN_0: u32 = 0x0000_0140;
/// PMC interrupt enable SET — write-only, Volta+ (writing 1 bits enables).
pub const INTR_EN_SET_0: u32 = 0x0000_0160;
/// PMC interrupt enable CLEAR — write-only, Volta+ (writing 1 bits disables).
pub const INTR_EN_CLEAR_0: u32 = 0x0000_0180;
/// Master engine enable — write `0xFFFF_FFFF` to un-gate all present clock domains.
pub const ENABLE: u32 = 0x0000_0200;
/// Per-device engine enable (also exposed as PBDMA master enable on some GPUs).
pub const DEVICE_ENABLE: u32 = 0x0000_0204;

/// Per-generation interrupt register semantics.
///
/// Pre-Volta (Kepler/Maxwell/Pascal): INTR_EN_0 at 0x140 is directly writable.
/// Writing 0 disables all interrupts, writing the mask re-enables them.
///
/// Volta+ (GV100, TU10x, GA10x, AD10x, GH100, GB10x): INTR_EN_0 at 0x140 is
/// READ-ONLY (shows current enable mask). Interrupt control uses a SET/CLEAR
/// register pair at 0x160/0x180 — writing 1 bits to SET enables, writing 1 bits
/// to CLEAR disables. Writing to 0x140 is a NO-OP (Exp 229 lockup #5 confirmed).
#[derive(Debug, Clone, Copy)]
pub struct InterruptProfile {
    /// Offset to read the current interrupt enable mask (0x140, all gens).
    pub intr_en_readable: u32,
    /// Whether writing to `intr_en_readable` takes effect.
    /// true for Kepler/Maxwell/Pascal, false for Volta+.
    pub intr_en_writable: bool,
    /// Write-only SET register (Volta+). None for pre-Volta.
    pub intr_en_set: Option<u32>,
    /// Write-only CLEAR register (Volta+). None for pre-Volta.
    pub intr_en_clear: Option<u32>,
    /// Offset to read (and ACK edge-triggered) pending interrupts (0x100, all gens).
    pub intr_pending: u32,
}

impl InterruptProfile {
    /// Pre-Volta: INTR_EN_0 at 0x140 is directly writable.
    pub const PRE_VOLTA: Self = Self {
        intr_en_readable: INTR_EN_0,
        intr_en_writable: true,
        intr_en_set: None,
        intr_en_clear: None,
        intr_pending: INTR,
    };

    /// Volta+: INTR_EN_0 is read-only, use SET/CLEAR pair.
    pub const VOLTA_PLUS: Self = Self {
        intr_en_readable: INTR_EN_0,
        intr_en_writable: false,
        intr_en_set: Some(INTR_EN_SET_0),
        intr_en_clear: Some(INTR_EN_CLEAR_0),
        intr_pending: INTR,
    };

    /// Look up the interrupt profile for a given SM version.
    pub const fn for_sm(sm: u32) -> Self {
        if sm >= 70 {
            Self::VOLTA_PLUS
        } else {
            Self::PRE_VOLTA
        }
    }

    /// The BAR0 offset to write for disabling all interrupts.
    /// Volta+: writes to CLEAR register. Pre-Volta: writes to INTR_EN_0 directly.
    pub const fn disable_offset(&self) -> u32 {
        match self.intr_en_clear {
            Some(off) => off,
            None => self.intr_en_readable,
        }
    }

    /// The value to write for disabling all interrupts.
    /// Volta+ CLEAR: 0xFFFFFFFF (set all bits to disable).
    /// Pre-Volta direct: 0x00000000 (clear mask to disable).
    pub const fn disable_value(&self) -> u32 {
        if self.intr_en_writable {
            0x0000_0000
        } else {
            0xFFFF_FFFF
        }
    }
}

/// Quench GPU interrupt generation via direct BAR0 mmap using the appropriate
/// register semantics for the GPU generation.
///
/// Opens `/sys/bus/pci/devices/{bdf}/resource0`, mmaps 4 KiB, and writes to
/// the correct register (INTR_EN_CLEAR@0x180 for Volta+, INTR_EN_0@0x140
/// for pre-Volta). Also reads back to verify and logs the result.
///
/// # Safety
/// Uses `unsafe` for volatile MMIO register access via sysfs resource0 mmap.
/// The alternative (no quench) is a system-wide lockup from IRQ storm.
pub fn quench_interrupts(bdf: &str, profile: &InterruptProfile, context: &str) {
    let bar0_path = format!("/sys/bus/pci/devices/{bdf}/resource0");
    let f = match std::fs::OpenOptions::new().read(true).write(true).open(&bar0_path) {
        Ok(f) => f,
        Err(e) => {
            tracing::warn!(bdf, context, error = %e, "interrupt quench: cannot open BAR0");
            return;
        }
    };

    let map = unsafe {
        rustix::mm::mmap(
            std::ptr::null_mut(),
            0x1000,
            rustix::mm::ProtFlags::READ | rustix::mm::ProtFlags::WRITE,
            rustix::mm::MapFlags::SHARED,
            &f,
            0,
        )
    };

    match map {
        Ok(ptr) => {
            let base = ptr.cast::<u8>();
            // SAFETY: all offsets are within the 0x1000 mapped BAR0 page.
            let old_en = unsafe {
                std::ptr::read_volatile(
                    base.add(profile.intr_en_readable as usize).cast::<u32>(),
                )
            };

            let disable_off = profile.disable_offset() as usize;
            let disable_val = profile.disable_value();
            unsafe {
                std::ptr::write_volatile(
                    base.add(disable_off).cast::<u32>(),
                    disable_val,
                );
            }

            // SAFETY: offset is within the 0x1000 mapped BAR0 page.
            let new_en = unsafe {
                std::ptr::read_volatile(
                    base.add(profile.intr_en_readable as usize).cast::<u32>(),
                )
            };

            // SAFETY: offset is within the 0x1000 mapped BAR0 page.
            let pending = unsafe {
                std::ptr::read_volatile(
                    base.add(profile.intr_pending as usize).cast::<u32>(),
                )
            };

            // SAFETY: unmapping the 0x1000 BAR0 page mapped above.
            let _ = unsafe { rustix::mm::munmap(ptr, 0x1000) };

            tracing::info!(
                bdf,
                context,
                old_en = format_args!("0x{old_en:08x}"),
                new_en = format_args!("0x{new_en:08x}"),
                pending = format_args!("0x{pending:08x}"),
                disable_offset = format_args!("0x{disable_off:03x}"),
                "interrupt quench complete"
            );

            if new_en != 0 {
                tracing::warn!(
                    bdf, context,
                    new_en = format_args!("0x{new_en:08x}"),
                    "interrupt quench: INTR_EN still nonzero"
                );
            }
        }
        Err(e) => {
            tracing::warn!(bdf, context, error = %e, "interrupt quench: BAR0 mmap failed");
        }
    }
}

/// Set PCI command register bit 10 (INTx Disable) via sysfs config space.
/// This is a PCI-spec generic operation that works across all GPU generations.
pub fn intx_disable(bdf: &str, context: &str) {
    let cfg_path = format!("/sys/bus/pci/devices/{bdf}/config");
    let mut f = match std::fs::OpenOptions::new().read(true).write(true).open(&cfg_path) {
        Ok(f) => f,
        Err(e) => {
            tracing::warn!(bdf, context, error = %e, "INTx disable: cannot open config");
            return;
        }
    };

    use std::io::{Read, Seek, Write};
    let mut cmd_bytes = [0u8; 2];
    if f.seek(std::io::SeekFrom::Start(4)).is_ok()
        && f.read_exact(&mut cmd_bytes).is_ok()
    {
        let old_cmd = u16::from_le_bytes(cmd_bytes);
        let new_cmd = old_cmd | 0x0400;
        let _ = f.seek(std::io::SeekFrom::Start(4));
        let _ = f.write_all(&new_cmd.to_le_bytes());
        tracing::info!(
            bdf, context,
            old_cmd = format_args!("0x{old_cmd:04x}"),
            new_cmd = format_args!("0x{new_cmd:04x}"),
            "PCI INTx disabled"
        );
    }
}
