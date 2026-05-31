// SPDX-License-Identifier: AGPL-3.0-or-later
//! Sovereign BAR0 MMIO access — direct GPU register read/write via sysfs.
//!
//! Maps `/sys/class/drm/{node}/device/resource0` (or an explicit PCI sysfs
//! path) to perform volatile 32-bit register operations. This is the same
//! physical BAR0 window used by ecosystem PMU/init tooling.
//!
//! Requires root or appropriate PCI sysfs permissions.
//!
//! # Safety model
//!
//! BAR0 writes affect real hardware state. Incorrect register writes can
//! hang the GPU, corrupt display, or require a reboot. This module is used
//! exclusively for well-known init sequences parsed from NVIDIA firmware
//! blobs by the `gsp::firmware_parser` module.

use crate::error::DriverError;
use crate::vfio::device::{ApplyError, RegisterAccess};
use std::path::Path;
use toadstool_hw_safe::SafeMmapRegion;

#[cfg(test)]
use crate::mmio_region::MmioRegion;

enum Bar0Backing {
    Sysfs(SafeMmapRegion),
    #[cfg(test)]
    Test(MmioRegion),
}

/// GPU BAR0 MMIO mapping for direct register access.
///
/// Wraps an mmap of the PCI BAR0 resource file. All reads and writes are
/// volatile, matching hardware MMIO semantics.
///
/// ## Thread safety (`Send` / `Sync`)
///
/// The mapping is owned together with the open `resource0` [`std::fs::File`]; the
/// kernel keeps the mmap valid for the file’s lifetime. Volatile MMIO accesses
/// are performed through hw-safe and are safe to use
/// across threads for aligned 32-bit operations when hardware access ordering is
/// respected by callers—matching other BAR0 wrappers in this crate.
pub struct Bar0Access {
    backing: Bar0Backing,
}

impl std::fmt::Debug for Bar0Access {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bar0Access")
            .field("size", &self.size())
            .finish_non_exhaustive()
    }
}

impl Bar0Access {
    fn read_u32_at(&self, off: usize) -> Result<u32, DriverError> {
        match &self.backing {
            Bar0Backing::Sysfs(m) => m
                .as_volatile()
                .read_u32(off)
                .map_err(|e| DriverError::MmapFailed(std::borrow::Cow::Owned(e.to_string()))),
            #[cfg(test)]
            Bar0Backing::Test(r) => r.read_u32(off),
        }
    }

    fn write_u32_at(&mut self, off: usize, value: u32) -> Result<(), DriverError> {
        match &mut self.backing {
            Bar0Backing::Sysfs(m) => m
                .as_volatile()
                .write_u32(off, value)
                .map_err(|e| DriverError::MmapFailed(std::borrow::Cow::Owned(e.to_string()))),
            #[cfg(test)]
            Bar0Backing::Test(r) => r.write_u32(off, value),
        }
    }
    /// Open BAR0 from a DRM render node path (e.g. `/dev/dri/renderD128`).
    ///
    /// Resolves the sysfs device directory and maps `resource0`.
    ///
    /// # Errors
    ///
    /// Returns `ApplyError::MmioFailed` if the render node path cannot be parsed
    /// or if opening/mapping the BAR0 resource fails.
    pub fn from_render_node(render_node_path: &str) -> Result<Self, ApplyError> {
        let node_name =
            render_node_path
                .rsplit('/')
                .next()
                .ok_or_else(|| ApplyError::MmioFailed {
                    offset: 0,
                    detail: format!("cannot parse render node from '{render_node_path}'"),
                })?;
        let sysfs_device = crate::linux_paths::sysfs_class_drm_device(node_name);
        Self::from_sysfs_device(&sysfs_device)
    }

    /// Open BAR0 from a sysfs device directory (e.g. `/sys/class/drm/renderD128/device`).
    ///
    /// # Errors
    ///
    /// Returns `ApplyError::MmioFailed` if opening/mapping the BAR0 resource fails,
    /// or if the device is held by a live ember instance.
    pub fn from_sysfs_device(sysfs_device: &str) -> Result<Self, ApplyError> {
        #[cfg(feature = "vfio")]
        if let Some(bdf) = bdf_from_sysfs_device(sysfs_device)
            && crate::vfio::ember_gate::is_device_held_by_ember(&bdf)
        {
            return Err(ApplyError::MmioFailed {
                offset: 0,
                detail: format!(
                    "device {bdf} is held by ember — use EmberSession::connect() instead of direct BAR0 open"
                ),
            });
        }
        let path = format!("{sysfs_device}/resource0");
        Self::open_resource(&path)
    }

    /// Open BAR0 from an explicit resource file path.
    fn open_resource(path: &str) -> Result<Self, ApplyError> {
        let mmap = SafeMmapRegion::map_shared_rw(Path::new(path)).map_err(|e| {
            ApplyError::MmioFailed {
                offset: 0,
                detail: format!("mmap {path}: {e}"),
            }
        })?;

        let size = mmap.size();
        tracing::info!(
            path,
            size_mib = size / (1024 * 1024),
            "BAR0 MMIO mapped for sovereign register access"
        );

        Ok(Self {
            backing: Bar0Backing::Sysfs(mmap),
        })
    }

