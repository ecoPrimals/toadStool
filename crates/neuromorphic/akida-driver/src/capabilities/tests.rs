// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;

#[test]
fn test_chip_version_from_device_id() {
    assert_eq!(ChipVersion::from_device_id(0xBCA1), ChipVersion::Akd1000);
    assert_eq!(ChipVersion::from_device_id(0xBCA2), ChipVersion::Akd1500);
    assert!(matches!(
        ChipVersion::from_device_id(0xFFFF),
        ChipVersion::Unknown(0xFFFF)
    ));
}

#[test]
fn test_pcie_bandwidth_calculation() {
    let gen2_x4 = PcieConfig::new(2, 4);
    assert!((gen2_x4.bandwidth_gbps - 2.0).abs() < f32::EPSILON);

    let gen3_x8 = PcieConfig::new(3, 8);
    assert!((gen3_x8.bandwidth_gbps - 8.0).abs() < f32::EPSILON);

    let gen4_x16 = PcieConfig::new(4, 16);
    assert!((gen4_x16.bandwidth_gbps - 32.0).abs() < f32::EPSILON);
}

#[test]
fn test_mesh_topology_total_slots() {
    let mesh = MeshTopology {
        x: 5,
        y: 8,
        z: 2,
        functional_count: 80,
    };
    assert_eq!(mesh.total_slots(), 80);
    assert_eq!(mesh.functional_count, 80);
}

#[test]
fn test_mesh_topology_with_disabled_nps() {
    let mesh = MeshTopology {
        x: 5,
        y: 8,
        z: 2,
        functional_count: 78,
    };
    assert_eq!(mesh.total_slots(), 80);
    assert_eq!(mesh.functional_count, 78);
}

#[test]
fn test_clock_mode_parsing() {
    assert_eq!(
        ClockMode::from_sysfs_str("performance"),
        ClockMode::Performance
    );
    assert_eq!(ClockMode::from_sysfs_str("economy"), ClockMode::Economy);
    assert_eq!(ClockMode::from_sysfs_str("eco"), ClockMode::Economy);
    assert_eq!(ClockMode::from_sysfs_str("low_power"), ClockMode::LowPower);
    assert_eq!(
        ClockMode::from_sysfs_str("  PERF  "),
        ClockMode::Performance
    );
    assert_eq!(ClockMode::from_sysfs_str("unknown"), ClockMode::Performance);
}

#[test]
fn test_clock_mode_penalties() {
    assert!((ClockMode::Performance.expected_speed_penalty() - 0.0).abs() < f64::EPSILON);
    assert!(ClockMode::Economy.expected_speed_penalty() > 0.0);
    assert!(ClockMode::Economy.expected_power_savings() > 0.0);
    assert!(
        ClockMode::LowPower.expected_speed_penalty() > ClockMode::Economy.expected_speed_penalty()
    );
}

#[test]
fn test_weight_mutation_support_variants() {
    assert_eq!(WeightMutationSupport::None, WeightMutationSupport::None);
    assert_ne!(
        WeightMutationSupport::Full,
        WeightMutationSupport::ReadoutOnly
    );
}

#[test]
fn test_chip_version_from_register() {
    assert_eq!(ChipVersion::from_register(0x10), ChipVersion::Akd1000);
    assert_eq!(ChipVersion::from_register(0x15), ChipVersion::Akd1500);
    assert!(matches!(
        ChipVersion::from_register(0x99),
        ChipVersion::Unknown(_)
    ));
}

#[test]
fn test_chip_version_typical_counts() {
    assert_eq!(ChipVersion::Akd1000.typical_npu_count(), 80);
    assert_eq!(ChipVersion::Akd1500.typical_npu_count(), 80);
    assert_eq!(ChipVersion::Unknown(0).typical_npu_count(), 0);
}

#[test]
fn test_chip_version_typical_memory() {
    assert_eq!(ChipVersion::Akd1000.typical_memory_mb(), 10);
    assert_eq!(ChipVersion::Akd1500.typical_memory_mb(), 10);
    assert_eq!(ChipVersion::Unknown(0).typical_memory_mb(), 0);
}

#[test]
fn test_pcie_config_generation_speeds() {
    let gen1 = PcieConfig::new(1, 4);
    assert!((gen1.speed_gts - 2.5).abs() < f32::EPSILON);
    let gen3 = PcieConfig::new(3, 8);
    assert!((gen3.speed_gts - 8.0).abs() < f32::EPSILON);
    let gen5 = PcieConfig::new(5, 16);
    assert!((gen5.speed_gts - 32.0).abs() < f32::EPSILON);
}

#[test]
fn test_pcie_config_unknown_generation() {
    let cfg = PcieConfig::new(99, 4);
    assert!((cfg.bandwidth_gbps - 16.0).abs() < f32::EPSILON); // default per_lane
}

#[test]
fn test_clock_mode_lp_variants() {
    assert_eq!(ClockMode::from_sysfs_str("lp"), ClockMode::LowPower);
    assert_eq!(ClockMode::from_sysfs_str("lowpower"), ClockMode::LowPower);
}

#[test]
fn test_batch_capabilities_defaults() {
    // BatchCapabilities::from_sysfs returns None when sysfs not available
    let result = BatchCapabilities::from_sysfs("/nonexistent/pci/device");
    assert!(result.is_none());
}

#[test]
fn test_pcie_config_new() {
    let cfg = PcieConfig::new(3, 8);
    assert_eq!(cfg.generation, 3);
    assert_eq!(cfg.lanes, 8);
    assert!((cfg.speed_gts - 8.0).abs() < f32::EPSILON);
    assert!((cfg.bandwidth_gbps - 8.0).abs() < f32::EPSILON);
}

#[test]
fn test_mesh_topology_from_sysfs_nonexistent() {
    let result = MeshTopology::from_sysfs("/nonexistent/pci/device");
    assert!(result.is_none());
}

#[test]
fn test_clock_mode_expected_values() {
    assert!((ClockMode::Performance.expected_speed_penalty() - 0.0).abs() < f64::EPSILON);
    assert!((ClockMode::Economy.expected_speed_penalty() - 0.19).abs() < f64::EPSILON);
    assert!((ClockMode::Economy.expected_power_savings() - 0.18).abs() < f64::EPSILON);
    assert!((ClockMode::LowPower.expected_power_savings() - 0.35).abs() < f64::EPSILON);
}

#[test]
fn test_capabilities_struct_equality() {
    let cfg1 = PcieConfig::new(3, 8);
    let cfg2 = PcieConfig::new(3, 8);
    assert_eq!(cfg1, cfg2);
}

#[test]
fn test_chip_version_unknown_typical() {
    let v = ChipVersion::Unknown(0x1234);
    assert_eq!(v.typical_npu_count(), 0);
    assert_eq!(v.typical_memory_mb(), 0);
}
