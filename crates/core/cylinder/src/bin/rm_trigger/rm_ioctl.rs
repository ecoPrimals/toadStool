// SPDX-License-Identifier: AGPL-3.0-or-later
//! NVIDIA RM ioctl wrappers — NVOS21/NVOS54 ABI adapters and alloc/control helpers.

use std::os::fd::AsFd;

use rustix::ioctl::{Ioctl, IoctlOutput, Opcode};

pub const NV_IOCTL_MAGIC: u8 = b'F';
const NV_ESC_RM_ALLOC: u8 = 0x2B;
const NV_ESC_RM_CONTROL: u8 = 0x2A;

/// nvidia-470 uses NVOS21 (28 bytes) for NV_ESC_RM_ALLOC (0x2B).
/// Status is at offset 24 — NOT at offset 28 like in NVOS64 from 510+.
/// NVOS21 has no `params_size` field; RM infers size from hClass.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Nvos21Parameters {
    h_root: u32,          // 0
    h_object_parent: u32, // 4
    h_object_new: u32,    // 8
    h_class: u32,         // 12
    p_alloc_parms: u64,   // 16
    status: u32,          // 24 — the REAL status field on 470.x
    _pad: u32,            // 28 — alignment padding (never used by kernel)
}

/// 470.x RM_CONTROL uses 32-byte NVOS54.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct Nvos54Parameters {
    h_client: u32,
    h_object: u32,
    cmd: u32,
    flags: u32,
    params: u64,
    params_size: u32,
    status: u32,
}

/// Scheduling control — enable field for TSG schedule.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct NvGpfifoScheduleParams {
    pub b_enable: u32,
}

pub const fn iowr(magic: u8, nr: u8, size: usize) -> Opcode {
    let dir: u32 = 3;
    (dir << 30) | ((size as u32 & 0x3FFF) << 16) | ((magic as u32) << 8) | nr as u32
}

pub const RM_ALLOC_OP: Opcode = iowr(
    NV_IOCTL_MAGIC,
    NV_ESC_RM_ALLOC,
    size_of::<Nvos21Parameters>(),
);
pub const RM_CTRL_OP: Opcode = iowr(
    NV_IOCTL_MAGIC,
    NV_ESC_RM_CONTROL,
    size_of::<Nvos54Parameters>(),
);

/// Rustix ioctl adapter for raw-buffer NVIDIA RM commands.
pub struct RmRawIoctl<const OP: Opcode> {
    pub ptr: *mut u8,
}

// SAFETY: OP is a compile-time NVIDIA RM ioctl opcode; ptr points to a properly
// sized buffer matching the kernel ABI; IS_MUTATING=true because kernel writes back.
unsafe impl<const OP: Opcode> Ioctl for RmRawIoctl<OP> {
    type Output = i32;
    const IS_MUTATING: bool = true;

    fn opcode(&self) -> Opcode {
        OP
    }

    fn as_ptr(&mut self) -> *mut std::ffi::c_void {
        self.ptr.cast()
    }

    /// # Safety
    /// Caller guarantees `out` points to valid ioctl return data.
    unsafe fn output_from_ptr(
        out: IoctlOutput,
        _: *mut std::ffi::c_void,
    ) -> rustix::io::Result<Self::Output> {
        Ok(out)
    }
}

/// Rustix ioctl adapter for typed struct RM commands.
struct RmIoctl<const OP: Opcode, T> {
    ptr: *mut T,
}

// SAFETY: same as RmRawIoctl but for typed repr(C) structs.
unsafe impl<const OP: Opcode, T> Ioctl for RmIoctl<OP, T> {
    type Output = i32;
    const IS_MUTATING: bool = true;

    fn opcode(&self) -> Opcode {
        OP
    }

    fn as_ptr(&mut self) -> *mut std::ffi::c_void {
        self.ptr.cast()
    }

    /// # Safety
    /// Caller guarantees `out` points to valid ioctl return data.
    unsafe fn output_from_ptr(
        out: IoctlOutput,
        _: *mut std::ffi::c_void,
    ) -> rustix::io::Result<Self::Output> {
        Ok(out)
    }
}