    /// Construct BAR0 access from a heap-backed [`MmioRegion`] for unit tests.
    #[cfg(test)]
    pub(crate) fn from_mmio_region_for_test(region: MmioRegion) -> Self {
        Self {
            backing: Bar0Backing::Test(region),
        }
    }

    /// BAR0 mapping size in bytes.
    #[must_use]
    pub fn size(&self) -> usize {
        match &self.backing {
            Bar0Backing::Sysfs(m) => m.size(),
            #[cfg(test)]
            Bar0Backing::Test(r) => r.len(),
        }
    }

    /// Read a GPU identification register (`NV_PMC_BOOT_0` at offset 0x0).
    ///
    /// Returns the chip ID word. Useful for verifying BAR0 access works.
    ///
    /// # Errors
    ///
    /// Returns `ApplyError::MmioFailed` if the read fails.
    pub fn read_boot_id(&self) -> Result<u32, ApplyError> {
        self.read_u32(0)
    }
}

impl RegisterAccess for Bar0Access {
    fn read_u32(&self, offset: u32) -> Result<u32, ApplyError> {
        let off = offset as usize;
        self.read_u32_at(off)
            .map_err(|e: DriverError| ApplyError::MmioFailed {
                offset,
                detail: e.to_string(),
            })
    }

    fn write_u32(&mut self, offset: u32, value: u32) -> Result<(), ApplyError> {
        let off = offset as usize;
        self.write_u32_at(off, value)
            .map_err(|e: DriverError| ApplyError::MmioFailed {
                offset,
                detail: e.to_string(),
            })
    }
}

// SAFETY: `MmioRegion` wraps `NonNull<u8>`, which is not `Send` on its own.
// The sysfs `resource0` mmap stays valid when moved: the open `File` pins the
// mapping and the kernel keeps MMIO pages mapped until `munmap` on drop.
unsafe impl Send for Bar0Access {}

// SAFETY: Shared `&Bar0Access` only exposes volatile aligned `u32` MMIO via
// `&self`; no aliased Rust `&mut` to the mapped bytes. Concurrent MMIO ordering
// is the caller's responsibility, matching other BAR0 wrappers in this crate.
unsafe impl Sync for Bar0Access {}

/// Extract a PCI BDF from a sysfs device directory path.
///
/// Follows the sysfs symlink (e.g. `/sys/class/drm/renderD128/device` ->
/// `/sys/devices/.../0000:06:00.0`) and returns the final BDF component.
/// Returns `None` if the path can't be resolved or doesn't end in a BDF.
#[cfg(feature = "vfio")]
fn bdf_from_sysfs_device(sysfs_device: &str) -> Option<String> {
    let resolved = std::fs::canonicalize(sysfs_device).ok()?;
    let name = resolved.file_name()?.to_str()?;
    if name.contains(':') && name.contains('.') {
        Some(name.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mmio_region::MmioRegion;

    #[test]
    fn bar0_heap_region_read_boot_id_and_offset() {
        let mut backing = vec![0u8; 256].into_boxed_slice();
        let boot0 = 0x1720_00A1u32;
        backing[0..4].copy_from_slice(&boot0.to_le_bytes());
        backing[16..20].copy_from_slice(&0xCAFE_BABEu32.to_le_bytes());
        let region = MmioRegion::from_heap_slice_for_test(backing);
        let bar0 = Bar0Access::from_mmio_region_for_test(region);
        assert_eq!(bar0.size(), 256);
        assert_eq!(bar0.read_boot_id().expect("BOOT0"), boot0);
        assert_eq!(bar0.read_u32(16).expect("off 16"), 0xCAFE_BABE);
    }

    #[test]
    fn bar0_heap_region_read_oob_reports_offset() {
        let backing = vec![0u8; 8].into_boxed_slice();
        let region = MmioRegion::from_heap_slice_for_test(backing);
        let bar0 = Bar0Access::from_mmio_region_for_test(region);
        match bar0.read_u32(8) {
            Err(ApplyError::MmioFailed { offset, detail }) => {
                assert_eq!(offset, 8);
                assert!(
                    detail.contains("out of range") || detail.contains("MMIO read"),
                    "unexpected detail: {detail}"
                );
            }
            other => panic!("expected MmioFailed, got {other:?}"),
        }
    }

    #[test]
    fn bar0_nonexistent_path_fails() {
        let result = Bar0Access::from_render_node("/dev/dri/renderD999");
        assert!(result.is_err());
    }

    #[test]
    fn bar0_parse_node_name() {
        let result = Bar0Access::from_render_node("/dev/dri/renderD128");
        // Will fail without root/permissions, but should parse the path correctly
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("resource0") || err.contains("renderD128"),
            "error should reference the sysfs path: {err}"
        );
    }

    #[test]
    #[ignore = "requires root and NVIDIA GPU"]
    fn bar0_read_boot_id() {
        let bar0 =
            Bar0Access::from_render_node("/dev/dri/renderD128").expect("BAR0 access (needs root)");
        let boot_id = bar0.read_boot_id().expect("read NV_PMC_BOOT_0");
        tracing::debug!(boot_id = format!("{boot_id:#010x}"), "NV_PMC_BOOT_0");
        assert_ne!(boot_id, 0, "boot ID should not be zero");
        assert_ne!(
            boot_id, 0xFFFF_FFFF,
            "boot ID should not be all-ones (unmapped)"
        );
    }
}
