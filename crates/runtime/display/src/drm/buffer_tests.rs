// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests extracted from buffer.rs (S335).

use super::buffer::*;

#[test]
fn test_pixel_format_bpp() {
    assert_eq!(PixelFormat::RGBA8888.bpp(), 32);
    assert_eq!(PixelFormat::BGRA8888.bpp(), 32);
    assert_eq!(PixelFormat::RGB888.bpp(), 24);
    assert_eq!(PixelFormat::RGB565.bpp(), 16);
}

#[test]
fn test_pixel_format_bytes_per_pixel() {
    assert_eq!(PixelFormat::RGBA8888.bytes_per_pixel(), 4);
    assert_eq!(PixelFormat::BGRA8888.bytes_per_pixel(), 4);
    assert_eq!(PixelFormat::RGB888.bytes_per_pixel(), 3);
    assert_eq!(PixelFormat::RGB565.bytes_per_pixel(), 2);
}

#[test]
fn test_pixel_format_to_drm_fourcc() {
    use drm::buffer::DrmFourcc;
    assert_eq!(PixelFormat::RGBA8888.to_drm_fourcc(), DrmFourcc::Argb8888);
    assert_eq!(PixelFormat::BGRA8888.to_drm_fourcc(), DrmFourcc::Abgr8888);
    assert_eq!(PixelFormat::RGB888.to_drm_fourcc(), DrmFourcc::Rgb888);
    assert_eq!(PixelFormat::RGB565.to_drm_fourcc(), DrmFourcc::Rgb565);
}

#[test]
fn test_mapped_buffer_view_write_pixel_and_fill() {
    let mut data = vec![0u8; 16 * 4];
    let mut view = MappedBufferView {
        data: data.as_mut_slice(),
        width: 4,
        height: 4,
        stride: 16,
        format: PixelFormat::RGBA8888,
    };
    view.fill(0xFF_00_00_FF);
    assert_eq!(view.dimensions(), (4, 4));
    assert_eq!(view.stride(), 16);
    view.write_pixel(0, 0, 0x00_FF_00_FF);
}

#[test]
fn test_mapped_buffer_view_write_pixel_bounds() {
    let mut data = vec![0u8; 8 * 4];
    let mut view = MappedBufferView {
        data: data.as_mut_slice(),
        width: 4,
        height: 2,
        stride: 16,
        format: PixelFormat::RGBA8888,
    };
    view.write_pixel(0, 0, 0x11_22_33_44);
    view.write_pixel(3, 1, 0xAA_BB_CC_DD);
    assert_eq!(view.dimensions(), (4, 2));
}

#[test]
fn test_mapped_buffer_view_write_pixel_out_of_bounds_no_panic() {
    let mut data = vec![0u8; 16];
    let mut view = MappedBufferView {
        data: data.as_mut_slice(),
        width: 2,
        height: 2,
        stride: 8,
        format: PixelFormat::RGBA8888,
    };
    view.write_pixel(10, 10, 0xFF);
    view.write_pixel(2, 0, 0xFF);
    view.write_pixel(0, 2, 0xFF);
}

#[test]
fn test_mapped_buffer_view_copy_from_slice() {
    let mut data = vec![0u8; 64];
    let mut view = MappedBufferView {
        data: data.as_mut_slice(),
        width: 4,
        height: 4,
        stride: 16,
        format: PixelFormat::RGBA8888,
    };
    let pixels = vec![0x11u8; 32];
    view.copy_from_slice(&pixels);
    assert_eq!(&data[..32], &pixels[..]);
}

#[test]
fn test_mapped_buffer_view_copy_from_slice_clamps() {
    let mut data = vec![0u8; 16];
    let mut view = MappedBufferView {
        data: data.as_mut_slice(),
        width: 2,
        height: 2,
        stride: 8,
        format: PixelFormat::RGBA8888,
    };
    let large_slice = vec![0xFFu8; 1000];
    view.copy_from_slice(&large_slice);
    assert_eq!(data.len(), 16);
}

#[test]
fn test_mapped_buffer_view_rgb565_fill() {
    let mut data = vec![0u8; 8 * 2];
    let mut view = MappedBufferView {
        data: data.as_mut_slice(),
        width: 4,
        height: 2,
        stride: 8,
        format: PixelFormat::RGB565,
    };
    view.fill(0xFFFF);
    view.write_pixel(1, 1, 0x0000);
}
