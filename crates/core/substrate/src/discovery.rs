//! Hardware Discovery and Auto-Configuration
//!
//! Deep Debt: ToadStool discovers and configures hardware at runtime
//! - No scripts needed
//! - No sudo required
//! - Self-evolves when hardware changes
//! - Userspace drivers by default

use anyhow::{Context, Result, bail};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Hardware substrate that ToadStool discovered and configured
#[derive(Debug, Clone)]
pub struct DiscoveredSubstrate {
    pub substrate_type: SubstrateType,
    pub device_id: String,
    pub capabilities: SubstrateCapabilities,
    pub backend: BackendType,
}

/// Type of compute substrate
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubstrateType {
    /// GPU (NVIDIA, AMD, Intel)
    Gpu { vendor: String, model: String },
    /// NPU (Akida, Loihi, etc.)
    Npu { vendor: String, model: String },
    /// CPU
    Cpu { vendor: String, model: String },
    /// FPGA
    Fpga { vendor: String, model: String },
}

/// Backend type for accessing hardware
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendType {
    /// Userspace driver (no sudo, always works)
    Userspace,
    /// Kernel driver (if available)
    Kernel,
}

/// Capabilities discovered from hardware
#[derive(Debug, Clone)]
pub struct SubstrateCapabilities {
    pub compute_units: u32,
    pub memory_mb: u64,
    pub supports_neural: bool,
    pub supports_compute: bool,
    pub power_mw: Option<u32>,
}

/// Hardware Discovery Engine
///
/// Deep Debt: ToadStool discovers hardware without external tools
#[derive(Debug)]
pub struct HardwareDiscovery {
    discovered: HashMap<String, DiscoveredSubstrate>,
}

impl HardwareDiscovery {
    /// Create new discovery engine
    pub fn new() -> Self {
        Self {
            discovered: HashMap::new(),
        }
    }

    /// Discover all available hardware
    ///
    /// Deep Debt: Pure Rust, no scripts, no sudo
    pub fn discover_all(&mut self) -> Result<Vec<DiscoveredSubstrate>> {
        info!("🔍 ToadStool hardware discovery starting...");

        // Discover NPUs
        self.discover_npus()?;

        // Discover GPUs
        self.discover_gpus()?;

        // Discover CPUs
        self.discover_cpus()?;

        // Discover FPGAs
        self.discover_fpgas()?;

        let substrates: Vec<_> = self.discovered.values().cloned().collect();
        info!("✓ Discovered {} substrate(s)", substrates.len());

        Ok(substrates)
    }

    /// Discover Akida NPUs
    ///
    /// Deep Debt: Direct PCIe discovery, no kernel driver required
    fn discover_npus(&mut self) -> Result<()> {
        info!("  🧠 Scanning for NPUs...");

        // Scan PCIe bus for Akida devices
        let pcie_devices = self.scan_pcie_bus()?;

        for device in pcie_devices {
            if self.is_akida_device(&device)? {
                info!("    ✓ Found Akida NPU: {}", device.address);

                // Check if userspace access is available
                if let Ok(substrate) = self.configure_akida_userspace(&device) {
                    self.discovered.insert(device.address.clone(), substrate);
                } else if let Ok(substrate) = self.configure_akida_kernel(&device) {
                    self.discovered.insert(device.address.clone(), substrate);
                } else {
                    warn!("    ⚠ Akida NPU found but not accessible: {}", device.address);
                }
            }
        }

        Ok(())
    }

    /// Configure Akida NPU with userspace driver
    ///
    /// Deep Debt: No sudo, no kernel module, just works
    fn configure_akida_userspace(&self, device: &PcieDevice) -> Result<DiscoveredSubstrate> {
        // Check if PCIe resources are accessible
        let resource_path = format!("/sys/bus/pci/devices/{}/resource0", device.address);
        if !Path::new(&resource_path).exists() {
            bail!("PCIe resource not accessible");
        }

        // Try to read device ID register (no sudo needed)
        if !self.can_read_pcie_resource(&device.address)? {
            bail!("PCIe resource not readable");
        }

        info!("    ✓ Userspace driver available (no sudo needed)");

        // Discover capabilities by reading hardware registers
        let capabilities = self.discover_akida_capabilities(&device.address)?;

        Ok(DiscoveredSubstrate {
            substrate_type: SubstrateType::Npu {
                vendor: "BrainChip".to_string(),
                model: format!("Akida {}", capabilities.compute_units),
            },
            device_id: device.address.clone(),
            capabilities,
            backend: BackendType::Userspace,
        })
    }

