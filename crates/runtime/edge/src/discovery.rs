// SPDX-License-Identifier: AGPL-3.0-only
//! # Device Discovery Service
//!
//! Automatic discovery and connection to edge devices using multiple discovery methods.
//! Supports serial port scanning, network discovery, and device-specific protocols.

use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use toadstool::{
    error::{ToadStoolError, ToadStoolResult},
};

use crate::platforms::*;
use crate::EdgeRuntimeConfig;

/// Device Discovery Service
pub struct DeviceDiscoveryService {
    config: EdgeRuntimeConfig,
    discovered_devices: Arc<RwLock<HashMap<Uuid, Arc<dyn EdgeDevice>>>>,
    discovery_methods: Vec<Box<dyn DiscoveryMethod>>,
    last_discovery: Arc<RwLock<Option<Instant>>>,
}

/// Discovery Method Trait
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

/// Serial Port Discovery Method
pub struct SerialPortDiscovery {
    baud_rates: Vec<u32>,
    timeout: Duration,
}

/// Network Discovery Method
pub struct NetworkDiscovery {
    scan_range: Vec<IpAddr>,
    ports: Vec<u16>,
    timeout: Duration,
}

/// USB Device Discovery Method
pub struct USBDiscovery {
    vendor_filters: Vec<u16>,
    product_filters: Vec<u16>,
}

/// Bluetooth Discovery Method
pub struct BluetoothDiscovery {
    scan_duration: Duration,
    device_types: Vec<String>,
}

/// mDNS Discovery Method
pub struct MDNSDiscovery {
    service_types: Vec<String>,
    timeout: Duration,
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
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 0)),
                IpAddr::V4(Ipv4Addr::new(10, 0, 0, 0)),
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
                            info!("Discovery method {} found {} devices", method_name, devices.len());
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
        
        info!("Device discovery completed. Found {} unique devices", all_devices.len());
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

#[async_trait::async_trait]
impl DiscoveryMethod for SerialPortDiscovery {
    fn get_name(&self) -> &str {
        "Serial Port Discovery"
    }
    
    async fn discover(&self) -> ToadStoolResult<Vec<Arc<dyn EdgeDevice>>> {
        let mut devices = Vec::new();
        
        // Get available serial ports
        let ports = serialport::available_ports()
            .map_err(|e| ToadStoolError::discovery_error(
                format!("Failed to enumerate serial ports: {}", e)
            ))?;
        
        for port in ports {
            // Try to identify device type
            if let Some(device) = self.identify_serial_device(&port).await {
                devices.push(device);
            }
        }
        
        Ok(devices)
    }
    
    async fn is_available(&self) -> bool {
        serialport::available_ports().is_ok()
    }
    
    fn get_supported_types(&self) -> Vec<String> {
        vec![
            "Arduino".to_string(),
            "ESP32".to_string(),
            "Generic Serial".to_string(),
        ]
    }
}

impl SerialPortDiscovery {
    async fn identify_serial_device(&self, port: &serialport::SerialPortInfo) -> Option<Arc<dyn EdgeDevice>> {
        if let serialport::SerialPortType::UsbPort(usb_info) = &port.port_type {
            // Check for Arduino devices
            if ArduinoDevice::is_arduino_device(usb_info.vid, usb_info.pid) {
                let board = ArduinoDevice::detect_board_type(usb_info.vid, usb_info.pid);
                if let Ok(device) = ArduinoDevice::new(
                    board,
                    "1.0".to_string(),
                    port.port_name.clone(),
                    9600,
                ) {
                    return Some(Arc::new(device));
                }
            }
            
            // Check for ESP32 devices
            if self.is_esp32_device(usb_info.vid, usb_info.pid) {
                // Create ESP32 device (implementation needed)
                // For now, we'll skip ESP32 creation
                debug!("Found ESP32 device on {}", port.port_name);
            }
        }
        
        None
    }
    
    fn is_esp32_device(&self, vid: u16, pid: u16) -> bool {
        match vid {
            0x10C4 => matches!(pid, 0xEA60), // Silicon Labs CP210x
            0x1A86 => matches!(pid, 0x7523), // CH340
            0x0403 => matches!(pid, 0x6001 | 0x6010 | 0x6011), // FTDI
            _ => false,
        }
    }
}

#[async_trait::async_trait]
impl DiscoveryMethod for NetworkDiscovery {
    fn get_name(&self) -> &str {
        "Network Discovery"
    }
    
