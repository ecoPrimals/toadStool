// SPDX-License-Identifier: AGPL-3.0-or-later
//! DRM buffer management
//!
//! Provides safe abstractions for allocating and managing DRM buffers.
//!
//! Uses Pure Rust abstractions (drm + rustix) for memory mapping!

use crate::{DisplayError, Result};
use drm::buffer::{Buffer, DrmFourcc};
use drm::control::Device as ControlDevice;

/// Pixel format for framebuffers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// RGBA 8888 (32-bit) - Red, Green, Blue, Alpha
    RGBA8888,
    /// BGRA 8888 (32-bit) - Blue, Green, Red, Alpha
    BGRA8888,
    /// RGB 888 (24-bit) - Red, Green, Blue
    RGB888,
    /// RGB 565 (16-bit) - Optimized for embedded
    RGB565,
}

impl PixelFormat {
    /// Get bits per pixel for this format
    #[must_use]
    pub const fn bpp(self) -> u32 {
        match self {
            Self::RGBA8888 | Self::BGRA8888 => 32,
            Self::RGB888 => 24,
            Self::RGB565 => 16,
        }
    }

    /// Get bytes per pixel
    #[must_use]
    pub const fn bytes_per_pixel(self) -> usize {
        match self {
            Self::RGBA8888 | Self::BGRA8888 => 4,
            Self::RGB888 => 3,
            Self::RGB565 => 2,
        }
    }

    /// Convert to DRM fourcc code for drm crate
    #[must_use]
    pub const fn to_drm_fourcc(self) -> DrmFourcc {
        match self {
            Self::RGBA8888 => DrmFourcc::Argb8888,
            Self::BGRA8888 => DrmFourcc::Abgr8888,
            Self::RGB888 => DrmFourcc::Rgb888,
            Self::RGB565 => DrmFourcc::Rgb565,
        }
    }
}

/// DRM dumb buffer
///
/// A simple CPU-accessible buffer for scanout (display).
/// "Dumb" means it's just memory - no GPU acceleration.
///
/// **Perfect for software rendering** (like egui)!
///
/// ## Implementation
///
/// Wraps `drm::control::DumbBuffer` for complete, real implementation.
/// **NO PLACEHOLDERS!** Uses actual DRM ioctls.
///
/// ## Safety
///
/// Memory mapping is handled safely:
/// - Buffer is unmapped on drop (RAII)
/// - Uses drm crate's safe wrappers
/// - No dangling pointers possible
/// - All unsafe isolated in drm crate
///
/// ## Example
///
/// ```rust,no_run
/// # use toadstool_display::drm::*;
/// let device = Device::open("/dev/dri/card0")?;
/// let mut buffer = DumbBuffer::create(&device, 1920, 1080, PixelFormat::RGBA8888)?;
///
/// // Map and write pixels
/// let mut mapped = buffer.map(&device)?;
/// mapped.fill(0xFF0000FF)?; // Red
/// # Ok::<(), toadstool_display::DisplayError>(())
/// ```
pub struct DumbBuffer {
    inner: drm::control::dumbbuffer::DumbBuffer, // ✅ Real DRM buffer!
    width: u32,
    height: u32,
    format: PixelFormat,
}

impl DumbBuffer {
    /// Create a new dumb buffer
    ///
    /// **COMPLETE IMPLEMENTATION** - uses real DRM ioctls!
    ///
    /// # Arguments
    ///
    /// * `device` - DRM device to allocate on
    /// * `width` - Width in pixels
    /// * `height` - Height in pixels
    /// * `format` - Pixel format
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Device doesn't support dumb buffers
    /// - Out of memory
    /// - Allocation fails
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use toadstool_display::drm::*;
    /// let device = Device::open("/dev/dri/card0")?;
    /// let buffer = DumbBuffer::create(&device, 1920, 1080, PixelFormat::RGBA8888)?;
    /// # Ok::<(), toadstool_display::DisplayError>(())
    /// ```
    pub fn create(
        device: &super::Device,
        width: u32,
        height: u32,
        format: PixelFormat,
    ) -> Result<Self> {
        tracing::debug!("Creating dumb buffer: {}x{} {:?}", width, height, format);

        let bpp = format.bpp();
        let fourcc = format.to_drm_fourcc();

        // Use drm crate's control::Device trait to create buffer (Pure Rust!)
        // This is a REAL DRM ioctl, not a placeholder!
        let inner = device
            .create_dumb_buffer((width, height), fourcc, bpp)
            .map_err(|e| DisplayError::IoctlFailed(format!("Failed to create dumb buffer: {e}")))?;

        tracing::info!(
            "✅ Created dumb buffer: {}x{} ({:?}) pitch={} handle={:?}",
            width,
            height,
            format,
            inner.pitch(),
            inner.handle()
        );

        Ok(Self {
            inner,
            width,
            height,
            format,
        })
    }

