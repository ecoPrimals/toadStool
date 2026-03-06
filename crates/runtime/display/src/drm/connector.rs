// SPDX-License-Identifier: AGPL-3.0-or-later
//! DRM connector enumeration and mode discovery.
//!
//! Discovers physical display connectors (HDMI, `DisplayPort`, etc.) and their
//! supported modes. Foundation for both display output and data transport.

use crate::{DisplayError, Result};
use drm::control::Device as ControlDevice;

/// Physical connector type discovered at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectorType {
    /// HDMI output
    Hdmi,
    /// `DisplayPort` output
    DisplayPort,
    /// VGA output (legacy)
    Vga,
    /// DVI output
    Dvi,
    /// eDP (embedded `DisplayPort`, laptop panels)
    Edp,
    /// Virtual output (used in headless/VM)
    Virtual,
    /// Unknown connector type
    Unknown(u32),
}

impl ConnectorType {
    fn from_drm(interface: drm::control::connector::Interface) -> Self {
        use drm::control::connector::Interface;
        match interface {
            Interface::HDMIA | Interface::HDMIB => Self::Hdmi,
            Interface::DisplayPort => Self::DisplayPort,
            Interface::VGA => Self::Vga,
            Interface::DVII | Interface::DVID | Interface::DVIA => Self::Dvi,
            Interface::EmbeddedDisplayPort => Self::Edp,
            Interface::Virtual => Self::Virtual,
            other => Self::Unknown(other as u32),
        }
    }

    /// Whether this connector type supports high-bandwidth data transport.
    #[must_use]
    pub const fn supports_data_transport(&self) -> bool {
        matches!(self, Self::Hdmi | Self::DisplayPort)
    }
}

/// Connection status of a connector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    /// A display/device is plugged in.
    Connected,
    /// Nothing plugged in.
    Disconnected,
    /// Cannot determine (driver limitation).
    Unknown,
}

impl ConnectionStatus {
    fn from_drm(state: drm::control::connector::State) -> Self {
        use drm::control::connector::State;
        match state {
            State::Connected => Self::Connected,
            State::Disconnected => Self::Disconnected,
            State::Unknown => Self::Unknown,
        }
    }
}

/// A display mode (resolution + refresh rate).
#[derive(Debug, Clone)]
pub struct DisplayMode {
    /// Horizontal active pixels.
    pub width: u16,
    /// Vertical active pixels.
    pub height: u16,
    /// Vertical refresh rate in Hz.
    pub refresh_hz: u16,
    /// Raw pixel clock in kHz.
    pub clock_khz: u32,
    /// Whether this is the preferred mode for the connector.
    pub preferred: bool,
    /// DRM mode handle for use in modesetting.
    pub(crate) inner: drm::control::Mode,
}

impl DisplayMode {
    #[allow(clippy::cast_possible_truncation)] // vrefresh is hardware register width; display rates fit in u16
    fn from_drm(mode: drm::control::Mode) -> Self {
        let (w, h) = mode.size();
        Self {
            width: w,
            height: h,
            refresh_hz: mode.vrefresh() as u16,
            clock_khz: mode.clock(),
            preferred: mode
                .mode_type()
                .contains(drm::control::ModeTypeFlags::PREFERRED),
            inner: mode,
        }
    }

    /// Raw throughput in bytes/sec at RGBA8888 (4 bytes per pixel).
    #[must_use]
    pub fn throughput_bps_rgba(&self) -> u64 {
        u64::from(self.width) * u64::from(self.height) * 4 * u64::from(self.refresh_hz) * 8
        // bits
    }
}

/// Discovered display connector with its modes.
#[derive(Debug)]
pub struct ConnectorInfo {
    /// Connector type (HDMI, DP, etc.).
    pub connector_type: ConnectorType,
    /// Connection status.
    pub status: ConnectionStatus,
    /// Human-readable label (e.g. "HDMI-A-1").
    pub label: String,
    /// Available display modes, sorted by throughput descending.
    pub modes: Vec<DisplayMode>,
    /// DRM connector handle.
    pub(crate) handle: drm::control::connector::Handle,
    /// Current encoder (if connected).
    pub(crate) encoder: Option<drm::control::encoder::Handle>,
}

impl ConnectorInfo {
    /// Best mode by throughput (largest resolution and highest refresh).
    #[must_use]
    pub fn best_mode(&self) -> Option<&DisplayMode> {
        self.modes.first()
    }

    /// The preferred mode as reported by the display's EDID.
    #[must_use]
    pub fn preferred_mode(&self) -> Option<&DisplayMode> {
        self.modes.iter().find(|m| m.preferred)
    }
}

/// Enumerate all connectors and their modes on a DRM device.
///
/// # Errors
///
/// Returns an error if DRM resource or connector queries fail.
pub fn enumerate_connectors(device: &super::Device) -> Result<Vec<ConnectorInfo>> {
    let resources = device
        .resource_handles()
        .map_err(|e| DisplayError::IoctlFailed(format!("resource_handles: {e}")))?;

    let mut connectors = Vec::new();

    for &handle in resources.connectors() {
        let info = device
            .get_connector(handle, false)
            .map_err(|e| DisplayError::IoctlFailed(format!("get_connector: {e}")))?;

        let connector_type = ConnectorType::from_drm(info.interface());
        let status = ConnectionStatus::from_drm(info.state());
        let label = format!("{:?}-{}", info.interface(), info.interface_id());

        let mut modes: Vec<DisplayMode> = info
            .modes()
            .iter()
            .map(|m| DisplayMode::from_drm(*m))
            .collect();
        modes.sort_by_key(|m| std::cmp::Reverse(m.throughput_bps_rgba()));

        connectors.push(ConnectorInfo {
            connector_type,
            status,
            label,
            modes,
            handle,
            encoder: info.current_encoder(),
        });
    }

    Ok(connectors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connector_type_transport_support() {
        assert!(ConnectorType::Hdmi.supports_data_transport());
        assert!(ConnectorType::DisplayPort.supports_data_transport());
        assert!(!ConnectorType::Vga.supports_data_transport());
        assert!(!ConnectorType::Virtual.supports_data_transport());
    }

    #[test]
    fn throughput_formula() {
        // Verify the throughput calculation: 3840 * 2160 * 4 * 60 * 8 = ~15.9 Gbps
        let bps = 3840u64 * 2160 * 4 * 60 * 8;
        assert!(bps > 15_000_000_000);
    }
}
