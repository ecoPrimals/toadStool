// SPDX-License-Identifier: AGPL-3.0-only
//! Page flip and `VSync` support.
//!
//! Provides double-buffered page flipping for tear-free output.
//! Essential for data transport: each page flip pushes one "frame" of encoded
//! data to the display connector at the native refresh rate.

use crate::{DisplayError, Result};
use drm::control::{Device as ControlDevice, PageFlipFlags, PageFlipTarget};

use super::buffer::DumbBuffer;
use super::modesetting::ModesetPipeline;

/// Double-buffered page flipper.
///
/// Maintains two framebuffers and alternates between them on each flip,
/// allowing writes to the back buffer while the front buffer is being
/// scanned out.
pub struct PageFlipper {
    /// Front buffer framebuffer handle (currently displayed).
    front_fb: drm::control::framebuffer::Handle,
    /// Back buffer framebuffer handle (being written to).
    back_fb: drm::control::framebuffer::Handle,
    /// CRTC driving the output.
    crtc: drm::control::crtc::Handle,
    /// Whether the front and back are swapped.
    swapped: bool,
}

impl PageFlipper {
    /// Create a page flipper from a modeset pipeline and a second buffer.
    ///
    /// `pipeline` provides the first framebuffer (front). `back_buffer` is
    /// attached as the second framebuffer.
    ///
    /// # Errors
    ///
    /// Returns an error if the back buffer cannot be attached as a framebuffer.
    pub fn new(
        device: &super::Device,
        pipeline: &ModesetPipeline,
        back_buffer: &DumbBuffer,
    ) -> Result<Self> {
        let back_fb = device
            .add_framebuffer(back_buffer.inner(), 32, 32)
            .map_err(|e| DisplayError::IoctlFailed(format!("add_framebuffer (back): {e}")))?;

        Ok(Self {
            front_fb: pipeline.fb,
            back_fb,
            crtc: pipeline.crtc,
            swapped: false,
        })
    }

    /// The framebuffer handle for the buffer currently being written to (back).
    #[must_use]
    pub fn back_fb(&self) -> drm::control::framebuffer::Handle {
        if self.swapped {
            self.front_fb
        } else {
            self.back_fb
        }
    }

    /// Flip: swap front and back, requesting the CRTC to display the new front
    /// at the next `VSync`.
    ///
    /// Uses `page_flip` which is non-blocking; the actual flip happens at `VSync`.
    /// Returns immediately. For synchronous behaviour, poll for the flip event
    /// on the DRM fd afterward.
    ///
    /// # Errors
    ///
    /// Returns an error if the page flip ioctl fails.
    pub fn flip(&mut self, device: &super::Device) -> Result<()> {
        let new_front = self.back_fb();

        device
            .page_flip(
                self.crtc,
                new_front,
                PageFlipFlags::EVENT,
                None::<PageFlipTarget>,
            )
            .map_err(|e| DisplayError::IoctlFailed(format!("page_flip: {e}")))?;

        self.swapped = !self.swapped;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn flipper_size() {
        assert!(std::mem::size_of::<super::PageFlipper>() > 0);
    }
}