    async fn discover(&self) -> ToadStoolResult<Vec<Arc<dyn EdgeDevice>>> {
        let mut devices = Vec::new();
        
        // Scan network ranges
        for ip in &self.scan_range {
            let scan_devices = self.scan_network_range(*ip).await?;
            devices.extend(scan_devices);
        }
        
        Ok(devices)
    }
    
    async fn is_available(&self) -> bool {
        // Check if network interface is available
        true
    }
    
    fn get_supported_types(&self) -> Vec<String> {
        vec![
            "Raspberry Pi".to_string(),
            "Linux Edge".to_string(),
            "ESP32".to_string(),
            "Network Device".to_string(),
        ]
    }
}

impl NetworkDiscovery {
    async fn scan_network_range(&self, base_ip: IpAddr) -> ToadStoolResult<Vec<Arc<dyn EdgeDevice>>> {
        let mut devices = Vec::new();
        
        // For now, implement a simple ping-based scan
        // In a real implementation, this would use more sophisticated network scanning
        
        match base_ip {
            IpAddr::V4(ipv4) => {
                let base_octets = ipv4.octets();
                
                // Scan /24 network
                for host in 1..255 {
                    let target_ip = Ipv4Addr::new(
                        base_octets[0],
                        base_octets[1],
                        base_octets[2],
                        host,
                    );
                    
                    if let Some(device) = self.probe_network_device(IpAddr::V4(target_ip)).await {
                        devices.push(device);
                    }
                }
            }
            IpAddr::V6(_) => {
                // IPv6 scanning not implemented yet
                debug!("IPv6 scanning not yet implemented");
            }
        }
        
        Ok(devices)
    }
    
    async fn probe_network_device(&self, ip: IpAddr) -> Option<Arc<dyn EdgeDevice>> {
        // Try to connect to common ports
        for &port in &self.ports {
            let socket_addr = SocketAddr::new(ip, port);
            
            // Try to connect
            if let Ok(_stream) = tokio::time::timeout(
                self.timeout,
                tokio::net::TcpStream::connect(socket_addr)
            ).await {
                // Device is reachable, try to identify it
                if let Some(device) = self.identify_network_device(ip, port).await {
                    return Some(device);
                }
            }
        }
        
        None
    }
    
    async fn identify_network_device(&self, ip: IpAddr, port: u16) -> Option<Arc<dyn EdgeDevice>> {
        // Try to identify device type based on open ports and responses
        match port {
            22 => {
                // SSH port - likely Linux-based edge device
                debug!("Found SSH service on {}:{}", ip, port);
                // Could be Raspberry Pi or other Linux edge device
                // Implementation needed for RaspberryPiDevice
                None
            }
            80 | 8080 => {
                // HTTP port - could be ESP32 or other web-enabled device
                debug!("Found HTTP service on {}:{}", ip, port);
                None
            }
            _ => None,
        }
    }
}

#[async_trait::async_trait]
impl DiscoveryMethod for USBDiscovery {
    fn get_name(&self) -> &str {
        "USB Discovery"
    }
    
    async fn discover(&self) -> ToadStoolResult<Vec<Arc<dyn EdgeDevice>>> {
        // USB discovery is largely covered by serial port discovery
        // This could be extended to handle other USB device types
        Ok(Vec::new())
    }
    
    async fn is_available(&self) -> bool {
        // Check if USB subsystem is available
        true
    }
    
    fn get_supported_types(&self) -> Vec<String> {
        vec![
            "USB Device".to_string(),
        ]
    }
}

#[async_trait::async_trait]
impl DiscoveryMethod for BluetoothDiscovery {
    fn get_name(&self) -> &str {
        "Bluetooth Discovery"
    }
    
    async fn discover(&self) -> ToadStoolResult<Vec<Arc<dyn EdgeDevice>>> {
        if !self.is_available().await {
            debug!("No Bluetooth adapters found via sysfs");
            return Ok(Vec::new());
        }
        // Bluetooth device discovery requires pairing and service enumeration,
        // which depends on the BlueZ D-Bus API or direct HCI commands.
        // Return empty for now — the adapter check above is the real evolution.
        debug!("Bluetooth adapter present but device enumeration requires BlueZ D-Bus integration");
        Ok(Vec::new())
    }
    
    async fn is_available(&self) -> bool {
        // Probe for Bluetooth adapters via sysfs (pure Rust, no libc)
        let bt_class = std::path::Path::new("/sys/class/bluetooth");
        if !bt_class.exists() {
            return false;
        }
        match std::fs::read_dir(bt_class) {
            Ok(entries) => entries.filter_map(|e| e.ok()).next().is_some(),
            Err(_) => false,
        }
    }
    
