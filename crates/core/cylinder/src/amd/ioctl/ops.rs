// SPDX-License-Identifier: AGPL-3.0-or-later
//! AMD DRM ioctl operations — GEM, context, VA, BO list, CS, fence, query.

#[cfg(doc)]
use crate::error::DriverError;
use crate::error::DriverResult;
use std::os::unix::io::RawFd;

use super::types::*;

/// Size of a `#[repr(C)]` struct as a `u32` for ioctl encoding.
#[expect(
    clippy::cast_possible_truncation,
    reason = "asserted in bounds; kernel ioctl structs are always < 4 GiB"
)]
const fn size_of_u32<T>() -> u32 {
    assert!(std::mem::size_of::<T>() <= u32::MAX as usize);
    std::mem::size_of::<T>() as u32
}

/// Encode a Rust reference as a kernel-compatible `u64` pointer.
fn kernel_ptr<T>(r: &T) -> u64 {
    std::ptr::from_ref(r) as u64
}

/// Read the kernel's output from a `#[repr(C)]` ioctl struct — zero unsafe.
fn read_ioctl_output<T: bytemuck::Pod, R: bytemuck::Pod>(arg: &T) -> R {
    bytemuck::pod_read_unaligned(&bytemuck::bytes_of(arg)[..std::mem::size_of::<R>()])
}

/// Perform a named DRM ioctl on a `#[repr(C)]` struct.
fn amd_ioctl<T>(fd: RawFd, request: u64, arg: &mut T, name: &'static str) -> DriverResult<()> {
    crate::drm::drm_ioctl_named(fd, request, arg, name)
}

/// Perform a DRM ioctl and read a scalar output from the union overlay.
fn amd_ioctl_read<T: bytemuck::Pod, R: bytemuck::Pod>(
    fd: RawFd,
    request: u64,
    arg: &mut T,
    name: &'static str,
) -> DriverResult<R> {
    amd_ioctl(fd, request, arg, name)?;
    Ok(read_ioctl_output(arg))
}

/// Current `CLOCK_MONOTONIC` time in nanoseconds.
fn clock_monotonic_ns() -> u64 {
    let ts = rustix::time::clock_gettime(rustix::time::ClockId::Monotonic);
    #[expect(
        clippy::cast_sign_loss,
        reason = "CLOCK_MONOTONIC never returns negative values"
    )]
    let ns = (ts.tv_sec as u64)
        .saturating_mul(1_000_000_000)
        .saturating_add(ts.tv_nsec as u64);
    ns
}

/// Build an IOWR request number for an AMD DRM command.
const fn amd_iowr<T>(cmd: u32) -> u64 {
    crate::drm::drm_iowr_pub(cmd, size_of_u32::<T>())
}

/// Build an IOW request number for an AMD DRM command.
const fn amd_iow<T>(cmd: u32) -> u64 {
    crate::drm::drm_iow_pub(cmd, size_of_u32::<T>())
}

/// Create an amdgpu GPU context.
///
/// # Errors
///
/// Returns [`DriverError`] if the context allocation ioctl fails.
pub fn create_context(fd: RawFd) -> DriverResult<u32> {
    let mut ctx = AmdgpuCtx {
        op: AMDGPU_CTX_OP_ALLOC_CTX,
        ..Default::default()
    };
    amd_ioctl(
        fd,
        amd_iowr::<AmdgpuCtx>(DRM_AMDGPU_CTX),
        &mut ctx,
        "AMDGPU_CTX_ALLOC",
    )?;
    Ok(ctx.ctx_id)
}

/// Destroy an amdgpu GPU context.
///
/// # Errors
///
/// Returns [`DriverError`] if the context free ioctl fails.
pub fn destroy_context(fd: RawFd, ctx_id: u32) -> DriverResult<()> {
    let mut ctx = AmdgpuCtx {
        op: AMDGPU_CTX_OP_FREE_CTX,
        ctx_id,
        ..Default::default()
    };
    amd_ioctl(
        fd,
        amd_iowr::<AmdgpuCtx>(DRM_AMDGPU_CTX),
        &mut ctx,
        "AMDGPU_CTX_FREE",
    )
}

/// Allocate a GEM buffer object.
///
/// Returns `(handle, actual_size)`.
///
/// # Errors
///
/// Returns [`DriverError`] if the GEM create ioctl fails.
pub fn gem_create(fd: RawFd, size: u64, domains: u32) -> DriverResult<(u32, u64)> {
    let mut args = AmdgpuGemCreate {
        bo_size: size,
        alignment: 4096,
        domains: u64::from(domains),
        domain_flags: 0,
    };
    amd_ioctl(
        fd,
        amd_iowr::<AmdgpuGemCreate>(DRM_AMDGPU_GEM_CREATE),
        &mut args,
        "AMDGPU_GEM_CREATE",
    )?;
    let handle: u32 = read_ioctl_output(&args);
    Ok((handle, args.bo_size))
}

