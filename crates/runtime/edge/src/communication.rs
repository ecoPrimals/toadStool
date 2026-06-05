// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Communication Manager
//!
//! Handles communication protocols for edge devices including serial, network, and wireless protocols.

use std::collections::HashMap;
#[cfg(feature = "serial-transport")]
use std::io::Write;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio::time::timeout;
use tracing::info;

use toadstool::error::{ToadStoolError, ToadStoolResult};

#[cfg(not(feature = "serial-transport"))]
use crate::serial_transport::SERIAL_TRANSPORT_UNAVAILABLE;
use crate::EdgeRuntimeConfig;

/// Communication Manager
pub struct CommunicationManager {
    config: EdgeRuntimeConfig,
    protocols: Arc<RwLock<HashMap<String, Box<dyn CommunicationProtocol>>>>,
}

/// Communication Protocol Trait
///
/// Stored as `Box<dyn CommunicationProtocol>`. Uses manual `Pin<Box<dyn Future>>`
/// for dyn-compatibility (no `async-trait` macro).
pub trait CommunicationProtocol: Send + Sync {
    /// Get protocol name
    fn get_name(&self) -> &str;

    /// Check if protocol is available
    fn is_available(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send + '_>>;

    /// Send message
    fn send_message<'a>(
        &'a self,
        address: &'a str,
        message: &'a [u8],
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ToadStoolResult<()>> + Send + 'a>>;

    /// Receive message
    fn receive_message<'a>(
        &'a self,
        address: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ToadStoolResult<Vec<u8>>> + Send + 'a>>;

    /// Establish connection
    fn connect<'a>(
        &'a self,
        address: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ToadStoolResult<()>> + Send + 'a>>;

    /// Close connection
    fn disconnect<'a>(
        &'a self,
        address: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ToadStoolResult<()>> + Send + 'a>>;
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
        #[cfg(feature = "serial-transport")]
        {
            let serial = SerialProtocol {
                timeout: timeout_ms,
                baud_rates: vec![9600, 115200, 57600, 38400],
            };
            if serial.is_available().await {
                let mut protocols = self.protocols.write().await;
                protocols.insert("serial".to_string(), Box::new(serial));
                info!("Registered Serial communication protocol");
            }
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

    fn protocol_key_for_address(address: &str) -> &'static str {
        if address.starts_with('/') || address.contains("tty") || address.contains("COM") {
            "serial"
        } else {
            "tcp"
        }
    }

    /// Send message via appropriate protocol (auto-detect from address format)
    pub async fn send(&self, address: &str, message: &[u8]) -> ToadStoolResult<()> {
        let key = Self::protocol_key_for_address(address);
        #[cfg(not(feature = "serial-transport"))]
        if key == "serial" {
            return Err(ToadStoolError::runtime(
                SERIAL_TRANSPORT_UNAVAILABLE.to_string(),
            ));
        }

        let protocols = self.protocols.read().await;
        let protocol = protocols.get(key).ok_or_else(|| {
            ToadStoolError::runtime(format!("No suitable protocol for address: {}", address))
        })?;
        protocol.send_message(address, message).await
    }

    /// Receive message via appropriate protocol
    pub async fn receive(&self, address: &str) -> ToadStoolResult<Vec<u8>> {
        let key = Self::protocol_key_for_address(address);
        #[cfg(not(feature = "serial-transport"))]
        if key == "serial" {
            return Err(ToadStoolError::runtime(
                SERIAL_TRANSPORT_UNAVAILABLE.to_string(),
            ));
        }

        let protocols = self.protocols.read().await;
        let protocol = protocols.get(key).ok_or_else(|| {
            ToadStoolError::runtime(format!("No suitable protocol for address: {}", address))
        })?;
        protocol.receive_message(address).await
    }
}

/// Serial port communication protocol (`serial-transport` feature).
#[cfg(feature = "serial-transport")]
struct SerialProtocol {
    timeout: Duration,
    baud_rates: Vec<u32>,
}

#[cfg(feature = "serial-transport")]
impl CommunicationProtocol for SerialProtocol {
    fn get_name(&self) -> &str {
        "Serial"
    }

    fn is_available(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send + '_>> {
        Box::pin(async { serialport::available_ports().is_ok() })
    }