    fn get_supported_types(&self) -> Vec<String> {
        vec![
            "Bluetooth Device".to_string(),
            "ESP32 Bluetooth".to_string(),
        ]
    }
}

#[async_trait::async_trait]
impl DiscoveryMethod for MDNSDiscovery {
    fn get_name(&self) -> &str {
        "mDNS Discovery"
    }

    async fn discover(&self) -> ToadStoolResult<Vec<Arc<dyn EdgeDevice>>> {
        // Primary: filesystem-based discovery for edge devices registering via biomeOS runtime
        // Edge devices on the same host (e.g. Raspberry Pi running ToadStool) register sockets
        if let Some(devices) = self.discover_via_filesystem().await? {
            if !devices.is_empty() {
                return Ok(devices);
            }
        }

        // Fallback: scan for _toadstool-edge._tcp service type via filesystem polling
        // Devices can register by creating JSON descriptor in $XDG_RUNTIME_DIR/edge-devices/
        if let Some(devices) = self.discover_via_edge_registry().await? {
            if !devices.is_empty() {
                return Ok(devices);
            }
        }

        debug!("mDNS/filesystem discovery: no edge devices found");
        Ok(Vec::new())
    }

    async fn is_available(&self) -> bool {
        // Always available: filesystem-based discovery works without mDNS libraries
        true
    }

    fn get_supported_types(&self) -> Vec<String> {
        vec![
            "mDNS Device".to_string(),
            "Network Service".to_string(),
            "ToadStool Edge".to_string(),
        ]
    }
}

impl MDNSDiscovery {
    /// Discover edge devices via biomeOS runtime directory (Unix sockets for edge proxies)
    async fn discover_via_filesystem(&self) -> ToadStoolResult<Option<Vec<Arc<dyn EdgeDevice>>>> {
        let biomeos_dir = toadstool::platform_paths::biomeos_runtime_dir();

        if !biomeos_dir.exists() {
            return Ok(None);
        }

        let mut devices = Vec::new();
        let mut entries = match tokio::fs::read_dir(&biomeos_dir).await {
            Ok(e) => e,
            Err(_) => return Ok(None),
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "sock") {
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                if stem == "toadstool-edge" || stem.ends_with("-edge") {
                    debug!("Found edge device socket: {}", path.display());
                    if let Some(device) = self.create_edge_device_from_socket(&path).await {
                        devices.push(device);
                    }
                }
            }
        }

        if devices.is_empty() {
            Ok(None)
        } else {
            Ok(Some(devices))
        }
    }

    /// Discover via edge device registry ($XDG_RUNTIME_DIR/edge-devices/*.json)
    async fn discover_via_edge_registry(&self) -> ToadStoolResult<Option<Vec<Arc<dyn EdgeDevice>>>> {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .ok()
            .or_else(|| std::env::var("BIOMEOS_RUNTIME_DIR").ok())
            .unwrap_or_else(|| std::env::temp_dir().join("toadstool-runtime").to_string_lossy().into_owned());

        let edge_dir = std::path::PathBuf::from(&runtime_dir).join("edge-devices");
        if !edge_dir.exists() {
            return Ok(None);
        }

        let mut devices = Vec::new();
        let mut entries = match tokio::fs::read_dir(&edge_dir).await {
            Ok(e) => e,
            Err(_) => return Ok(None),
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "json") {
                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                    if let Ok(device) = self.parse_edge_registry_entry(&content) {
                        devices.push(device);
                    }
                }
            }
        }

        if devices.is_empty() {
            Ok(None)
        } else {
            Ok(Some(devices))
        }
    }

    async fn create_edge_device_from_socket(
        &self,
        path: &std::path::Path,
    ) -> Option<Arc<dyn EdgeDevice>> {
        if !path.exists() {
            debug!("Edge socket {:?} no longer exists, skipping", path);
            return None;
        }
        let device = LinuxEdgeDevice::from_socket_path(path.to_path_buf());
        info!("Created LinuxEdge device from socket: {:?}", path);
        Some(Arc::new(device))
    }

    fn parse_edge_registry_entry(&self, content: &str) -> ToadStoolResult<Arc<dyn EdgeDevice>> {
        let device = LinuxEdgeDevice::from_registry_json(content)?;
        info!("Parsed edge registry entry: {}", device.get_info().name);
        Ok(Arc::new(device))
    }
} 