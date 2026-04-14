// SPDX-License-Identifier: AGPL-3.0-or-later


use super::*;

#[test]
fn test_load_config_from_capabilities() {
    let caps = Capabilities {
        chip_version: crate::ChipVersion::Akd1000,
        npu_count: 80,
        memory_mb: 10,
        pcie: crate::PcieConfig {
            generation: 2,
            lanes: 1,
            speed_gts: 5.0,
            bandwidth_gbps: 0.5,
        },
        power_mw: None,
        temperature_c: None,
        mesh: None,
        clock_mode: None,
        batch: None,
        weight_mutation: crate::capabilities::WeightMutationSupport::None,
    };

    let config = LoadConfig::from_capabilities(&caps, 0);
    assert_eq!(config.chunk_size, 4096); // 10MB device -> 4KB chunks
}

#[test]
fn test_model_program_creation() {
    let data = vec![0x42; 1000];
    let program = ModelProgram::new(data);

    assert_eq!(program.memory_bytes, 1000);
    assert_eq!(program.npus_required, 1); // Small program -> 1 NPU
    assert_ne!(program.checksum, 0);
}

#[test]
fn test_program_chunking() {
    let data = vec![0x42; 1000];
    let program = ModelProgram::new(data);

    let chunks = program.chunk(100);
    assert_eq!(chunks.len(), 10);
    assert_eq!(chunks[0].len(), 100);
}

#[test]
fn test_throughput_calculation() {
    let throughput = calculate_throughput(1_048_576, 1.0);
    assert!((throughput - 1.0).abs() < 0.01); // ~1 MB/s
}

#[test]
fn test_throughput_zero_seconds() {
    let throughput = calculate_throughput(1000, 0.0);
    assert!((throughput - 0.0).abs() < f64::EPSILON);
}

#[test]
fn test_load_config_minimal() {
    let config = LoadConfig::minimal(1);
    assert_eq!(config.device_index, 1);
    assert_eq!(config.chunk_size, 1024);
    assert_eq!(config.timeout_ms, 1000);
    assert!(!config.validate);
}

#[test]
fn test_load_config_from_capabilities_medium_device() {
    let caps = Capabilities {
        chip_version: crate::ChipVersion::Akd1000,
        npu_count: 80,
        memory_mb: 25,
        pcie: crate::PcieConfig::new(3, 8),
        power_mw: None,
        temperature_c: None,
        mesh: None,
        clock_mode: None,
        batch: None,
        weight_mutation: crate::capabilities::WeightMutationSupport::None,
    };
    let config = LoadConfig::from_capabilities(&caps, 0);
    assert_eq!(config.chunk_size, 16384);
}

#[test]
fn test_load_config_from_capabilities_large_device() {
    let caps = Capabilities {
        chip_version: crate::ChipVersion::Akd1500,
        npu_count: 80,
        memory_mb: 64,
        pcie: crate::PcieConfig::new(4, 16),
        power_mw: None,
        temperature_c: None,
        mesh: None,
        clock_mode: None,
        batch: None,
        weight_mutation: crate::capabilities::WeightMutationSupport::None,
    };
    let config = LoadConfig::from_capabilities(&caps, 0);
    assert_eq!(config.chunk_size, 65536);
}

#[test]
fn test_model_program_with_npu_config() {
    let data = vec![0u8; 5000];
    let config = NpuConfig {
        required_npus: 20,
        execution_groups: 2,
        memory_per_npu: 256,
    };
    let program = ModelProgram::new(data).with_npu_config(config);
    assert_eq!(program.npus_required, 20);
    assert_eq!(program.npu_config.as_ref().unwrap().required_npus, 20);
}

#[test]
fn test_model_program_validate_for_device_ok() {
    let data = vec![0u8; 1000];
    let program = ModelProgram::new(data);
    let caps = Capabilities {
        chip_version: crate::ChipVersion::Akd1000,
        npu_count: 80,
        memory_mb: 10,
        pcie: crate::PcieConfig::new(3, 8),
        power_mw: None,
        temperature_c: None,
        mesh: None,
        clock_mode: None,
        batch: None,
        weight_mutation: crate::capabilities::WeightMutationSupport::None,
    };
    assert!(program.validate_for_device(&caps).is_ok());
}

#[test]
fn test_model_program_validate_for_device_memory_overflow() {
    let data = vec![0u8; 20 * 1024 * 1024];
    let program = ModelProgram::new(data);
    let caps = Capabilities {
        chip_version: crate::ChipVersion::Akd1000,
        npu_count: 80,
        memory_mb: 10,
        pcie: crate::PcieConfig::new(3, 8),
        power_mw: None,
        temperature_c: None,
        mesh: None,
        clock_mode: None,
        batch: None,
        weight_mutation: crate::capabilities::WeightMutationSupport::None,
    };
    let result = program.validate_for_device(&caps);
    assert!(result.is_err());
}

#[test]
fn test_estimate_npu_requirement_sizes() {
    let small = vec![0u8; 5_000];
    let p = ModelProgram::new(small);
    assert_eq!(p.npus_required, 1);

    let medium = vec![0u8; 50_000];
    let p = ModelProgram::new(medium);
    assert_eq!(p.npus_required, 10);

    let large = vec![0u8; 200_000];
    let p = ModelProgram::new(large);
    assert_eq!(p.npus_required, 20);

    let huge = vec![0u8; 2_000_000];
    let p = ModelProgram::new(huge);
    assert_eq!(p.npus_required, 80);
}

#[test]
fn test_model_loader_new() {
    let config = LoadConfig::minimal(0);
    let _loader = ModelLoader::new(config);
}

#[test]
fn test_load_metrics_struct() {
    let metrics = LoadMetrics {
        bytes_transferred: 1024,
        chunks_transferred: 4,
        duration: std::time::Duration::from_secs(1),
        throughput_mbps: 1.0,
    };
    assert_eq!(metrics.bytes_transferred, 1024);
    assert_eq!(metrics.chunks_transferred, 4);
}