/// Result of an RM_ALLOC ioctl.
pub struct RmAllocResult {
    pub rc: i32,
    pub status: u32,
    /// The handle actually assigned by the kernel (may differ from requested).
    pub h_object_new: u32,
}

/// Issue NV_ESC_RM_ALLOC via raw 32-byte buffer.
///
/// nvidia-470 RM may REWRITE h_object_new with its own RM-assigned handle.
/// Callers MUST use the returned `h_object_new` for subsequent operations.
pub fn rm_alloc(
    fd: impl AsFd,
    root: u32,
    parent: u32,
    handle: u32,
    class: u32,
    params_ptr: u64,
    params_size: u32,
) -> RmAllocResult {
    let mut buf = [0xDDu8; 32];
    buf[0..4].copy_from_slice(&root.to_ne_bytes());
    buf[4..8].copy_from_slice(&parent.to_ne_bytes());
    buf[8..12].copy_from_slice(&handle.to_ne_bytes());
    buf[12..16].copy_from_slice(&class.to_ne_bytes());
    buf[16..24].copy_from_slice(&params_ptr.to_ne_bytes());
    let sentinel_24: u32 = if params_size > 0 {
        params_size
    } else {
        0xAAAA_AAAA
    };
    buf[24..28].copy_from_slice(&sentinel_24.to_ne_bytes());
    buf[28..32].copy_from_slice(&0xDEAD_BEEFu32.to_ne_bytes());

    // SAFETY: buf is 32 bytes matching NVOS21 kernel ABI; fd is valid.
    let ioctl = RmRawIoctl::<{ RM_ALLOC_OP }> {
        ptr: buf.as_mut_ptr(),
    };
    let rc = match unsafe { rustix::ioctl::ioctl(&fd, ioctl) } {
        Ok(v) => v,
        Err(e) => {
            eprintln!("  RM_ALLOC(cls=0x{class:04x}, h=0x{handle:08x}): errno={e}");
            return RmAllocResult {
                rc: -1,
                status: 0xFFFF_FFFF,
                h_object_new: handle,
            };
        }
    };

    let h_new_out = u32::from_ne_bytes([buf[8], buf[9], buf[10], buf[11]]);
    let val_24 = u32::from_ne_bytes([buf[24], buf[25], buf[26], buf[27]]);
    let val_28 = u32::from_ne_bytes([buf[28], buf[29], buf[30], buf[31]]);
    let status = if val_28 != 0xDEAD_BEEF {
        val_28
    } else {
        val_24
    };

    eprintln!(
        "  RM_ALLOC(cls=0x{:04x}, h=0x{:08x}→0x{:08x}): rc={} status=0x{:x}",
        class, handle, h_new_out, rc, status
    );

    RmAllocResult {
        rc,
        status,
        h_object_new: h_new_out,
    }
}

/// Issue NV_ESC_RM_CONTROL. Returns (ioctl_rc, rm_status).
pub fn rm_ctrl(
    fd: impl AsFd,
    client: u32,
    object: u32,
    cmd: u32,
    params_ptr: u64,
    params_size: u32,
) -> (i32, u32) {
    let mut p = Nvos54Parameters {
        h_client: client,
        h_object: object,
        cmd,
        params: params_ptr,
        params_size,
        status: 0,
        ..Default::default()
    };
    // SAFETY: p is repr(C) matching kernel NVOS54 ABI; fd is valid.
    let ioctl = RmIoctl::<{ RM_CTRL_OP }, Nvos54Parameters> { ptr: &mut p };
    let rc = match unsafe { rustix::ioctl::ioctl(&fd, ioctl) } {
        Ok(v) => v,
        Err(e) => {
            eprintln!("  RM_CTRL(cmd=0x{cmd:08x}, obj=0x{object:08x}): errno={e}");
            return (-1, p.status);
        }
    };
    eprintln!(
        "  RM_CTRL(cmd=0x{:08x}, obj=0x{:08x}): rc={} status=0x{:x}",
        cmd, object, rc, p.status
    );
    (rc, p.status)
}
