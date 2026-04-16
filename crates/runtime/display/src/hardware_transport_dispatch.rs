// SPDX-License-Identifier: AGPL-3.0-or-later
//! Finite dispatch for [`HardwareTransport`](toadstool_core::HardwareTransport) implementations
//! in this crate — replaces `Box<dyn HardwareTransport>` at router boundaries.

use toadstool_core::{HardwareTransport, TransportError, TransportInfo};

use crate::capture_transport::CaptureTransport;
use crate::pcie_transport::PcieTransport;
use crate::transport::DisplayTransport;

#[cfg(feature = "serial-transport")]
use crate::serial_transport::SerialTransport;

/// Test-only loopback transport (see integration tests and router unit tests).
#[doc(hidden)]
pub struct TestLoopbackTransport {
    pub(crate) info: TransportInfo,
    pub(crate) buf: Vec<u8>,
    pub(crate) bandwidth_bps: u64,
}

impl TestLoopbackTransport {
    /// Creates a loopback transport with configurable bandwidth (for filter tests).
    #[must_use]
    pub fn new(
        id: &str,
        direction: toadstool_core::TransportDirection,
        bandwidth_bps: u64,
    ) -> Self {
        use toadstool_core::TransportMedium;
        Self {
            info: TransportInfo {
                id: id.to_string(),
                label: id.to_string(),
                medium: TransportMedium::Serial,
                direction,
            },
            buf: Vec::new(),
            bandwidth_bps,
        }
    }

    /// Same as [`Self::new`] with default 1 Mbps bandwidth.
    #[must_use]
    pub fn with_default_bandwidth(id: &str, direction: toadstool_core::TransportDirection) -> Self {
        Self::new(id, direction, 1_000_000)
    }

    /// Pre-seed the internal receive buffer (e.g. JSON-RPC transport tests simulating pending data).
    #[must_use]
    pub fn with_initial_recv_data(mut self, data: impl AsRef<[u8]>) -> Self {
        self.buf = data.as_ref().to_vec();
        self
    }
}

impl HardwareTransport for TestLoopbackTransport {
    fn info(&self) -> &TransportInfo {
        &self.info
    }

    fn bandwidth_bps(&self) -> u64 {
        self.bandwidth_bps
    }

    fn is_available(&self) -> bool {
        true
    }

    fn send(&mut self, data: &[u8]) -> Result<usize, TransportError> {
        self.buf.extend_from_slice(data);
        Ok(data.len())
    }

    fn recv(&mut self, buf: &mut [u8]) -> Result<usize, TransportError> {
        let n = buf.len().min(self.buf.len());
        buf[..n].copy_from_slice(&self.buf[..n]);
        self.buf.drain(..n);
        Ok(n)
    }
}

/// High-bandwidth mock transport for filter tests (medium Display).
#[doc(hidden)]
pub struct TestHighBandwidthTransport {
    info: TransportInfo,
    bandwidth_bps: u64,
}

impl TestHighBandwidthTransport {
    /// Creates a transport with explicit metadata and bandwidth.
    #[must_use]
    pub fn new(
        id: &str,
        direction: toadstool_core::TransportDirection,
        bandwidth_bps: u64,
    ) -> Self {
        use toadstool_core::TransportMedium;
        Self {
            info: TransportInfo {
                id: id.to_string(),
                label: id.to_string(),
                medium: TransportMedium::Display,
                direction,
            },
            bandwidth_bps,
        }
    }
}

impl HardwareTransport for TestHighBandwidthTransport {
    fn info(&self) -> &TransportInfo {
        &self.info
    }

    fn bandwidth_bps(&self) -> u64 {
        self.bandwidth_bps
    }

    fn is_available(&self) -> bool {
        true
    }

    fn send(&mut self, data: &[u8]) -> Result<usize, TransportError> {
        Ok(data.len())
    }

    fn recv(&mut self, buf: &mut [u8]) -> Result<usize, TransportError> {
        Ok(buf.len().min(0))
    }
}

/// Closed set of hardware transports used with [`crate::TransportRouter`].
pub enum HardwareTransportDispatch {
    /// DRM display output (HDMI/DP).
    Display(DisplayTransport),
    /// V4L2 capture input.
    Capture(CaptureTransport),
    /// PCIe peer path.
    Pcie(PcieTransport),
    /// USB serial / UART (feature `serial-transport`).
    #[cfg(feature = "serial-transport")]
    Serial(SerialTransport),
    /// Loopback test double.
    TestLoopback(TestLoopbackTransport),
    /// High-bandwidth test double.
    TestHighBandwidth(TestHighBandwidthTransport),
}

impl HardwareTransport for HardwareTransportDispatch {
    fn info(&self) -> &TransportInfo {
        match self {
            Self::Display(t) => t.info(),
            Self::Capture(t) => t.info(),
            Self::Pcie(t) => t.info(),
            #[cfg(feature = "serial-transport")]
            Self::Serial(t) => t.info(),
            Self::TestLoopback(t) => t.info(),
            Self::TestHighBandwidth(t) => t.info(),
        }
    }

    fn bandwidth_bps(&self) -> u64 {
        match self {
            Self::Display(t) => t.bandwidth_bps(),
            Self::Capture(t) => t.bandwidth_bps(),
            Self::Pcie(t) => t.bandwidth_bps(),
            #[cfg(feature = "serial-transport")]
            Self::Serial(t) => t.bandwidth_bps(),
            Self::TestLoopback(t) => t.bandwidth_bps(),
            Self::TestHighBandwidth(t) => t.bandwidth_bps(),
        }
    }

    fn is_available(&self) -> bool {
        match self {
            Self::Display(t) => t.is_available(),
            Self::Capture(t) => t.is_available(),
            Self::Pcie(t) => t.is_available(),
            #[cfg(feature = "serial-transport")]
            Self::Serial(t) => t.is_available(),
            Self::TestLoopback(t) => t.is_available(),
            Self::TestHighBandwidth(t) => t.is_available(),
        }
    }

    fn send(&mut self, data: &[u8]) -> Result<usize, TransportError> {
        match self {
            Self::Display(t) => t.send(data),
            Self::Capture(t) => t.send(data),
            Self::Pcie(t) => t.send(data),
            #[cfg(feature = "serial-transport")]
            Self::Serial(t) => t.send(data),
            Self::TestLoopback(t) => t.send(data),
            Self::TestHighBandwidth(t) => t.send(data),
        }
    }

    fn recv(&mut self, buf: &mut [u8]) -> Result<usize, TransportError> {
        match self {
            Self::Display(t) => t.recv(buf),
            Self::Capture(t) => t.recv(buf),
            Self::Pcie(t) => t.recv(buf),
            #[cfg(feature = "serial-transport")]
            Self::Serial(t) => t.recv(buf),
            Self::TestLoopback(t) => t.recv(buf),
            Self::TestHighBandwidth(t) => t.recv(buf),
        }
    }
}
