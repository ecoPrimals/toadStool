// SPDX-License-Identifier: AGPL-3.0-or-later
//! Serial Transport — bidirectional data over USB serial / UART.
//!
//! Wraps the `serialport` crate (same backing lib as the edge crate) as a
//! [`HardwareTransport`](toadstool_core::HardwareTransport). Connects the high-bandwidth display/capture path
//! to low-bandwidth edge devices: `capture in -> serial out -> Arduino`.

#[cfg(feature = "serial-transport")]
mod inner {
    use std::io::{Read, Write};
    use std::sync::Mutex;
    use std::time::Duration;

    use toadstool_core::{
        HardwareTransport, TransportDirection, TransportError, TransportInfo, TransportMedium,
    };

    /// Default baud rate.
    const DEFAULT_BAUD: u32 = 115_200;
    /// Default I/O timeout.
    const DEFAULT_TIMEOUT: Duration = Duration::from_millis(500);

    /// A bidirectional hardware transport over a serial port.
    ///
    /// The inner `serialport::SerialPort` trait object is `!Sync`, so we wrap
    /// it in a `Mutex` to satisfy `HardwareTransport: Send + Sync`. The mutex
    /// is uncontended in practice — serial I/O is inherently sequential.
    pub struct SerialTransport {
        info: TransportInfo,
        port: Mutex<Box<dyn serialport::SerialPort>>,
        baud: u32,
    }

    impl SerialTransport {
        /// Open a serial transport on the given port (e.g. `/dev/ttyUSB0`).
        pub fn open(port_path: &str, baud: Option<u32>) -> Result<Self, TransportError> {
            let baud = baud.unwrap_or(DEFAULT_BAUD);

            let port = serialport::new(port_path, baud)
                .timeout(DEFAULT_TIMEOUT)
                .open()
                .map_err(|e| TransportError::OpenFailed(format!("{e}")))?;

            Ok(Self {
                info: TransportInfo {
                    id: port_path.to_string(),
                    label: format!("Serial:{port_path}@{baud}"),
                    medium: TransportMedium::Serial,
                    direction: TransportDirection::Bidirectional,
                },
                port: Mutex::new(port),
                baud,
            })
        }

        /// Current baud rate.
        #[must_use]
        pub const fn baud(&self) -> u32 {
            self.baud
        }
    }

    impl HardwareTransport for SerialTransport {
        fn info(&self) -> &TransportInfo {
            &self.info
        }

        fn bandwidth_bps(&self) -> u64 {
            // Serial: ~10 bits per byte (start + 8 data + stop).
            u64::from(self.baud) / 10 * 8
        }

        fn is_available(&self) -> bool {
            true
        }

        fn send(&mut self, data: &[u8]) -> Result<usize, TransportError> {
            let mut port = self
                .port
                .lock()
                .map_err(|e| TransportError::Unavailable(format!("serial lock poisoned: {e}")))?;
            port.write_all(data)?;
            port.flush()?;
            drop(port);
            Ok(data.len())
        }

        fn recv(&mut self, buf: &mut [u8]) -> Result<usize, TransportError> {
            let n = self
                .port
                .lock()
                .map_err(|e| TransportError::Unavailable(format!("serial lock poisoned: {e}")))?
                .read(buf)?;
            Ok(n)
        }
    }

    /// Discover all serial ports on the system.
    #[must_use]
    pub fn discover_serial_transports() -> Vec<TransportInfo> {
        let ports = match serialport::available_ports() {
            Ok(p) => p,
            Err(_) => return Vec::new(),
        };

        ports
            .into_iter()
            .map(|p| TransportInfo {
                id: p.port_name.clone(),
                label: format!("Serial:{}", p.port_name),
                medium: TransportMedium::Serial,
                direction: TransportDirection::Bidirectional,
            })
            .collect()
    }
}

#[cfg(feature = "serial-transport")]
pub use inner::{SerialTransport, discover_serial_transports};

#[cfg(not(feature = "serial-transport"))]
pub use feature_disabled::discover_serial_transports;

#[cfg(not(feature = "serial-transport"))]
mod feature_disabled {
    use toadstool_core::TransportInfo;

    /// Discover serial transports when `serial-transport` feature is disabled.
    ///
    /// Returns an empty list — no serial ports are discoverable without the
    /// `serial-transport` dependency. This is the correct production behavior
    /// when the feature is not enabled.
    #[must_use]
    pub const fn discover_serial_transports() -> Vec<TransportInfo> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discover_serial_transports_returns_vec() {
        let transports = discover_serial_transports();
        // Returns Vec; empty when serial-transport feature disabled or no ports
        assert!(transports.iter().all(|t| !t.id.is_empty()));
    }
}