    /// Configure Akida NPU with kernel driver (if available)
    fn configure_akida_kernel(&self, device: &PcieDevice) -> Result<DiscoveredSubstrate> {
        // Check if /dev/akida* exists
        let dev_path = format!("/dev/akida{}", device.index.unwrap_or(0));
        if !Path::new(&dev_path).exists() {
            bail!("Kernel driver not available");
        }

        info!("    ✓ Kernel driver available");

        // Use existing device manager
        use crate::neuromorphic::akida_driver::DeviceManager;
        let mut manager = DeviceManager::new();
        let devices = manager.discover()?;

        if let Some(device_info) = devices.first() {
            let capabilities = SubstrateCapabilities {
                compute_units: device_info.capabilities().npu_count,
                memory_mb: device_info.capabilities().memory_mb as u64,
                supports_neural: true,
                supports_compute: false,
                power_mw: device_info.capabilities().power_mw,
            };

            Ok(DiscoveredSubstrate {
                substrate_type: SubstrateType::Npu {
                    vendor: "BrainChip".to_string(),
                    model: format!("Akida {:?}", device_info.capabilities().chip_version),
                },
                device_id: dev_path,
                capabilities,
                backend: BackendType::Kernel,
            })
        } else {
            bail!("No Akida devices found via kernel driver");
        }
    }

    /// Discover Akida capabilities by reading hardware registers
    ///
    /// Deep Debt: Runtime discovery from hardware, no hardcoding
    fn discover_akida_capabilities(&self, pcie_address: &str) -> Result<SubstrateCapabilities> {
        use crate::neuromorphic::akida_driver::backends::UserspaceBackend;

        // Initialize userspace backend (no sudo)
        let backend = UserspaceBackend::init(pcie_address)
            .context("Failed to initialize userspace backend")?;

        // Query capabilities from hardware
        let caps = backend.capabilities();

        Ok(SubstrateCapabilities {
            compute_units: caps.npu_count,
            memory_mb: caps.memory_mb as u64,
            supports_neural: true,
            supports_compute: false,
            power_mw: caps.power_mw,
        })
    }

    /// Discover GPUs
    fn discover_gpus(&mut self) -> Result<()> {
        info!("  🎮 Scanning for GPUs...");

        // Use existing GPU detection from barracuda
        use crate::barracuda::DeviceDiscovery;

        match DeviceDiscovery::discover_all() {
            Ok(devices) => {
                for device in devices {
                    let device_id = format!("gpu-{}", device.id);
                    let substrate = DiscoveredSubstrate {
                        substrate_type: SubstrateType::Gpu {
                            vendor: device.vendor.clone(),
                            model: device.name.clone(),
                        },
                        device_id: device_id.clone(),
                        capabilities: SubstrateCapabilities {
                            compute_units: device.compute_units,
                            memory_mb: device.memory_bytes / (1024 * 1024),
                            supports_neural: true,
                            supports_compute: true,
                            power_mw: None,
                        },
                        backend: BackendType::Userspace, // WebGPU is userspace
                    };

                    info!("    ✓ Found GPU: {} {}", device.vendor, device.name);
                    self.discovered.insert(device_id, substrate);
                }
            }
            Err(e) => {
                warn!("  ⚠ GPU discovery failed: {}", e);
            }
        }

        Ok(())
    }

    /// Discover CPUs
    fn discover_cpus(&mut self) -> Result<()> {
        info!("  💻 Scanning for CPUs...");

        // Read from /proc/cpuinfo
        if let Ok(cpuinfo) = std::fs::read_to_string("/proc/cpuinfo") {
            if let Some(model) = cpuinfo.lines()
                .find(|line| line.starts_with("model name"))
                .and_then(|line| line.split(':').nth(1))
            {
                let model = model.trim().to_string();
                let vendor = if model.contains("Intel") {
                    "Intel"
                } else if model.contains("AMD") {
                    "AMD"
                } else {
                    "Unknown"
                }.to_string();

                // Evolved from num_cpus::get() to std::thread::available_parallelism() (Rust 1.59+)
                let cores = std::thread::available_parallelism()
                    .map(|p| p.get() as u32)
                    .unwrap_or(1);

                let substrate = DiscoveredSubstrate {
                    substrate_type: SubstrateType::Cpu {
                        vendor: vendor.clone(),
                        model: model.clone(),
                    },
                    device_id: "cpu-0".to_string(),
                    capabilities: SubstrateCapabilities {
                        compute_units: cores,
                        memory_mb: self.get_system_memory_mb(),
                        supports_neural: false,
                        supports_compute: true,
                        power_mw: None,
                    },
                    backend: BackendType::Userspace,
                };

                info!("    ✓ Found CPU: {} {} ({} cores)", vendor, model, cores);
                self.discovered.insert("cpu-0".to_string(), substrate);
            }
        }

        Ok(())
    }

