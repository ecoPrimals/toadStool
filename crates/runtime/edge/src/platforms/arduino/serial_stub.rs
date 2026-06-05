// SPDX-License-Identifier: AGPL-3.0-or-later
//! Serial I/O stubs when `serial-transport` is disabled.

use std::time::Duration;

use toadstool::error::{ToadStoolError, ToadStoolResult};

use crate::serial_transport::SERIAL_TRANSPORT_UNAVAILABLE;

use super::device::ArduinoDevice;

impl ArduinoDevice {
    pub(super) async fn open_serial_connection(&self) -> ToadStoolResult<()> {
        Err(ToadStoolError::runtime(SERIAL_TRANSPORT_UNAVAILABLE.to_string()))
    }

    pub(super) async fn close_serial_connection(&self) -> ToadStoolResult<()> {
        Err(ToadStoolError::runtime(SERIAL_TRANSPORT_UNAVAILABLE.to_string()))
    }

    pub(super) async fn send_command(&self, _command: &str) -> ToadStoolResult<String> {
        Err(ToadStoolError::runtime(SERIAL_TRANSPORT_UNAVAILABLE.to_string()))
    }

    pub(super) async fn read_serial_output(&self, _timeout: Duration) -> ToadStoolResult<String> {
        Err(ToadStoolError::runtime(SERIAL_TRANSPORT_UNAVAILABLE.to_string()))
    }
}
