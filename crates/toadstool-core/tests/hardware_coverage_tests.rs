// SPDX-License-Identifier: AGPL-3.0-or-later
//! Coverage tests for toadstool-core hardware module
//! Exercises `HardwareManager`, `HardwareDevice`, `HardwareType`, `enable_npu_userspace`.

use toadstool_core::hardware::{HardwareDevice, HardwareError, HardwareManager, HardwareType};

#[test]
fn test_hardware_manager_discover() {
    let manager = HardwareManager::discover().expect("Discovery should succeed");
    assert!(!manager.devices().is_empty());
}

#[test]
fn test_hardware_manager_cpu_always_present() {
    let manager = HardwareManager::discover().unwrap();
    let cpus = manager.devices_by_type(HardwareType::Cpu);
    assert!(!cpus.is_empty());
    assert!(cpus.iter().any(|d| d.name == "CPU"));
}

#[test]
fn test_hardware_manager_devices_by_type_gpu() {
    let manager = HardwareManager::discover().unwrap();
    let gpus = manager.devices_by_type(HardwareType::Gpu);
    // May be empty on systems without GPU
    for gpu in &gpus {
        assert_eq!(gpu.hardware_type, HardwareType::Gpu);
    }
}

#[test]
fn test_hardware_manager_devices_by_type_npu() {
    let manager = HardwareManager::discover().unwrap();
    let npus = manager.devices_by_type(HardwareType::Npu);
    for npu in &npus {
        assert_eq!(npu.hardware_type, HardwareType::Npu);
    }
}

#[test]
fn test_hardware_manager_has_gpu() {
    let manager = HardwareManager::discover().unwrap();
    let has_gpu = manager.has_gpu();
    // Result depends on system
    let _ = has_gpu;
}

#[test]
fn test_hardware_manager_has_npu() {
    let manager = HardwareManager::discover().unwrap();
    let has_npu = manager.has_npu();
    let _ = has_npu;
}

#[test]
fn test_hardware_manager_device_count() {
    let manager = HardwareManager::discover().unwrap();
    let count = manager.device_count();
    assert!(count >= 1);
    assert_eq!(count, manager.devices().len());
}

#[test]
fn test_hardware_manager_rescan() {
    let mut manager = HardwareManager::discover().unwrap();
    let result = manager.rescan();
    assert!(result.is_ok());
}

#[test]
fn test_hardware_type_variants() {
    let _ = HardwareType::Gpu;
    let _ = HardwareType::Npu;
    let _ = HardwareType::Cpu;
    let _ = HardwareType::Fpga;
    let _ = HardwareType::Custom;
}

#[test]
fn test_hardware_device_structure() {
    let device = HardwareDevice {
        hardware_type: HardwareType::Gpu,
        name: "Test GPU".to_string(),
        pcie_address: Some("0000:01:00.0".to_string()),
        vendor_id: Some("10de".to_string()),
        device_id: Some("1234".to_string()),
        driver_available: true,
        userspace_capable: true,
    };
    assert_eq!(device.name, "Test GPU");
    assert_eq!(device.hardware_type, HardwareType::Gpu);
    assert!(device.pcie_address.is_some());
}

#[test]
fn test_enable_npu_userspace_nonexistent_device() {
    let manager = HardwareManager::discover().unwrap();
    let result = manager.enable_npu_userspace("0000:ff:00.0-nonexistent-pcie-addr");
    assert!(result.is_err());
    assert!(matches!(result, Err(HardwareError::NpuNotFound { .. })));
}

#[test]
fn test_hardware_error_display() {
    let err = HardwareError::NpuNotFound {
        address: "0000:01:00.0".to_string(),
    };
    let s = err.to_string();
    assert!(s.contains("NPU device not found"));
    assert!(s.contains("0000:01:00.0"));
}

#[test]
fn test_hardware_device_all_fields() {
    let device = HardwareDevice {
        hardware_type: HardwareType::Npu,
        name: "Akida AKD1500".to_string(),
        pcie_address: Some("0000:02:00.0".to_string()),
        vendor_id: Some("1e7c".to_string()),
        device_id: Some("bca2".to_string()),
        driver_available: false,
        userspace_capable: true,
    };
    assert_eq!(device.name, "Akida AKD1500");
    assert_eq!(device.device_id.as_deref(), Some("bca2"));
    assert!(!device.driver_available);
    assert!(device.userspace_capable);
}

#[test]
fn test_hardware_type_fpga_and_custom() {
    let _ = HardwareType::Fpga;
    let _ = HardwareType::Custom;
    assert_ne!(HardwareType::Gpu, HardwareType::Npu);
    assert_eq!(HardwareType::Cpu, HardwareType::Cpu);
}

#[test]
fn test_hardware_manager_devices_iteration() {
    let manager = HardwareManager::discover().unwrap();
    let devices: Vec<_> = manager.devices().iter().collect();
    assert!(!devices.is_empty());
    for d in &devices {
        assert!(!d.name.is_empty());
    }
}

#[test]
fn test_hardware_manager_devices_by_type_fpga_custom() {
    let manager = HardwareManager::discover().unwrap();
    let fpga = manager.devices_by_type(HardwareType::Fpga);
    let custom = manager.devices_by_type(HardwareType::Custom);
    assert!(fpga.is_empty());
    assert!(custom.is_empty());
}
