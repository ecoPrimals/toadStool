// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests for `zero_config` module
//!
//! This module provides comprehensive test coverage for the `zero_config` module's
//! core data structures. These tests verify basic functionality including
//! struct creation, serialization, and deserialization.

use toadstool_cli::zero_config::*;

#[cfg(test)]
mod cpu_info_tests {
    use super::*;

    #[test]
    fn test_cpu_info_creation() {
        let cpu = CpuInfo {
            cores: 8,
            architecture: "x86_64".to_string(),
            model: "AMD Ryzen 7 5800X".to_string(),
            frequency: 3800,
            vendor: "AMD".to_string(),
        };

        assert_eq!(cpu.cores, 8);
        assert_eq!(cpu.architecture, "x86_64");
        assert_eq!(cpu.model, "AMD Ryzen 7 5800X");
        assert_eq!(cpu.frequency, 3800);
        assert_eq!(cpu.vendor, "AMD");
    }

    #[test]
    fn test_cpu_info_arm_architecture() {
        let cpu = CpuInfo {
            cores: 8,
            architecture: "aarch64".to_string(),
            model: "Apple M1".to_string(),
            frequency: 3200,
            vendor: "Apple".to_string(),
        };

        assert_eq!(cpu.architecture, "aarch64");
        assert_eq!(cpu.vendor, "Apple");
    }

    #[test]
    fn test_cpu_info_serialization() {
        let cpu = CpuInfo {
            cores: 16,
            architecture: "x86_64".to_string(),
            model: "Intel Xeon".to_string(),
            frequency: 2400,
            vendor: "Intel".to_string(),
        };

        let json = serde_json::to_string(&cpu).expect("Failed to serialize");
        assert!(json.contains("x86_64"));
        assert!(json.contains("Intel Xeon"));
        assert!(json.contains("2400"));
    }

    #[test]
    fn test_cpu_info_deserialization() {
        let json = r#"{
            "cores": 4,
            "architecture": "riscv64",
            "model": "SiFive U74",
            "frequency": 1500,
            "vendor": "SiFive"
        }"#;

        let cpu: CpuInfo = serde_json::from_str(json).expect("Failed to deserialize");

        assert_eq!(cpu.cores, 4);
        assert_eq!(cpu.architecture, "riscv64");
        assert_eq!(cpu.model, "SiFive U74");
        assert_eq!(cpu.frequency, 1500);
        assert_eq!(cpu.vendor, "SiFive");
    }

    #[test]
    fn test_cpu_info_debug_format() {
        let cpu = CpuInfo {
            cores: 8,
            architecture: "x86_64".to_string(),
            model: "Test CPU".to_string(),
            frequency: 3000,
            vendor: "TestVendor".to_string(),
        };

        let debug_str = format!("{cpu:?}");
        assert!(debug_str.contains("CpuInfo"));
        assert!(debug_str.contains('8'));
        assert!(debug_str.contains("x86_64"));
    }
}

#[cfg(test)]
mod memory_info_tests {
    use super::*;

    #[test]
    fn test_memory_info_creation() {
        let memory = MemoryInfo {
            total_bytes: 16_000_000_000,
            available_bytes: 8_000_000_000,
            memory_type: "DDR4".to_string(),
        };

        assert_eq!(memory.total_bytes, 16_000_000_000);
        assert_eq!(memory.available_bytes, 8_000_000_000);
        assert_eq!(memory.memory_type, "DDR4");
        assert!(memory.available_bytes <= memory.total_bytes);
    }

    #[test]
    fn test_memory_info_with_ddr5() {
        let memory = MemoryInfo {
            total_bytes: 32_000_000_000,
            available_bytes: 28_000_000_000,
            memory_type: "DDR5".to_string(),
        };

        assert_eq!(memory.memory_type, "DDR5");
        assert_eq!(memory.total_bytes, 32_000_000_000);
    }

    #[test]
    fn test_memory_info_serialization() {
        let memory = MemoryInfo {
            total_bytes: 8_000_000_000,
            available_bytes: 4_000_000_000,
            memory_type: "DDR4".to_string(),
        };

        let json = serde_json::to_string(&memory).expect("Failed to serialize");
        assert!(json.contains("8000000000"));
        assert!(json.contains("DDR4"));
    }

    #[test]
    fn test_memory_info_deserialization() {
        let json = r#"{
            "total_bytes": 64000000000,
            "available_bytes": 32000000000,
            "memory_type": "DDR5"
        }"#;

        let memory: MemoryInfo = serde_json::from_str(json).expect("Failed to deserialize");

        assert_eq!(memory.total_bytes, 64_000_000_000);
        assert_eq!(memory.available_bytes, 32_000_000_000);
        assert_eq!(memory.memory_type, "DDR5");
    }

    #[test]
    fn test_memory_info_clone() {
        let memory1 = MemoryInfo {
            total_bytes: 16_000_000_000,
            available_bytes: 8_000_000_000,
            memory_type: "DDR4".to_string(),
        };

        let memory2 = memory1.clone();
        assert_eq!(memory1.total_bytes, memory2.total_bytes);
        assert_eq!(memory1.available_bytes, memory2.available_bytes);
        assert_eq!(memory1.memory_type, memory2.memory_type);
    }
}

#[cfg(test)]
mod storage_info_tests {
    use super::*;

    #[test]
    fn test_storage_info_creation() {
        let storage = StorageInfo {
            total_bytes: 1_000_000_000_000,
            available_bytes: 500_000_000_000,
            storage_type: "NVMe SSD".to_string(),
            filesystem: "ext4".to_string(),
        };

        assert_eq!(storage.total_bytes, 1_000_000_000_000);
        assert_eq!(storage.available_bytes, 500_000_000_000);
        assert_eq!(storage.storage_type, "NVMe SSD");
        assert_eq!(storage.filesystem, "ext4");
        assert!(storage.available_bytes <= storage.total_bytes);
    }

