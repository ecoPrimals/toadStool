// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests extracted from drm.rs (S334).

use super::drm::*;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::unix::io::AsRawFd;

fn temp_mmap_file(size: usize) -> (File, std::path::PathBuf) {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("cylinder_drm_mmap_test_{unique}"));
    let mut f = File::create(&path).expect("create temp file");
    f.write_all(&vec![0u8; size]).expect("write temp file");
    f.sync_all().expect("sync temp file");
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .expect("reopen temp file");
    (file, path)
}

#[test]
fn ioctl_numbers_are_consistent() {
    assert_eq!(DRM_IOCTL_VERSION & 0xFF, 0);
}

#[test]
fn drm_iowr_pub_constructs_valid_ioctl() {
    assert_eq!(drm_iowr_pub(0x00, 32), DRM_IOCTL_VERSION);
}

#[test]
fn drm_iow_pub_constructs_valid_ioctl() {
    assert_eq!(drm_iow_pub(0x09, 8), DRM_IOCTL_GEM_CLOSE);
}

#[test]
fn drm_gem_close_struct_size() {
    assert_eq!(std::mem::size_of::<DrmGemClose>(), 8);
}

#[test]
fn mapped_region_zero_length_fails() {
    let file = File::open("/dev/zero").unwrap();
    let fd = file.as_raw_fd();
    let result = MappedRegion::new(0, false, fd, 0);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("mmap length must be > 0")
    );
}

#[test]
fn mapped_region_slice_at_out_of_bounds() {
    let (file, path) = temp_mmap_file(4096);
    let fd = file.as_raw_fd();
    let region = MappedRegion::new(4096, true, fd, 0).unwrap();
    let result = region.slice_at(0, 4097);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("out of bounds"));
    let _ = std::fs::remove_file(path);
}

#[test]
fn mapped_region_slice_at_overflow() {
    let (file, path) = temp_mmap_file(4096);
    let fd = file.as_raw_fd();
    let region = MappedRegion::new(4096, true, fd, 0).unwrap();
    let result = region.slice_at(usize::MAX, 1);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("overflow"));
    let _ = std::fs::remove_file(path);
}

#[test]
fn mapped_region_slice_at_mut_out_of_bounds() {
    let (file, path) = temp_mmap_file(4096);
    let fd = file.as_raw_fd();
    let mut region = MappedRegion::new(4096, true, fd, 0).unwrap();
    let result = region.slice_at_mut(4090, 100);
    assert!(result.is_err());
    let _ = std::fs::remove_file(path);
}

#[test]
fn drm_version_struct_layout() {
    assert_eq!(std::mem::size_of::<DrmVersion>(), 64);
}

#[test]
fn drm_version_parsing_trim_nul() {
    let mut name_buf = [0u8; 64];
    name_buf[..6].copy_from_slice(b"amdgpu");
    let ver = DrmVersion {
        name_len: 6,
        ..Default::default()
    };
    let len = usize::try_from(ver.name_len)
        .unwrap_or(name_buf.len())
        .min(name_buf.len());
    let name = String::from_utf8_lossy(&name_buf[..len])
        .trim_end_matches('\0')
        .to_string();
    assert_eq!(name, "amdgpu");
}

#[test]
fn drm_render_node_index_range() {
    assert_eq!(DRI_RENDER_FIRST, 128);
    assert_eq!(DRI_RENDER_LAST, 191);
}

#[test]
fn drm_device_not_found_on_invalid_path() {
    assert!(DrmDevice::open("/dev/dri/renderD999").is_err());
}

#[test]
fn enumerate_render_nodes_returns_vec() {
    let nodes = enumerate_render_nodes();
    for info in &nodes {
        assert!(!info.path.is_empty());
        assert!(!info.driver.is_empty());
    }
}

#[test]
fn drm_device_info_has_driver_and_path() {
    let info = DrmDeviceInfo {
        path: "/dev/dri/renderD128".to_string(),
        driver: "amdgpu".to_string(),
        version_major: 3,
        version_minor: 57,
    };
    let debug = format!("{info:?}");
    assert!(debug.contains("amdgpu"));
    assert!(debug.contains("renderD128"));
}

#[test]
fn open_by_driver_nonexistent_fails() {
    assert!(DrmDevice::open_by_driver("nonexistent_drm_driver_xyz").is_err());
}

#[test]
fn drm_gem_close_default() {
    let close = DrmGemClose::default();
    assert_eq!(close.handle, 0);
    assert_eq!(close.pad, 0);
}

#[test]
fn mapped_region_slice_at_valid_range() {
    let (file, path) = temp_mmap_file(4096);
    let fd = file.as_raw_fd();
    let region = MappedRegion::new(4096, true, fd, 0).unwrap();
    let slice = region.slice_at(0, 256).unwrap();
    assert_eq!(slice.len(), 256);
    let _ = std::fs::remove_file(path);
}