/// Get the mmap offset for a GEM buffer (for `mmap(2)`).
///
/// # Errors
///
/// Returns [`DriverError`] if the GEM mmap ioctl fails.
pub fn gem_mmap_offset(fd: RawFd, handle: u32) -> DriverResult<u64> {
    let mut args = AmdgpuGemMmap {
        handle_or_addr: u64::from(handle),
    };
    amd_ioctl(
        fd,
        amd_iowr::<AmdgpuGemMmap>(DRM_AMDGPU_GEM_MMAP),
        &mut args,
        "AMDGPU_GEM_MMAP",
    )?;
    Ok(args.handle_or_addr)
}

/// Map a GEM buffer into GPU virtual address space.
///
/// # Errors
///
/// Returns [`DriverError`] if the GEM VA map ioctl fails.
pub fn gem_va_map(fd: RawFd, handle: u32, va: u64, size: u64) -> DriverResult<()> {
    let mut args = AmdgpuGemVa {
        handle,
        operation: AMDGPU_VA_OP_MAP,
        flags: AMDGPU_VM_PAGE_READABLE | AMDGPU_VM_PAGE_WRITEABLE | AMDGPU_VM_PAGE_EXECUTABLE,
        va_address: va,
        map_size: size,
        ..Default::default()
    };
    amd_ioctl(
        fd,
        amd_iow::<AmdgpuGemVa>(DRM_AMDGPU_GEM_VA),
        &mut args,
        "AMDGPU_GEM_VA_MAP",
    )
}

/// Create a BO (buffer object) list for command submission.
///
/// # Errors
///
/// Returns [`DriverError`] if the BO list creation ioctl fails.
pub fn create_bo_list(fd: RawFd, handles: &[u32]) -> DriverResult<u32> {
    let entries: Vec<AmdgpuBoListEntry> = handles
        .iter()
        .map(|&h| AmdgpuBoListEntry {
            bo_handle: h,
            bo_priority: 0,
        })
        .collect();

    let mut req = AmdgpuBoListIn {
        operation: AMDGPU_BO_LIST_OP_CREATE,
        bo_number: u32::try_from(entries.len())
            .map_err(|_| crate::error::DriverError::platform_overflow("BO count fits in u32"))?,
        bo_info_size: size_of_u32::<AmdgpuBoListEntry>(),
        bo_info_ptr: entries.first().map_or(0, kernel_ptr),
        ..Default::default()
    };

    amd_ioctl_read(
        fd,
        amd_iowr::<AmdgpuBoListIn>(DRM_AMDGPU_BO_LIST),
        &mut req,
        "AMDGPU_BO_LIST_CREATE",
    )
}

/// Destroy a BO list.
///
/// # Errors
///
/// Returns [`DriverError`] if the BO list destruction ioctl fails.
pub fn destroy_bo_list(fd: RawFd, list_handle: u32) -> DriverResult<()> {
    let mut req = AmdgpuBoListIn {
        operation: AMDGPU_BO_LIST_OP_DESTROY,
        list_handle,
        ..Default::default()
    };
    amd_ioctl(
        fd,
        amd_iowr::<AmdgpuBoListIn>(DRM_AMDGPU_BO_LIST),
        &mut req,
        "AMDGPU_BO_LIST_DESTROY",
    )
}

/// Submit a command buffer (indirect buffer) to the GFX ring.
///
/// Returns a fence handle for synchronization.
///
/// # Errors
///
/// Returns [`DriverError`] if the CS submission ioctl fails.
pub fn submit_command(
    fd: RawFd,
    ctx_id: u32,
    bo_list: u32,
    ib_gpu_addr: u64,
    ib_size_bytes: u32,
) -> DriverResult<u64> {
    submit_command_ip(fd, ctx_id, bo_list, ib_gpu_addr, ib_size_bytes, AMDGPU_HW_IP_GFX)
}

