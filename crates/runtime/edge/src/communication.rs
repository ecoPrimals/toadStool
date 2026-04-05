// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Communication Manager
//!
//! Handles communication protocols for edge devices including serial, network, and wireless protocols.

use std::collections::HashMap;
use std::io::Write;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

use toadstool::{
    error::{ToadStoolError, ToadStoolResult},
};

use crate::EdgeRuntimeConfig;

/// Communication Manager
pub struct CommunicationManager {
    config: EdgeRuntimeConfig,
    protocols: Arc<RwLock<HashMap<String, Box<dyn CommunicationProtocol>>>>,
}

/// Communication Protocol Trait
#[async_trait::async_trait]
pub trait CommunicationProtocol: Send + Sync {
    /// Get protocol name
    fn get_name(&self) -> &str;

    /// Check if protocol is available
    async fn is_available(&self) -> bool;

    /// Send message
    async fn send_message(&self, address: &str, message: &[u8]) -> ToadStoolResult<()>;

    /// Receive message
    async fn receive_message(&self, address: &str) -> ToadStoolResult<Vec<u8>>;

    /// Establish connection
    async fn connect(&self, address: &str) -> ToadStoolResult<()>;

    /// Close connection
    async fn disconnect(&self, address: &str) -> ToadStoolResult<()>;
}

impl CommunicationManager {
    /// Create a new communication manager
    pub async fn new(config: &EdgeRuntimeConfig) -> ToadStoolResult<Self> {
        info!("Initializing communication manager");

        let manager = Self {
            config: config.clone(),
            protocols: Arc::new(RwLock::new(HashMap::new())),
        };

        // Initialize protocols (Serial, Network)
        manager.initialize_protocols().await?;

        Ok(manager)
    }

    /// Initialize communication protocols using existing transport types
    async fn initialize_protocols(&self) -> ToadStoolResult<()> {
        let timeout_ms = Duration::from_millis(self.config.communication_timeout_ms);

        // Serial port protocol (for Arduino, ESP32, etc.)
        let serial = SerialProtocol {
            timeout: timeout_ms,
            baud_rates: vec![9600, 115200, 57600, 38400],
        };
        if serial.is_available().await {
            let mut protocols = self.protocols.write().await;
            protocols.insert("serial".to_string(), Box::new(serial));
            info!("Registered Serial communication protocol");
        }

        // TCP network protocol (for Raspberry Pi, Linux edge, etc.)
        let network = NetworkProtocol {
            timeout: timeout_ms,
        };
        if network.is_available().await {
            let mut protocols = self.protocols.write().await;
            protocols.insert("tcp".to_string(), Box::new(network));
            info!("Registered TCP network communication protocol");
        }

        let count = self.protocols.read().await.len();
        info!("Communication protocols initialized: {} active", count);
        Ok(())
    }

    /// Send message via appropriate protocol (auto-detect from address format)
    pub async fn send(&self, address: &str, message: &[u8]) -> ToadStoolResult<()> {
        let protocol = if address.starts_with('/') || address.contains("tty") || address.contains("COM") {
            self.protocols.read().await.get("serial").cloned()
        } else {
            self.protocols.read().await.get("tcp").cloned()
        };

        match protocol {
            Some(p) => p.send_message(address, message).await,
            None => Err(ToadStoolError::discovery_error(format!(
                "No suitable protocol for address: {}",
                address
            ))),
        }
    }

    /// Receive message via appropriate protocol
    pub async fn receive(&self, address: &str) -> ToadStoolResult<Vec<u8>> {
        let protocol = if address.starts_with('/') || address.contains("tty") || address.contains("COM") {
            self.protocols.read().await.get("serial").cloned()
        } else {
            self.protocols.read().await.get("tcp").cloned()
        };

        match protocol {
            Some(p) => p.receive_message(address).await,
            None => Err(ToadStoolError::discovery_error(format!(
                "No suitable protocol for address: {}",
                address
            ))),
        }
    }
}

/// Serial port communication protocol
struct SerialProtocol {
    timeout: Duration,
    baud_rates: Vec<u32>,
}