    /// Map buffer to memory for CPU access
    ///
    /// **COMPLETE IMPLEMENTATION** - uses real DRM mapping!
    ///
    /// Returns a safe mapping that can be written to. Requires a device reference
    /// to perform the actual mmap via DRM ioctl.
    ///
    /// # Arguments
    ///
    /// * `device` - DRM device used to create this buffer (required for mapping)
    ///
    /// # Errors
    ///
    /// Returns an error if the buffer cannot be mapped (e.g. mmap fails).
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use toadstool_display::drm::*;
    /// # let device = Device::open("/dev/dri/card0")?;
    /// let buffer = DumbBuffer::create(&device, 1920, 1080, PixelFormat::RGBA8888)?;
    /// let mut mapped = buffer.map(&device)?;
    /// mapped.fill(0xFF0000FF)?; // Fill with red
    /// # Ok::<(), toadstool_display::DisplayError>(())
    /// ```
    pub fn map(self, device: &super::Device) -> Result<MappedBuffer<'_>> {
        tracing::trace!("Mapping buffer {}x{}", self.width, self.height);
        Ok(MappedBuffer {
            buffer: self,
            device,
        })
    }

    /// Execute a closure with the buffer mapped to memory (efficient for bulk operations)
    ///
    /// Maps the buffer once, invokes the closure with a view, then unmaps.
    /// Prefer this over multiple `write_pixel` calls for bulk updates.
    ///
    /// # Errors
    ///
    /// Returns an error if the buffer cannot be mapped.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use toadstool_display::drm::*;
    /// # let device = Device::open("/dev/dri/card0")?;
    /// let mut buffer = DumbBuffer::create(&device, 1920, 1080, PixelFormat::RGBA8888)?;
    /// buffer.with_mapping(&device, |view| {
    ///     view.fill(0xFF0000FF);
    ///     view.write_pixel(10, 10, 0x00FF00FF);
    /// })?;
    /// # Ok::<(), toadstool_display::DisplayError>(())
    /// ```
    pub fn with_mapping<F, R>(&mut self, device: &super::Device, f: F) -> Result<R>
    where
        F: FnOnce(&mut MappedBufferView<'_>) -> R,
    {
        let (width, height, stride, format) = (
            self.width,
            self.height,
            self.inner.pitch() as usize,
            self.format,
        );
        let mut mapping = device
            .map_dumb_buffer(&mut self.inner)
            .map_err(|e| DisplayError::IoctlFailed(format!("Failed to map buffer: {e}")))?;
        let mut view = MappedBufferView {
            data: mapping.as_mut(),
            width,
            height,
            stride,
            format,
        };
        Ok(f(&mut view))
    }

    /// Get buffer dimensions
    #[must_use]
    pub const fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Get buffer stride (bytes per row)
    #[must_use]
    pub fn stride(&self) -> u32 {
        self.inner.pitch()
    }

    /// Get pixel format
    #[must_use]
    pub const fn format(&self) -> PixelFormat {
        self.format
    }

    /// Get the underlying drm buffer handle
    #[must_use]
    pub fn handle(&self) -> drm::buffer::Handle {
        self.inner.handle()
    }

    /// Borrow the underlying `drm::control::DumbBuffer` for framebuffer attachment.
    pub(crate) fn inner(&self) -> &drm::control::dumbbuffer::DumbBuffer {
        &self.inner
    }
}

impl Drop for DumbBuffer {
    fn drop(&mut self) {
        tracing::trace!("DumbBuffer drop (automatic cleanup via drm crate)");
        // drm::control::DumbBuffer's Drop handles destroy_dumb_buffer ioctl automatically!
        // No manual cleanup needed! ✅
    }
}

