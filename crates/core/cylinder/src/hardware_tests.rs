// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests extracted from hardware.rs (S335).

use super::hardware::*;

#[test]
fn vendor_display() {
    assert_eq!(Vendor::Nvidia.to_string(), "NVIDIA");
    assert_eq!(Vendor::Amd.to_string(), "AMD");
    assert_eq!(Vendor::Intel.to_string(), "Intel");
    assert_eq!(Vendor::Unknown.to_string(), "Unknown");
}

#[test]
fn memory_type_display() {
    assert_eq!(MemoryType::Gddr5.to_string(), "GDDR5");
    assert_eq!(MemoryType::Hbm2.to_string(), "HBM2");
    assert_eq!(MemoryType::Hbm3.to_string(), "HBM3");
    assert_eq!(MemoryType::Gddr7.to_string(), "GDDR7");
}

#[test]
fn unknown_capabilities_are_conservative() {
    let caps = HardwareCapabilities::UNKNOWN;
    assert_eq!(caps.vendor, Vendor::Unknown);
    assert!(!caps.has_hardware_f64);
    assert!(!caps.has_hardware_f64_rcp);
    assert!(!caps.has_full_rate_fp64);
}

#[test]
fn capabilities_display() {
    let caps = HardwareCapabilities {
        vendor: Vendor::Nvidia,
        device_name: "Blackwell B",
        generation_name: "Blackwell",
        has_hardware_f64: true,
        has_hardware_f64_rcp: false,
        has_full_rate_fp64: false,
        native_wave_size: WaveSize::Wave32,
        memory_type: MemoryType::Gddr7,
        completion_style: CompletionStyle::DeviceFence,
        max_shared_mem_bytes: 49152,
    };
    let s = caps.to_string();
    assert!(s.contains("NVIDIA"));
    assert!(s.contains("Blackwell B"));
    assert!(s.contains("GDDR7"));
}

#[test]
fn vendor_equality_and_hash() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(Vendor::Nvidia);
    set.insert(Vendor::Amd);
    assert!(set.contains(&Vendor::Nvidia));
    assert!(!set.contains(&Vendor::Intel));
}

#[test]
fn device_topology_single() {
    let topo = DeviceTopology::single("Titan V", "0000:02:00.0", Vendor::Nvidia);
    assert_eq!(topo.function_count(), 1);
    assert_eq!(topo.functions[0].bdf, "0000:02:00.0");
    assert_eq!(topo.functions[0].function_index, 0);
    assert_eq!(topo.functions[0].vendor, Vendor::Nvidia);
    assert!(topo.shared_firmware.is_none());
}

#[test]
fn device_topology_dual() {
    let topo = DeviceTopology::dual("Tesla K80", "0000:4b:00.0", "0000:4c:00.0", Vendor::Nvidia);
    assert_eq!(topo.function_count(), 2);
    assert_eq!(topo.functions[0].bdf, "0000:4b:00.0");
    assert_eq!(topo.functions[1].bdf, "0000:4c:00.0");
    assert_eq!(topo.functions[1].function_index, 1);
}

#[test]
fn device_topology_with_firmware() {
    let topo = DeviceTopology::single("MI50", "0000:03:00.0", Vendor::Amd)
        .with_firmware(vec![0x55, 0xAA, 0x80]);
    assert!(topo.shared_firmware.is_some());
    assert_eq!(topo.shared_firmware.as_ref().unwrap().len(), 3);
}

#[test]
fn boot_probe_info_debug() {
    let info = BootProbeInfo {
        vendor: Vendor::Nvidia,
        family: "Volta".to_string(),
        warm: true,
        identity_raw: 0x1400_00a1,
    };
    assert!(format!("{info:?}").contains("Volta"));
    assert!(info.warm);
}

#[test]
fn boot_init_info_debug() {
    let info = BootInitInfo {
        memory_alive: true,
        writes_applied: 276,
        method: "vbios-interpreter".to_string(),
    };
    assert!(info.memory_alive);
    assert_eq!(info.writes_applied, 276);
}

#[test]
fn function_boot_result_defaults() {
    let result = FunctionBootResult {
        bdf: "0000:02:00.0".to_string(),
        function_index: 0,
        probe: None,
        init: None,
        compute_ready: false,
        error: Some("not implemented".to_string()),
    };
    assert!(!result.compute_ready);
    assert!(result.error.is_some());
}

#[test]
fn device_boot_result_all_ready() {
    let result = DeviceBootResult {
        functions: vec![
            FunctionBootResult {
                bdf: "0000:4b:00.0".to_string(),
                function_index: 0,
                probe: None,
                init: None,
                compute_ready: true,
                error: None,
            },
            FunctionBootResult {
                bdf: "0000:4c:00.0".to_string(),
                function_index: 1,
                probe: None,
                init: None,
                compute_ready: true,
                error: None,
            },
        ],
        all_ready: true,
    };
    assert!(result.all_ready);
    assert_eq!(result.functions.len(), 2);
}