    fn send_message<'a>(
        &'a self,
        address: &'a str,
        message: &'a [u8],
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ToadStoolResult<()>> + Send + 'a>> {
        Box::pin(async move {
            let baud = self.baud_rates.first().copied().unwrap_or(9600);
            let timeout = self.timeout;
            let address = address.to_string();
            let message = message.to_vec();

            tokio::task::spawn_blocking(move || {
                let mut port = serialport::new(&address, baud)
                    .timeout(timeout)
                    .open()
                    .map_err(|e| ToadStoolError::runtime(format!("Serial open failed: {}", e)))?;

                port.write_all(&message)
                    .map_err(|e| ToadStoolError::runtime(format!("Serial write failed: {}", e)))?;

                Ok::<(), ToadStoolError>(())
            })
            .await
            .map_err(|e| ToadStoolError::runtime(format!("Serial task join: {}", e)))?
        })
    }

    fn receive_message<'a>(
        &'a self,
        address: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ToadStoolResult<Vec<u8>>> + Send + 'a>>
    {
        Box::pin(async move {
            let baud = self.baud_rates.first().copied().unwrap_or(9600);
            let timeout = self.timeout;
            let address = address.to_string();

            let buf = tokio::task::spawn_blocking(move || {
                let mut port = serialport::new(&address, baud)
                    .timeout(timeout)
                    .open()
                    .map_err(|e| ToadStoolError::runtime(format!("Serial open failed: {}", e)))?;

                let mut buf = vec![0u8; 4096];
                let n = port
                    .read(&mut buf)
                    .map_err(|e| ToadStoolError::runtime(format!("Serial read failed: {}", e)))?;

                buf.truncate(n);
                Ok::<Vec<u8>, ToadStoolError>(buf)
            })
            .await
            .map_err(|e| ToadStoolError::runtime(format!("Serial task join: {}", e)))??;

            Ok(buf)
        })
    }

    fn connect<'a>(
        &'a self,
        _address: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ToadStoolResult<()>> + Send + 'a>> {
        Box::pin(async { Ok(()) })
    }

    fn disconnect<'a>(
        &'a self,
        _address: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ToadStoolResult<()>> + Send + 'a>> {
        Box::pin(async { Ok(()) })
    }
}

/// TCP network communication protocol
struct NetworkProtocol {
    timeout: Duration,
}

impl CommunicationProtocol for NetworkProtocol {
    fn get_name(&self) -> &str {
        "TCP"
    }

    fn is_available(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send + '_>> {
        Box::pin(async { true })
    }

    fn send_message<'a>(
        &'a self,
        address: &'a str,
        message: &'a [u8],
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ToadStoolResult<()>> + Send + 'a>> {
        Box::pin(async move {
            let mut stream = timeout(self.timeout, TcpStream::connect(address))
                .await
                .map_err(|_| ToadStoolError::runtime("TCP connect timeout".to_string()))?
                .map_err(|e| ToadStoolError::runtime(format!("TCP connect failed: {}", e)))?;

            use tokio::io::AsyncWriteExt;
            stream
                .write_all(message)
                .await
                .map_err(|e| ToadStoolError::runtime(format!("TCP write failed: {}", e)))?;

            Ok(())
        })
    }

    fn receive_message<'a>(
        &'a self,
        address: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ToadStoolResult<Vec<u8>>> + Send + 'a>>
    {
        Box::pin(async move {
            let mut stream = timeout(self.timeout, TcpStream::connect(address))
                .await
                .map_err(|_| ToadStoolError::runtime("TCP connect timeout".to_string()))?
                .map_err(|e| ToadStoolError::runtime(format!("TCP connect failed: {}", e)))?;

            let mut buf = vec![0u8; 4096];
            let n = timeout(self.timeout, stream.read(&mut buf))
                .await
                .map_err(|_| ToadStoolError::runtime("TCP read timeout".to_string()))?
                .map_err(|e| ToadStoolError::runtime(format!("TCP read failed: {}", e)))?;

            buf.truncate(n);
            Ok(buf)
        })
    }

    fn connect<'a>(
        &'a self,
        _address: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ToadStoolResult<()>> + Send + 'a>> {
        Box::pin(async { Ok(()) })
    }

    fn disconnect<'a>(
        &'a self,
        _address: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ToadStoolResult<()>> + Send + 'a>> {
        Box::pin(async { Ok(()) })
    }
}