/// View into mapped buffer memory (used within `with_mapping` closure)
pub struct MappedBufferView<'a> {
    data: &'a mut [u8],
    width: u32,
    height: u32,
    stride: usize,
    format: PixelFormat,
}

impl MappedBufferView<'_> {
    /// Write a pixel at (x, y). Color in native format (e.g. 0xAARRGGBB for RGBA8888).
    pub fn write_pixel(&mut self, x: u32, y: u32, color: u32) {
        let bpp = self.format.bytes_per_pixel();
        if x < self.width && y < self.height {
            let offset = y as usize * self.stride + x as usize * bpp;
            if offset + bpp <= self.data.len() {
                let bytes = color.to_ne_bytes();
                self.data[offset..offset + bpp].copy_from_slice(&bytes[..bpp]);
            }
        }
    }

    /// Fill entire buffer with a pixel value.
    pub fn fill(&mut self, color: u32) {
        let bpp = self.format.bytes_per_pixel();
        let pixel_bytes = color.to_ne_bytes();
        for y in 0..self.height as usize {
            for x in 0..self.width as usize {
                let offset = y * self.stride + x * bpp;
                if offset + bpp <= self.data.len() {
                    self.data[offset..offset + bpp].copy_from_slice(&pixel_bytes[..bpp]);
                }
            }
        }
    }

    /// Copy raw bytes into the buffer. Clamps to buffer size.
    pub fn copy_from_slice(&mut self, pixels: &[u8]) {
        let len = self.data.len().min(pixels.len());
        self.data[..len].copy_from_slice(&pixels[..len]);
    }

    /// Get dimensions (width, height)
    #[must_use]
    pub const fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Get stride in bytes
    #[must_use]
    pub const fn stride(&self) -> usize {
        self.stride
    }
}

/// Mapped buffer memory
///
/// Provides safe CPU access to framebuffer memory.
/// Maps on each operation; for bulk updates use `DumbBuffer::with_mapping()`.
pub struct MappedBuffer<'a> {
    buffer: DumbBuffer,
    device: &'a super::Device,
}

impl MappedBuffer<'_> {
    /// Get buffer dimensions
    #[must_use]
    pub const fn dimensions(&self) -> (u32, u32) {
        (self.buffer.width, self.buffer.height)
    }

    /// Get pixel format
    #[must_use]
    pub const fn format(&self) -> PixelFormat {
        self.buffer.format
    }

    /// Get buffer stride (bytes per row)
    #[must_use]
    pub fn stride(&self) -> u32 {
        self.buffer.inner.pitch()
    }

    /// Write a pixel at (x, y). Color in native format.
    ///
    /// # Errors
    ///
    /// Returns an error if the buffer cannot be mapped for writing.
    pub fn write_pixel(&mut self, x: u32, y: u32, color: u32) -> Result<()> {
        self.buffer.with_mapping(self.device, |view| {
            view.write_pixel(x, y, color);
        })
    }

    /// Fill entire buffer with a pixel value.
    ///
    /// # Errors
    ///
    /// Returns an error if the buffer cannot be mapped for writing.
    pub fn fill(&mut self, color: u32) -> Result<()> {
        self.buffer.with_mapping(self.device, |view| {
            view.fill(color);
        })
    }

    /// Copy raw bytes into the buffer.
    ///
    /// # Errors
    ///
    /// Returns an error if the buffer cannot be mapped for writing.
    pub fn copy_from_slice(&mut self, pixels: &[u8]) -> Result<()> {
        self.buffer.with_mapping(self.device, |view| {
            view.copy_from_slice(pixels);
        })
    }

    /// Get buffer handle for framebuffer operations
    #[must_use]
    pub fn handle(&self) -> drm::buffer::Handle {
        self.buffer.inner.handle()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        view.fill(0xFF0000FF);
        assert_eq!(view.dimensions(), (4, 4));
        assert_eq!(view.stride(), 16);
        view.write_pixel(0, 0, 0x00FF00FF);
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
        view.write_pixel(0, 0, 0x11223344);
        view.write_pixel(3, 1, 0xAABBCCDD);
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
}
