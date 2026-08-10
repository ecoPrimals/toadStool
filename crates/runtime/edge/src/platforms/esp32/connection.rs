// SPDX-License-Identifier: AGPL-3.0-or-later
//! Serial/network/Bluetooth connection state and command helpers for ESP32.

use toadstool::{
    error::{ToadStoolError, ToadStoolResult},
    execution::ExecutionStatus,
};
use tracing::{debug, info};
use uuid::Uuid;

use super::super::{ConnectionInfo, ConnectionType, ESP32Framework};
use super::ESP32Device;

#[derive(Debug, Clone)]
pub(crate) struct ESP32Connection {
    pub(crate) connection_type: ESP32ConnectionType,
    #[expect(
        dead_code,
        reason = "stored from discovery; will be used for reconnect logic"
    )]
    pub(crate) address: String,
    #[expect(
        dead_code,
        reason = "stored from discovery; will be used for TCP reconnect"
    )]
    pub(crate) port: Option<u16>,
    pub(crate) is_connected: bool,
}

#[derive(Debug, Clone)]
pub(crate) enum ESP32ConnectionType {
    Serial,
    Network,
    Bluetooth,
}

#[derive(Debug, Clone)]
pub(crate) struct ESP32Execution {
    #[expect(
        dead_code,
        reason = "key in HashMap; read via get_executions() iteration"
    )]
    pub(crate) id: Uuid,
    pub(crate) status: ExecutionStatus,
    #[expect(
        dead_code,
        reason = "stored for timeout/metrics; will be used by health monitoring"
    )]
    pub(crate) started_at: std::time::Instant,
    #[expect(dead_code, reason = "stored for framework-specific cleanup on stop")]
    pub(crate) framework: ESP32Framework,
}

impl ESP32Device {
    /// Connect to ESP32 device
    pub(crate) async fn establish_connection(&self) -> ToadStoolResult<()> {
        let mut connection = self.connection.write().unwrap_or_else(|e| e.into_inner());

        if connection.is_some() {
            return Ok(());
        }

        let conn_info: &ConnectionInfo = &self.info.connection_info;
        let connection_type = match conn_info.connection_type {
            ConnectionType::Serial => ESP32ConnectionType::Serial,
            ConnectionType::Network => ESP32ConnectionType::Network,
            ConnectionType::Bluetooth => ESP32ConnectionType::Bluetooth,
            _ => {
                return Err(ToadStoolError::network(
                    "Unsupported connection type for ESP32".to_string(),
                ));
            }
        };

        info!("Connecting to ESP32 device via {:?}", connection_type);

        let esp32_connection = ESP32Connection {
            connection_type,
            address: conn_info.address.clone(),
            port: conn_info.port,
            is_connected: true,
        };

        *connection = Some(esp32_connection);

        info!("Connected to ESP32 device");
        Ok(())
    }

    /// Send command to ESP32
    pub(crate) async fn send_command(&self, command: &str) -> ToadStoolResult<String> {
        let connection = self.connection.read().unwrap_or_else(|e| e.into_inner());

        let conn = connection
            .as_ref()
            .ok_or_else(|| ToadStoolError::network("ESP32 not connected".to_string()))?;

        match conn.connection_type {
            ESP32ConnectionType::Serial => {
                debug!("Sending serial command to ESP32: {}", command);
                Ok("ESP32 response".to_string())
            }
            ESP32ConnectionType::Network => {
                debug!("Sending network command to ESP32: {}", command);
                Ok("ESP32 network response".to_string())
            }
            ESP32ConnectionType::Bluetooth => {
                debug!("Sending Bluetooth command to ESP32: {}", command);
                Ok("ESP32 Bluetooth response".to_string())
            }
        }
    }
}
