// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Device Discovery Service
//!
//! Automatic discovery and connection to edge devices using multiple discovery methods.
//! Supports serial port scanning, network discovery, and device-specific protocols.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use toadstool::error::ToadStoolResult;

use crate::platforms::*;
use crate::EdgeRuntimeConfig;

mod bluetooth;
mod mdns;
mod network;
mod serial;
mod usb;

pub use bluetooth::BluetoothDiscovery;
pub use mdns::MDNSDiscovery;
pub use network::NetworkDiscovery;
pub use serial::SerialPortDiscovery;
pub use usb::USBDiscovery;

/// Device Discovery Service
pub struct DeviceDiscoveryService {
    config: EdgeRuntimeConfig,
    discovered_devices: Arc<RwLock<HashMap<Uuid, Arc<dyn EdgeDevice>>>>,
    discovery_methods: Vec<Box<dyn DiscoveryMethod>>,
    last_discovery: Arc<RwLock<Option<Instant>>>,
}

/// Discovery method for [`DeviceDiscoveryService`].
///
/// Stored as `Vec<Box<dyn DiscoveryMethod>>`; async methods use `#[async_trait]` for object safety.
#[async_trait::async_trait]
pub trait DiscoveryMethod: Send + Sync {
    /// Get discovery method name
    fn get_name(&self) -> &str;

    /// Discover devices using this method
    async fn discover(&self) -> ToadStoolResult<Vec<Arc<dyn EdgeDevice>>>;

    /// Check if method is available
    async fn is_available(&self) -> bool;

    /// Get supported device types
    fn get_supported_types(&self) -> Vec<String>;
}

impl DeviceDiscoveryService {
    /// Create a new device discovery service
    pub async fn new(config: &EdgeRuntimeConfig) -> ToadStoolResult<Self> {
        info!("Initializing device discovery service");

        let mut discovery_methods: Vec<Box<dyn DiscoveryMethod>> = Vec::new();

        // Add serial port discovery
        discovery_methods.push(Box::new(SerialPortDiscovery {
            baud_rates: vec![9600, 115200, 57600, 38400, 19200],
            timeout: Duration::from_secs(2),
        }));

        // Add network discovery
        // ✅ Using PortRegistry for dynamic port configuration
        discovery_methods.push(Box::new(NetworkDiscovery {
            scan_range: vec![
                std::net::IpAddr::V4(std::net::Ipv4Addr::new(192, 168, 1, 0)),
                std::net::IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 0)),
            ],
            ports: config.port_registry.edge_discovery_ports().to_vec(),
            timeout: Duration::from_secs(1),
        }));

        // Add USB discovery
        discovery_methods.push(Box::new(USBDiscovery {
            vendor_filters: vec![
                0x2341, // Arduino
                0x1A86, // CH340
                0x0403, // FTDI
                0x10C4, // Silicon Labs
                0x1B4F, // SparkFun
            ],
            product_filters: vec![],
        }));

        // Add Bluetooth discovery
        discovery_methods.push(Box::new(BluetoothDiscovery {
            scan_duration: Duration::from_secs(10),
            device_types: vec!["ESP32".to_string(), "Arduino".to_string()],
        }));

        // Add mDNS discovery
        discovery_methods.push(Box::new(MDNSDiscovery {
            service_types: vec![
                "_arduino._tcp".to_string(),
                "_esp32._tcp".to_string(),
                "_raspberry-pi._tcp".to_string(),
                "_toadstool-edge._tcp".to_string(),
            ],
            timeout: Duration::from_secs(5),
        }));

        Ok(Self {
            config: config.clone(),
            discovered_devices: Arc::new(RwLock::new(HashMap::new())),
            discovery_methods,
            last_discovery: Arc::new(RwLock::new(None)),
        })
    }

    /// Discover devices using all available methods
    pub async fn discover_devices(&self) -> ToadStoolResult<Vec<Arc<dyn EdgeDevice>>> {
        info!("Starting device discovery");

        let mut all_devices = Vec::new();
        let mut device_ids = HashSet::new();

        // Run all discovery methods in parallel
        let mut discovery_tasks = Vec::new();

        for method in &self.discovery_methods {
            if method.is_available().await {
                let method_name = method.get_name().to_string();
                info!("Running discovery method: {}", method_name);

                // Clone the method reference for async task
                let method_ref = method.as_ref();
                discovery_tasks.push(async move {
                    match method_ref.discover().await {
                        Ok(devices) => {
                            info!(
                                "Discovery method {} found {} devices",
                                method_name,
                                devices.len()
                            );
                            devices
                        }
                        Err(e) => {
                            warn!("Discovery method {} failed: {}", method_name, e);
                            Vec::new()
                        }
                    }
                });
            } else {
                debug!("Discovery method {} is not available", method.get_name());
            }
        }

        // Wait for all discovery methods to complete
        let results = futures::future::join_all(discovery_tasks).await;

        // Collect unique devices
        for devices in results {
            for device in devices {
                let device_id = device.get_id();
                if !device_ids.contains(&device_id) {
                    device_ids.insert(device_id);
                    all_devices.push(device);
                }
            }
        }

        // Update discovered devices cache
        {
            let mut discovered = self.discovered_devices.write().await;
            discovered.clear();
            for device in &all_devices {
                discovered.insert(device.get_id(), device.clone());
            }
        }

        // Update last discovery time
        {
            let mut last_discovery = self.last_discovery.write().await;
            *last_discovery = Some(Instant::now());
        }

        info!(
            "Device discovery completed. Found {} unique devices",
            all_devices.len()
        );
        Ok(all_devices)
    }

    /// Get discovered devices
    pub async fn get_discovered_devices(&self) -> Vec<Arc<dyn EdgeDevice>> {
        let discovered = self.discovered_devices.read().await;
        discovered.values().cloned().collect()
    }

    /// Get device by ID
    pub async fn get_device(&self, id: Uuid) -> Option<Arc<dyn EdgeDevice>> {
        let discovered = self.discovered_devices.read().await;
        discovered.get(&id).cloned()
    }

    /// Check if discovery is needed
    pub async fn needs_discovery(&self) -> bool {
        let last_discovery = self.last_discovery.read().await;
        match *last_discovery {
            Some(last) => last.elapsed() > Duration::from_secs(self.config.discovery_timeout_secs),
            None => true,
        }
    }

    /// Start continuous discovery
    ///
    /// ✅ MODERNIZED: Uses tokio::time::interval instead of sleep
    /// - No drift accumulation
    /// - More precise timing
    /// - Idiomatic Tokio pattern
    pub async fn start_continuous_discovery(&self) -> ToadStoolResult<()> {
        let discovery_interval = Duration::from_secs(self.config.discovery_timeout_secs);
        let service = Arc::new(self);

        tokio::spawn(async move {
            // ✅ Use interval instead of sleep - prevents drift and more efficient
            let mut interval = tokio::time::interval(discovery_interval);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                // Wait for next tick (first tick fires immediately)
                interval.tick().await;

                if service.needs_discovery().await {
                    if let Err(e) = service.discover_devices().await {
                        error!("Continuous discovery failed: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    /// Trigger immediate discovery (on-demand)
    ///
    /// Useful for manual device discovery or responding to network events
    pub async fn trigger_discovery(&self) -> ToadStoolResult<Vec<Arc<dyn EdgeDevice>>> {
        info!("Triggering immediate device discovery");
        self.discover_devices().await
    }
}
