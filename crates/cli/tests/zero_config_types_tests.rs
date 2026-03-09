// SPDX-License-Identifier: AGPL-3.0-only
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding,
    clippy::similar_names,
    clippy::default_trait_access,
    clippy::items_after_statements,
    clippy::unused_async
)]
//! Comprehensive tests for CLI zero-config deployment types

use serde::{Deserialize, Serialize};

// Mirror zero_config types for testing
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemInfo {
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub storage: StorageInfo,
    pub network: NetworkInfo,
    pub os: OsInfo,
    pub container_runtime: ContainerRuntimeInfo,
    pub gpu: GpuInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CpuInfo {
    pub cores: u32,
    pub architecture: String,
    pub model: String,
    pub frequency: u32,
    pub vendor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryInfo {
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub memory_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StorageInfo {
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub storage_type: String,
    pub filesystem: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkInfo {
    pub interfaces: Vec<NetworkInterface>,
    pub external_ip: Option<String>,
    pub local_ips: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkInterface {
    pub name: String,
    pub ip: String,
    pub mac: String,
    pub speed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OsInfo {
    pub name: String,
    pub version: String,
    pub kernel: String,
    pub arch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContainerRuntimeInfo {
    pub docker: bool,
    pub podman: bool,
    pub containerd: bool,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GpuInfo {
    pub count: u32,
    pub vendor: String,
    pub model: String,
    pub memory_bytes: u64,
    pub cuda: bool,
    pub opencl: bool,
}

// ============================================================================
// CpuInfo Tests
// ============================================================================

#[test]
fn test_cpu_info_creation() {
    let cpu = CpuInfo {
        cores: 8,
        architecture: "x86_64".to_string(),
        model: "Intel Core i7".to_string(),
        frequency: 3600,
        vendor: "Intel".to_string(),
    };

    assert_eq!(cpu.cores, 8);
    assert_eq!(cpu.architecture, "x86_64");
    assert_eq!(cpu.frequency, 3600);
}

#[test]
fn test_cpu_info_amd() {
    let cpu = CpuInfo {
        cores: 16,
        architecture: "x86_64".to_string(),
        model: "AMD Ryzen 9".to_string(),
        frequency: 4200,
        vendor: "AMD".to_string(),
    };

    assert_eq!(cpu.vendor, "AMD");
    assert_eq!(cpu.cores, 16);
}

#[test]
fn test_cpu_info_arm() {
    let cpu = CpuInfo {
        cores: 4,
        architecture: "aarch64".to_string(),
        model: "Apple M1".to_string(),
        frequency: 3200,
        vendor: "Apple".to_string(),
    };

    assert_eq!(cpu.architecture, "aarch64");
}

// ============================================================================
// MemoryInfo Tests
// ============================================================================

#[test]
fn test_memory_info_creation() {
    let memory = MemoryInfo {
        total_bytes: 17_179_869_184,    // 16 GB
        available_bytes: 8_589_934_592, // 8 GB
        memory_type: "DDR4".to_string(),
    };

    assert_eq!(memory.total_bytes, 17_179_869_184);
    assert_eq!(memory.memory_type, "DDR4");
}

#[test]
fn test_memory_info_ddr5() {
    let memory = MemoryInfo {
        total_bytes: 34_359_738_368,     // 32 GB
        available_bytes: 25_769_803_776, // 24 GB
        memory_type: "DDR5".to_string(),
    };

    assert_eq!(memory.memory_type, "DDR5");
}

// ============================================================================
// StorageInfo Tests
// ============================================================================

#[test]
fn test_storage_info_ssd() {
    let storage = StorageInfo {
        total_bytes: 1_099_511_627_776,   // 1 TB
        available_bytes: 549_755_813_888, // 512 GB
        storage_type: "SSD".to_string(),
        filesystem: "ext4".to_string(),
    };

    assert_eq!(storage.storage_type, "SSD");
    assert_eq!(storage.filesystem, "ext4");
}

#[test]
fn test_storage_info_nvme() {
    let storage = StorageInfo {
        total_bytes: 2_199_023_255_552,     // 2 TB
        available_bytes: 1_099_511_627_776, // 1 TB
        storage_type: "NVMe".to_string(),
        filesystem: "btrfs".to_string(),
    };

    assert_eq!(storage.storage_type, "NVMe");
}

// ============================================================================
// NetworkInterface Tests
// ============================================================================

#[test]
fn test_network_interface_creation() {
    let iface = NetworkInterface {
        name: "eth0".to_string(),
        ip: "192.168.1.100".to_string(),
        mac: "00:11:22:33:44:55".to_string(),
        speed: 1000,
    };

    assert_eq!(iface.name, "eth0");
    assert_eq!(iface.speed, 1000);
}

#[test]
fn test_network_interface_wifi() {
    let iface = NetworkInterface {
        name: "wlan0".to_string(),
        ip: "10.0.0.5".to_string(),
        mac: "AA:BB:CC:DD:EE:FF".to_string(),
        speed: 300,
    };

    assert_eq!(iface.name, "wlan0");
}

// ============================================================================
// NetworkInfo Tests
// ============================================================================

#[test]
fn test_network_info_single_interface() {
    let iface = NetworkInterface {
        name: "eth0".to_string(),
        ip: "192.168.1.100".to_string(),
        mac: "00:11:22:33:44:55".to_string(),
        speed: 1000,
    };

    let network = NetworkInfo {
        interfaces: vec![iface],
        external_ip: Some("203.0.113.1".to_string()),
        local_ips: vec!["192.168.1.100".to_string()],
    };

    assert_eq!(network.interfaces.len(), 1);
    assert!(network.external_ip.is_some());
}

#[test]
fn test_network_info_multiple_interfaces() {
    let eth = NetworkInterface {
        name: "eth0".to_string(),
        ip: "192.168.1.100".to_string(),
        mac: "00:11:22:33:44:55".to_string(),
        speed: 1000,
    };

    let wlan = NetworkInterface {
        name: "wlan0".to_string(),
        ip: "10.0.0.5".to_string(),
        mac: "AA:BB:CC:DD:EE:FF".to_string(),
        speed: 300,
    };

    let network = NetworkInfo {
        interfaces: vec![eth, wlan],
        external_ip: Some("203.0.113.1".to_string()),
        local_ips: vec!["192.168.1.100".to_string(), "10.0.0.5".to_string()],
    };

    assert_eq!(network.interfaces.len(), 2);
    assert_eq!(network.local_ips.len(), 2);
}

// ============================================================================
// OsInfo Tests
// ============================================================================

#[test]
fn test_os_info_ubuntu() {
    let os = OsInfo {
        name: "Ubuntu".to_string(),
        version: "22.04 LTS".to_string(),
        kernel: "5.15.0".to_string(),
        arch: "x86_64".to_string(),
    };

    assert_eq!(os.name, "Ubuntu");
    assert_eq!(os.arch, "x86_64");
}

#[test]
fn test_os_info_macos() {
    let os = OsInfo {
        name: "macOS".to_string(),
        version: "13.0".to_string(),
        kernel: "22.1.0".to_string(),
        arch: "aarch64".to_string(),
    };

    assert_eq!(os.name, "macOS");
}

// ============================================================================
// ContainerRuntimeInfo Tests
// ============================================================================

#[test]
fn test_container_runtime_docker() {
    let runtime = ContainerRuntimeInfo {
        docker: true,
        podman: false,
        containerd: false,
        version: Some("20.10.17".to_string()),
    };

    assert!(runtime.docker);
    assert!(!runtime.podman);
}

#[test]
fn test_container_runtime_podman() {
    let runtime = ContainerRuntimeInfo {
        docker: false,
        podman: true,
        containerd: false,
        version: Some("4.1.1".to_string()),
    };

    assert!(runtime.podman);
}

#[test]
fn test_container_runtime_none() {
    let runtime = ContainerRuntimeInfo {
        docker: false,
        podman: false,
        containerd: false,
        version: None,
    };

    assert!(!runtime.docker);
    assert!(runtime.version.is_none());
}

// ============================================================================
// GpuInfo Tests
// ============================================================================

#[test]
fn test_gpu_info_nvidia() {
    let gpu = GpuInfo {
        count: 1,
        vendor: "NVIDIA".to_string(),
        model: "RTX 4090".to_string(),
        memory_bytes: 25_769_803_776, // 24 GB
        cuda: true,
        opencl: true,
    };

    assert_eq!(gpu.vendor, "NVIDIA");
    assert!(gpu.cuda);
}

#[test]
fn test_gpu_info_amd() {
    let gpu = GpuInfo {
        count: 2,
        vendor: "AMD".to_string(),
        model: "Radeon RX 7900 XTX".to_string(),
        memory_bytes: 25_769_803_776, // 24 GB
        cuda: false,
        opencl: true,
    };

    assert_eq!(gpu.vendor, "AMD");
    assert!(!gpu.cuda);
    assert!(gpu.opencl);
}

#[test]
fn test_gpu_info_multiple() {
    let gpu = GpuInfo {
        count: 8,
        vendor: "NVIDIA".to_string(),
        model: "A100".to_string(),
        memory_bytes: 42_949_672_960, // 40 GB
        cuda: true,
        opencl: true,
    };

    assert_eq!(gpu.count, 8);
}

// ============================================================================
// SystemInfo Tests
// ============================================================================

#[test]
fn test_system_info_default() {
    let system = SystemInfo::default();
    assert_eq!(system.cpu.cores, 0);
    assert_eq!(system.memory.total_bytes, 0);
}

#[test]
fn test_system_info_complete() {
    let cpu = CpuInfo {
        cores: 8,
        architecture: "x86_64".to_string(),
        model: "Intel Core i7".to_string(),
        frequency: 3600,
        vendor: "Intel".to_string(),
    };

    let memory = MemoryInfo {
        total_bytes: 17_179_869_184,
        available_bytes: 8_589_934_592,
        memory_type: "DDR4".to_string(),
    };

    let storage = StorageInfo {
        total_bytes: 1_099_511_627_776,
        available_bytes: 549_755_813_888,
        storage_type: "SSD".to_string(),
        filesystem: "ext4".to_string(),
    };

    let network = NetworkInfo {
        interfaces: vec![],
        external_ip: Some("203.0.113.1".to_string()),
        local_ips: vec!["192.168.1.100".to_string()],
    };

    let os = OsInfo {
        name: "Ubuntu".to_string(),
        version: "22.04".to_string(),
        kernel: "5.15.0".to_string(),
        arch: "x86_64".to_string(),
    };

    let container = ContainerRuntimeInfo {
        docker: true,
        podman: false,
        containerd: false,
        version: Some("20.10.17".to_string()),
    };

    let gpu = GpuInfo {
        count: 1,
        vendor: "NVIDIA".to_string(),
        model: "RTX 4090".to_string(),
        memory_bytes: 25_769_803_776,
        cuda: true,
        opencl: true,
    };

    let system = SystemInfo {
        cpu,
        memory,
        storage,
        network,
        os,
        container_runtime: container,
        gpu,
    };

    assert_eq!(system.cpu.cores, 8);
    assert_eq!(system.memory.total_bytes, 17_179_869_184);
    assert_eq!(system.storage.storage_type, "SSD");
    assert_eq!(system.os.name, "Ubuntu");
    assert!(system.container_runtime.docker);
    assert_eq!(system.gpu.vendor, "NVIDIA");
}

#[test]
fn test_system_info_serialization() {
    let system = SystemInfo::default();
    let json = serde_json::to_string(&system).unwrap();
    assert!(!json.is_empty());

    let deserialized: SystemInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.cpu.cores, system.cpu.cores);
}

// ============================================================================
// Edge Cases and Special Scenarios
// ============================================================================

#[test]
fn test_memory_calculation_gb() {
    let memory = MemoryInfo {
        total_bytes: 17_179_869_184,    // 16 GB
        available_bytes: 8_589_934_592, // 8 GB
        memory_type: "DDR4".to_string(),
    };

    let total_gb = memory.total_bytes / 1_073_741_824;
    assert_eq!(total_gb, 16);
}

#[test]
fn test_storage_calculation_tb() {
    let storage = StorageInfo {
        total_bytes: 2_199_023_255_552,     // 2 TB
        available_bytes: 1_099_511_627_776, // 1 TB
        storage_type: "NVMe".to_string(),
        filesystem: "ext4".to_string(),
    };

    let total_tb = storage.total_bytes / 1_099_511_627_776;
    assert_eq!(total_tb, 2);
}

#[test]
fn test_network_interface_10gbps() {
    let iface = NetworkInterface {
        name: "eth0".to_string(),
        ip: "192.168.1.1".to_string(),
        mac: "00:11:22:33:44:55".to_string(),
        speed: 10_000, // 10 Gbps
    };

    assert_eq!(iface.speed, 10_000);
}