    #[test]
    fn test_storage_info_with_btrfs() {
        let storage = StorageInfo {
            total_bytes: 2_000_000_000_000,
            available_bytes: 1_500_000_000_000,
            storage_type: "SSD".to_string(),
            filesystem: "btrfs".to_string(),
        };

        assert_eq!(storage.filesystem, "btrfs");
    }

    #[test]
    fn test_storage_info_with_zfs() {
        let storage = StorageInfo {
            total_bytes: 4_000_000_000_000,
            available_bytes: 3_000_000_000_000,
            storage_type: "HDD".to_string(),
            filesystem: "zfs".to_string(),
        };

        assert_eq!(storage.filesystem, "zfs");
        assert_eq!(storage.storage_type, "HDD");
    }

    #[test]
    fn test_storage_info_serialization() {
        let storage = StorageInfo {
            total_bytes: 500_000_000_000,
            available_bytes: 250_000_000_000,
            storage_type: "NVMe".to_string(),
            filesystem: "xfs".to_string(),
        };

        let json = serde_json::to_string(&storage).expect("Failed to serialize");
        assert!(json.contains("NVMe"));
        assert!(json.contains("xfs"));
    }

    #[test]
    fn test_storage_info_deserialization() {
        let json = r#"{
            "total_bytes": 1000000000000,
            "available_bytes": 750000000000,
            "storage_type": "SSD",
            "filesystem": "ext4"
        }"#;

        let storage: StorageInfo = serde_json::from_str(json).expect("Failed to deserialize");

        assert_eq!(storage.total_bytes, 1_000_000_000_000);
        assert_eq!(storage.available_bytes, 750_000_000_000);
        assert_eq!(storage.storage_type, "SSD");
        assert_eq!(storage.filesystem, "ext4");
    }
}

#[cfg(test)]
mod network_interface_tests {
    use super::*;

    #[test]
    fn test_network_interface_creation() {
        let interface = NetworkInterface {
            name: "eth0".to_string(),
            ip: "192.168.1.100".to_string(),
            mac: "00:11:22:33:44:55".to_string(),
            speed: 1000,
        };

        assert_eq!(interface.name, "eth0");
        assert_eq!(interface.ip, "192.168.1.100");
        assert_eq!(interface.mac, "00:11:22:33:44:55");
        assert_eq!(interface.speed, 1000);
    }

    #[test]
    fn test_network_interface_with_ipv6() {
        let interface = NetworkInterface {
            name: "eth0".to_string(),
            ip: "fe80::1".to_string(),
            mac: "aa:bb:cc:dd:ee:ff".to_string(),
            speed: 10000,
        };

        assert_eq!(interface.ip, "fe80::1");
        assert_eq!(interface.speed, 10000);
    }

    #[test]
    fn test_network_interface_serialization() {
        let interface = NetworkInterface {
            name: "wlan0".to_string(),
            ip: "10.0.0.5".to_string(),
            mac: "12:34:56:78:90:ab".to_string(),
            speed: 100,
        };

        let json = serde_json::to_string(&interface).expect("Failed to serialize");
        assert!(json.contains("wlan0"));
        assert!(json.contains("10.0.0.5"));
    }
}

#[cfg(test)]
mod os_info_tests {
    use super::*;

    #[test]
    fn test_os_info_creation() {
        let os = OsInfo {
            name: "Linux".to_string(),
            version: "5.15.0".to_string(),
            kernel: "5.15.0-generic".to_string(),
            arch: "x86_64".to_string(),
        };

        assert_eq!(os.name, "Linux");
        assert_eq!(os.version, "5.15.0");
        assert_eq!(os.arch, "x86_64");
    }

    #[test]
    fn test_os_info_with_arm() {
        let os = OsInfo {
            name: "Darwin".to_string(),
            version: "22.0.0".to_string(),
            kernel: "Darwin 22.0.0".to_string(),
            arch: "arm64".to_string(),
        };

        assert_eq!(os.name, "Darwin");
        assert_eq!(os.arch, "arm64");
    }

    #[test]
    fn test_os_info_serialization() {
        let os = OsInfo {
            name: "Linux".to_string(),
            version: "6.1.0".to_string(),
            kernel: "6.1.0-arch".to_string(),
            arch: "x86_64".to_string(),
        };

        let json = serde_json::to_string(&os).expect("Failed to serialize");
        assert!(json.contains("Linux"));
        assert!(json.contains("x86_64"));
    }

    #[test]
    fn test_os_info_deserialization() {
        let json = r#"{
            "name": "Windows",
            "version": "11",
            "kernel": "NT 10.0.22000",
            "arch": "x86_64"
        }"#;

        let os: OsInfo = serde_json::from_str(json).expect("Failed to deserialize");

        assert_eq!(os.name, "Windows");
        assert_eq!(os.version, "11");
        assert_eq!(os.arch, "x86_64");
    }
}

#[cfg(test)]
mod system_info_tests {
    use super::*;

    #[test]
    fn test_system_info_default() {
        let system = SystemInfo::default();

        // Just verify it creates without panic
        let _json = serde_json::to_string(&system).expect("Failed to serialize default");
    }

    #[test]
    fn test_system_info_clone() {
        let system1 = SystemInfo::default();
        let system2 = system1.clone();

        // Verify clone works
        let json1 = serde_json::to_string(&system1).expect("Failed to serialize");
        let json2 = serde_json::to_string(&system2).expect("Failed to serialize");
        assert_eq!(json1, json2);
    }
}
