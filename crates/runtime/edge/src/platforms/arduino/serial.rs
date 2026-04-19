// SPDX-License-Identifier: AGPL-3.0-or-later
//! Serial port I/O for Arduino devices.

use std::io::{Read, Write};
use std::time::{Duration, Instant};
use tracing::{debug, info};

use toadstool::error::{ToadStoolError, ToadStoolResult};

use super::device::ArduinoDevice;

impl ArduinoDevice {
    /// Open serial connection
    pub(super) async fn open_serial_connection(&self) -> ToadStoolResult<()> {
        let mut port_guard = self.serial_port.lock().await;

        if port_guard.is_some() {
            return Ok(());
        }

        let port_name = &self.info.connection_info.address;
        let baud_rate = 9600; // Default Arduino baud rate

        info!("Opening serial connection to Arduino on {}", port_name);

        let port = serialport::new(port_name, baud_rate)
            .timeout(Duration::from_millis(1000))
            .open()
            .map_err(|e| {
                ToadStoolError::network(format!(
                    "Failed to open serial port {}: {}",
                    port_name, e
                ))
            })?;

        *port_guard = Some(port);
        info!("Serial connection established to Arduino");

        Ok(())
    }

    /// Close serial connection
    pub(super) async fn close_serial_connection(&self) -> ToadStoolResult<()> {
        let mut port_guard = self.serial_port.lock().await;

        if let Some(mut port) = port_guard.take() {
            info!("Closing serial connection to Arduino");
            // Send any cleanup commands if needed
            let _ = port.write_all(b"RESET\n");
            let _ = port.flush();
        }

        Ok(())
    }

    /// Send command to Arduino
    pub(super) async fn send_command(&self, command: &str) -> ToadStoolResult<String> {
        let mut port_guard = self.serial_port.lock().await;

        let port = port_guard.as_mut().ok_or_else(|| {
            ToadStoolError::network("Serial port not connected".to_string())
        })?;

        // Send command
        let command_bytes = format!("{}\n", command).into_bytes();
        port.write_all(&command_bytes).map_err(|e| {
            ToadStoolError::execution(format!("Failed to send command: {}", e))
        })?;

        port.flush().map_err(|e| {
            ToadStoolError::execution(format!("Failed to flush serial port: {}", e))
        })?;

        // Read response
        let mut buffer = vec![0; 1024];
        let bytes_read = port.read(&mut buffer).map_err(|e| {
            ToadStoolError::execution(format!("Failed to read response: {}", e))
        })?;

        let response = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
        debug!("Arduino response: {}", response);

        Ok(response)
    }

    /// Read serial output with a timeout.
    ///
    /// Collects all bytes available on the serial port within `timeout`,
    /// returning whatever the Arduino has written back.
    pub(super) async fn read_serial_output(&self, timeout: Duration) -> ToadStoolResult<String> {
        let mut port_guard = self.serial_port.lock().await;

        let port = port_guard.as_mut().ok_or_else(|| {
            ToadStoolError::network("Serial port not connected".to_string())
        })?;

        port.set_timeout(timeout).map_err(|e| {
            ToadStoolError::execution(format!("Failed to set serial timeout: {e}"))
        })?;

        let mut collected = Vec::with_capacity(4096);
        let mut buf = [0u8; 1024];
        let deadline = Instant::now() + timeout;

        while Instant::now() < deadline {
            match port.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => collected.extend_from_slice(&buf[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => break,
                Err(e) => {
                    return Err(ToadStoolError::execution(format!(
                        "Serial read error: {e}"
                    )));
                }
            }
        }

        Ok(String::from_utf8_lossy(&collected).into_owned())
    }
}