/// Submit a command buffer to a specific IP type.
///
/// `ip_type`: `AMDGPU_HW_IP_GFX` (0) or `AMDGPU_HW_IP_COMPUTE` (1).
///
/// Returns a fence handle for synchronization.
///
/// # Errors
///
/// Returns [`DriverError`] if the CS submission ioctl fails.
pub fn submit_command_ip(
    fd: RawFd,
    ctx_id: u32,
    bo_list: u32,
    ib_gpu_addr: u64,
    ib_size_bytes: u32,
    ip_type: u32,
) -> DriverResult<u64> {
    let ring = 0u32;
    let ib = AmdgpuCsChunkIb {
        va_start: ib_gpu_addr,
        ib_bytes: ib_size_bytes,
        ip_type,
        ring,
        ..Default::default()
    };

    let chunk = AmdgpuCsChunk {
        chunk_id: AMDGPU_CHUNK_ID_IB,
        length_dw: size_of_u32::<AmdgpuCsChunkIb>() / 4,
        chunk_data: kernel_ptr(&ib),
    };

    let chunk_ptr = kernel_ptr(&chunk);

    let mut cs_in = AmdgpuCsIn {
        ctx_id,
        bo_list_handle: bo_list,
        num_chunks: 1,
        chunks: chunk_ptr,
        ..Default::default()
    };

    amd_ioctl_read(
        fd,
        amd_iowr::<AmdgpuCsIn>(DRM_AMDGPU_CS),
        &mut cs_in,
        "AMDGPU_CS",
    )
}

/// Wait for a CS fence to signal (GFX ring).
///
/// # Errors
///
/// Returns [`DriverError`] if the wait ioctl fails.
pub fn sync_fence(fd: RawFd, ctx_id: u32, fence_handle: u64, timeout_ns: u64) -> DriverResult<()> {
    sync_fence_ip(fd, ctx_id, fence_handle, timeout_ns, AMDGPU_HW_IP_GFX)
}

/// Wait for a CS fence on a specific IP type.
///
/// # Errors
///
/// Returns [`DriverError`] if the wait ioctl fails.
pub fn sync_fence_ip(
    fd: RawFd,
    ctx_id: u32,
    fence_handle: u64,
    timeout_ns: u64,
    ip_type: u32,
) -> DriverResult<()> {
    let ring = 0u32;
    let abs_timeout = clock_monotonic_ns().saturating_add(timeout_ns);
    let mut req = AmdgpuWaitCsIn {
        handle: fence_handle,
        timeout: abs_timeout,
        ip_type,
        ip_instance: 0,
        ring,
        ctx_id,
    };
    let status: u64 = amd_ioctl_read(
        fd,
        amd_iowr::<AmdgpuWaitCsIn>(DRM_AMDGPU_WAIT_CS),
        &mut req,
        "AMDGPU_WAIT_CS",
    )?;
    if status != 0 {
        return Err(crate::error::DriverError::FenceTimeout {
            ms: timeout_ns / 1_000_000,
        });
    }
    Ok(())
}

/// Query the GFX hardware IP version (major, minor).
///
/// Returns `(major, minor)` — e.g. `(9, 0)` for Vega, `(10, 3)` for RDNA2.
///
/// # Errors
///
/// Returns [`DriverError`] if the info ioctl fails.
pub fn query_gfx_version(fd: RawFd) -> DriverResult<(u32, u32)> {
    let mut response = AmdgpuInfoHwIp::default();
    let resp_ptr = std::ptr::from_mut(&mut response) as u64;
    let mut req = AmdgpuInfoRequestRaw {
        return_pointer: resp_ptr,
        return_size: size_of_u32::<AmdgpuInfoHwIp>(),
        query: AMDGPU_INFO_HW_IP_INFO,
        hw_ip_type: AMDGPU_HW_IP_GFX,
        hw_ip_instance: 0,
        pad: [0; 2],
    };
    amd_ioctl(
        fd,
        crate::drm::drm_iowr_pub(DRM_AMDGPU_INFO, size_of_u32::<AmdgpuInfoRequestRaw>()),
        &mut req,
        "AMDGPU_INFO_HW_IP_INFO",
    )?;
    Ok((response.hw_ip_version_major, response.hw_ip_version_minor))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_of_u32_helper() {
        assert_eq!(size_of_u32::<AmdgpuGemCreate>(), 32);
        assert_eq!(size_of_u32::<AmdgpuCtx>(), 16);
        assert_eq!(size_of_u32::<AmdgpuGemMmap>(), 8);
        assert_eq!(size_of_u32::<AmdgpuGemVa>(), 40);
        assert_eq!(size_of_u32::<AmdgpuBoListIn>(), 24);
        assert_eq!(size_of_u32::<AmdgpuCsIn>(), 24);
        assert_eq!(size_of_u32::<AmdgpuWaitCsIn>(), 32);
    }

    #[test]
    fn read_ioctl_output_extracts_first_field() {
        let cs = AmdgpuCsIn {
            ctx_id: 0xDEAD_BEEF,
            bo_list_handle: 0xCAFE,
            ..Default::default()
        };
        let out: u32 = read_ioctl_output(&cs);
        assert_eq!(out, 0xDEAD_BEEF);
    }

    #[test]
    fn kernel_ptr_round_trips() {
        let val: u32 = 42;
        let ptr = kernel_ptr(&val);
        assert_eq!(ptr, std::ptr::from_ref(&val) as u64);
    }
}
