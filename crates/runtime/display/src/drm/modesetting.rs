// SPDX-License-Identifier: AGPL-3.0-or-later
//! DRM modesetting — CRTC allocation, framebuffer attachment, and mode configuration.
//!
//! Binds a dumb buffer to a CRTC and connector so pixels actually reach a physical
//! display output (HDMI, `DisplayPort`, etc.).

use crate::{DisplayError, Result};
use drm::control::Device as ControlDevice;

use super::buffer::DumbBuffer;
use super::connector::{ConnectorInfo, DisplayMode};

/// An active modesetting pipeline: connector -> encoder -> CRTC -> framebuffer.
#[allow(
    dead_code,
    reason = "DRM modesetting pipeline; used when display hardware is available"
)]
pub struct ModesetPipeline {
    /// The CRTC driving this pipeline.
    pub(crate) crtc: drm::control::crtc::Handle,
    /// The framebuffer ID attached to the CRTC.
    pub(crate) fb: drm::control::framebuffer::Handle,
    /// The connector being driven.
    pub(crate) connector: drm::control::connector::Handle,
    /// Active mode (resolution + refresh).
    pub(crate) mode: drm::control::Mode,
    /// Width in pixels.
    pub width: u16,
    /// Height in pixels.
    pub height: u16,
    /// Refresh rate in Hz.
    pub refresh_hz: u16,
}

/// Find a free CRTC that can drive the given connector.
fn find_crtc_for_connector(
    device: &super::Device,
    connector: &ConnectorInfo,
) -> Result<drm::control::crtc::Handle> {
    let resources = device
        .resource_handles()
        .map_err(|e| DisplayError::IoctlFailed(format!("resource_handles: {e}")))?;

    // If the connector already has an encoder, try its CRTC first.
    if let Some(enc_handle) = connector.encoder
        && let Ok(encoder) = device.get_encoder(enc_handle)
        && let Some(crtc) = encoder.crtc()
    {
        return Ok(crtc);
    }

    // Walk all encoders that can serve this connector and find a usable CRTC.
    let conn_info = device
        .get_connector(connector.handle, false)
        .map_err(|e| DisplayError::IoctlFailed(format!("get_connector: {e}")))?;

    for &enc_handle in conn_info.encoders() {
        if let Ok(encoder) = device.get_encoder(enc_handle)
            && let Some(&crtc) = resources.filter_crtcs(encoder.possible_crtcs()).first()
        {
            return Ok(crtc);
        }
    }

    Err(DisplayError::IoctlFailed(
        "no suitable CRTC found for connector".to_string(),
    ))
}

/// Set up a full modesetting pipeline: create framebuffer, pick CRTC, apply mode.
///
/// After this call, the contents of `buffer` are scanned out to the physical connector
/// at the chosen mode's resolution and refresh rate.
///
/// # Errors
///
/// Returns an error if no suitable CRTC is found, framebuffer creation fails, or `set_crtc` fails.
pub fn modeset(
    device: &super::Device,
    connector: &ConnectorInfo,
    mode: &DisplayMode,
    buffer: &DumbBuffer,
) -> Result<ModesetPipeline> {
    let crtc = find_crtc_for_connector(device, connector)?;

    // Attach the dumb buffer as a DRM framebuffer via the inner drm::control::DumbBuffer.
    let fb = device
        .add_framebuffer(buffer.inner(), 32, 32)
        .map_err(|e| DisplayError::IoctlFailed(format!("add_framebuffer: {e}")))?;

    // Apply: CRTC -> encoder -> connector with the selected mode.
    device
        .set_crtc(
            crtc,
            Some(fb),
            (0, 0),
            &[connector.handle],
            Some(mode.inner),
        )
        .map_err(|e| DisplayError::IoctlFailed(format!("set_crtc: {e}")))?;

    Ok(ModesetPipeline {
        crtc,
        fb,
        connector: connector.handle,
        mode: mode.inner,
        width: mode.width,
        height: mode.height,
        refresh_hz: mode.refresh_hz,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn pipeline_fields() {
        assert!(std::mem::size_of::<super::ModesetPipeline>() > 0);
    }
}