    /// Discover FPGAs
    ///
    /// **Deep Debt Status**: Placeholder — requires FPGA hardware for development
    ///
    /// ## Implementation Plan
    ///
    /// When FPGA hardware is available, implement discovery via:
    /// 1. **Intel FPGAs**: Query OPAE (Open Programmable Acceleration Engine) via sysfs
    ///    - `/sys/class/fpga_region/` for Intel PAC cards
    /// 2. **Xilinx FPGAs**: Query XRT (Xilinx Runtime) via sysfs
    ///    - `/sys/bus/pci/drivers/xclmgmt/` for Alveo cards
    /// 3. **Generic**: PCIe device class 0x1200 (Processing Accelerator)
    ///
    /// ## Current Behavior
    ///
    /// Returns `Ok(())` with no FPGAs discovered. This allows the system to
    /// function on machines without FPGA hardware while preserving the
    /// discovery API for future implementation.
    fn discover_fpgas(&mut self) -> Result<()> {
        info!("  🔌 Scanning for FPGAs...");

        // Deep Debt: FPGA discovery requires hardware access
        // Currently no-op; will implement when FPGA hardware available
        //
        // Implementation paths:
        // - Intel OPAE: check /sys/class/fpga_region/
        // - Xilinx XRT: check /sys/bus/pci/drivers/xclmgmt/
        // - Generic: PCIe class 0x1200

        debug!("    FPGA discovery not yet implemented (no hardware available)");
        Ok(())
    }

    /// Scan PCIe bus for devices
    ///
    /// Deep Debt: Direct sysfs access, no lspci command
    fn scan_pcie_bus(&self) -> Result<Vec<PcieDevice>> {
        let mut devices = Vec::new();
        let pcie_path = Path::new("/sys/bus/pci/devices");

        if !pcie_path.exists() {
            return Ok(devices);
        }

        for entry in std::fs::read_dir(pcie_path)? {
            let entry = entry?;
            let address = entry.file_name().to_string_lossy().to_string();

            // Read vendor and device IDs
            let vendor_path = entry.path().join("vendor");
            let device_path = entry.path().join("device");

            if let (Ok(vendor), Ok(device_id)) = (
                std::fs::read_to_string(&vendor_path),
                std::fs::read_to_string(&device_path),
            ) {
                devices.push(PcieDevice {
                    address,
                    vendor_id: vendor.trim().to_string(),
                    device_id: device_id.trim().to_string(),
                    index: None,
                });
            }
        }

        Ok(devices)
    }

    /// Check if device is Akida NPU
    fn is_akida_device(&self, device: &PcieDevice) -> Result<bool> {
        // BrainChip vendor ID: 0x1e7c
        // AKD1000: 0xbca1
        // AKD1500: 0xbca2
        Ok(device.vendor_id == "0x1e7c" &&
           (device.device_id == "0xbca1" || device.device_id == "0xbca2"))
    }

    /// Check if PCIe resource is readable (no sudo needed)
    fn can_read_pcie_resource(&self, address: &str) -> Result<bool> {
        let resource_path = format!("/sys/bus/pci/devices/{}/resource0", address);
        let path = Path::new(&resource_path);

        if !path.exists() {
            return Ok(false);
        }

        // Try to open for reading
        Ok(std::fs::File::open(path).is_ok())
    }

    /// Get system memory in MB
    fn get_system_memory_mb(&self) -> u64 {
        if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
            if let Some(line) = meminfo.lines().find(|l| l.starts_with("MemTotal:")) {
                if let Some(kb) = line.split_whitespace().nth(1) {
                    if let Ok(kb) = kb.parse::<u64>() {
                        return kb / 1024; // Convert KB to MB
                    }
                }
            }
        }
        0
    }

    /// React to hardware changes
    ///
    /// Deep Debt: ToadStool adapts when hardware is added/removed
    pub fn refresh(&mut self) -> Result<Vec<HardwareChange>> {
        let before = self.discovered.keys().cloned().collect::<Vec<_>>();

        // Re-discover all hardware
        self.discovered.clear();
        self.discover_all()?;

        let after = self.discovered.keys().cloned().collect::<Vec<_>>();

        // Detect changes
        let mut changes = Vec::new();

        // Detect additions
        for device_id in &after {
            if !before.contains(device_id) {
                changes.push(HardwareChange::Added {
                    device_id: device_id.clone(),
                    substrate: self.discovered[device_id].clone(),
                });
            }
        }

        // Detect removals
        for device_id in &before {
            if !after.contains(device_id) {
                changes.push(HardwareChange::Removed {
                    device_id: device_id.clone(),
                });
            }
        }

        if !changes.is_empty() {
            info!("🔄 Hardware changed: {} event(s)", changes.len());
        }

        Ok(changes)
    }
}

impl Default for HardwareDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

/// Hardware change event
#[derive(Debug, Clone)]
pub enum HardwareChange {
    Added {
        device_id: String,
        substrate: DiscoveredSubstrate,
    },
    Removed {
        device_id: String,
    },
}

/// PCIe device info
#[derive(Debug, Clone)]
struct PcieDevice {
    address: String,
    vendor_id: String,
    device_id: String,
    index: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardware_discovery() {
        let mut discovery = HardwareDiscovery::new();
        let substrates = discovery.discover_all().unwrap();

        // Should discover at least CPU
        assert!(!substrates.is_empty());
        assert!(substrates.iter().any(|s| matches!(
            s.substrate_type,
            SubstrateType::Cpu { .. }
        )));
    }

    #[test]
    fn test_pcie_scan() {
        let discovery = HardwareDiscovery::new();
        let devices = discovery.scan_pcie_bus().unwrap();

        // PCIe bus should have devices
        println!("Found {} PCIe devices", devices.len());
    }
}