#[async_trait::async_trait]
impl CommunicationProtocol for SerialProtocol {
    fn get_name(&self) -> &str {
        "Serial"
    }

    async fn is_available(&self) -> bool {
        serialport::available_ports().is_ok()
    }

    async fn send_message(&self, address: &str, message: &[u8]) -> ToadStoolResult<()> {
        let baud = self.baud_rates.first().copied().unwrap_or(9600);
        let timeout = self.timeout;
        let address = address.to_string();
        let message = message.to_vec();

        tokio::task::spawn_blocking(move || {
            let mut port = serialport::new(&address, baud)
                .timeout(timeout)
                .open()
                .map_err(|e| ToadStoolError::discovery_error(format!("Serial open failed: {}", e)))?;

            port.write_all(&message)
                .map_err(|e| ToadStoolError::discovery_error(format!("Serial write failed: {}", e)))?;

            Ok::<(), ToadStoolError>(())
        })
        .await
        .map_err(|e| ToadStoolError::discovery_error(format!("Serial task join: {}", e)))?
    }

    async fn receive_message(&self, address: &str) -> ToadStoolResult<Vec<u8>> {
        let baud = self.baud_rates.first().copied().unwrap_or(9600);
        let timeout = self.timeout;
        let address = address.to_string();

        let buf = tokio::task::spawn_blocking(move || {
            let mut port = serialport::new(&address, baud)
                .timeout(timeout)
                .open()
                .map_err(|e| ToadStoolError::discovery_error(format!("Serial open failed: {}", e)))?;

            let mut buf = vec![0u8; 4096];
            let n = port
                .read(&mut buf)
                .map_err(|e| ToadStoolError::discovery_error(format!("Serial read failed: {}", e)))?;

            buf.truncate(n);
            Ok::<Vec<u8>, ToadStoolError>(buf)
        })
        .await
        .map_err(|e| ToadStoolError::discovery_error(format!("Serial task join: {}", e)))??;

        Ok(buf)
    }

    async fn connect(&self, _address: &str) -> ToadStoolResult<()> {
        // Serial is connectionless per message - no persistent connection
        Ok(())
    }

    async fn disconnect(&self, _address: &str) -> ToadStoolResult<()> {
        Ok(())
    }
}

/// TCP network communication protocol
struct NetworkProtocol {
    timeout: Duration,
}

#[async_trait::async_trait]
impl CommunicationProtocol for NetworkProtocol {
    fn get_name(&self) -> &str {
        "TCP"
    }

    async fn is_available(&self) -> bool {
        true
    }

    async fn send_message(&self, address: &str, message: &[u8]) -> ToadStoolResult<()> {
        let mut stream = timeout(
            self.timeout,
            TcpStream::connect(address),
        )
        .await
        .map_err(|_| ToadStoolError::discovery_error("TCP connect timeout".to_string()))?
        .map_err(|e| ToadStoolError::discovery_error(format!("TCP connect failed: {}", e)))?;

        use tokio::io::AsyncWriteExt;
        stream
            .write_all(message)
            .await
            .map_err(|e| ToadStoolError::discovery_error(format!("TCP write failed: {}", e)))?;

        Ok(())
    }

    async fn receive_message(&self, address: &str) -> ToadStoolResult<Vec<u8>> {
        let mut stream = timeout(
            self.timeout,
            TcpStream::connect(address),
        )
        .await
        .map_err(|_| ToadStoolError::discovery_error("TCP connect timeout".to_string()))?
        .map_err(|e| ToadStoolError::discovery_error(format!("TCP connect failed: {}", e)))?;

        let mut buf = vec![0u8; 4096];
        let n = timeout(self.timeout, stream.read(&mut buf))
            .await
            .map_err(|_| ToadStoolError::discovery_error("TCP read timeout".to_string()))?
            .map_err(|e| ToadStoolError::discovery_error(format!("TCP read failed: {}", e)))?;

        buf.truncate(n);
        Ok(buf)
    }

    async fn connect(&self, _address: &str) -> ToadStoolResult<()> {
        Ok(())
    }

    async fn disconnect(&self, _address: &str) -> ToadStoolResult<()> {
        Ok(())
    }
} 