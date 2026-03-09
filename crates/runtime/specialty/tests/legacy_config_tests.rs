// SPDX-License-Identifier: AGPL-3.0-only
//! Comprehensive Tests for Legacy Runtime Configuration
//!
//! Tests cover:
//! - Configuration creation and defaults
//! - Serialization/deserialization
//! - Base config pattern integration
//! - Resource limits
//! - Platform-specific settings

use toadstool_runtime_legacy::types::configs::*;
use toadstool_common::config_bases::{TimeoutConfig, RetryConfig};
use std::time::Duration;

#[test]
fn test_legacy_runtime_config_defaults() {
    let config = LegacyRuntimeConfig::default();
    
    assert!(!config.supported_platforms.is_empty(), "Should have supported platforms");
    assert!(config.emulation.is_some(), "Should have emulation settings");
}

#[test]
fn test_legacy_runtime_config_serialization() {
    let config = LegacyRuntimeConfig::default();
    
    let json = serde_json::to_string(&config);
    assert!(json.is_ok(), "Config should serialize to JSON: {:?}", json.err());
    
    let json_str = json.unwrap();
    let deserialized: Result<LegacyRuntimeConfig, _> = serde_json::from_str(&json_str);
    assert!(
        deserialized.is_ok(),
        "Config should deserialize from JSON: {:?}",
        deserialized.err()
    );
}

#[test]
fn test_emulation_settings_defaults() {
    let settings = EmulationSettings::default();
    
    assert!(settings.enable_cycle_accurate, "Cycle accuracy should be enabled by default");
    assert_eq!(settings.emulation_speed, EmulationSpeed::Normal);
}

#[test]
fn test_emulation_speed_variants() {
    assert_ne!(EmulationSpeed::Normal, EmulationSpeed::Fast);
    assert_ne!(EmulationSpeed::Normal, EmulationSpeed::CycleAccurate);
    assert_ne!(EmulationSpeed::Fast, EmulationSpeed::Turbo);
}

#[test]
fn test_communication_settings_with_base_configs() {
    let settings = CommunicationSettings::default();
    
    // Verify base configs are properly integrated via flatten
    assert!(settings.timeouts.connection_timeout.as_secs() > 0, "Should have connection timeout");
    assert!(settings.retries.max_attempts > 0, "Should have retry attempts");
}

#[test]
fn test_communication_settings_custom() {
    let mut settings = CommunicationSettings::default();
    settings.timeouts.connection_timeout = Duration::from_secs(60);
    settings.retries.max_attempts = 5;
    
    assert_eq!(settings.timeouts.connection_timeout, Duration::from_secs(60));
    assert_eq!(settings.retries.max_attempts, 5);
}

#[test]
fn test_communication_settings_serialization() {
    let settings = CommunicationSettings::default();
    
    let json = serde_json::to_string(&settings);
    assert!(json.is_ok(), "Should serialize: {:?}", json.err());
    
    let deserialized: Result<CommunicationSettings, _> = 
        serde_json::from_str(&json.unwrap());
    assert!(deserialized.is_ok(), "Should deserialize: {:?}", deserialized.err());
}

#[test]
fn test_mainframe_connection_settings() {
    let settings = MainframeConnectionSettings {
        host: "localhost".to_string(),
        port: 23,
        codepage: "IBM037".to_string(),
        use_tls: false,
        timeout_seconds: 30,
    };
    
    assert_eq!(settings.host, "localhost");
    assert_eq!(settings.port, 23);
    assert!(!settings.use_tls);
}

#[test]
fn test_embedded_platform_config() {
    let config = EmbeddedPlatformConfig {
        architecture: "ARM Cortex-M".to_string(),
        clock_speed_hz: 100_000_000, // 100 MHz
        memory_size_bytes: 512 * 1024, // 512 KB
        enable_debugging: true,
        optimization_level: OptimizationLevel::Size,
    };
    
    assert_eq!(config.architecture, "ARM Cortex-M");
    assert_eq!(config.clock_speed_hz, 100_000_000);
    assert!(config.enable_debugging);
}

#[test]
fn test_optimization_level_variants() {
    assert_ne!(OptimizationLevel::None, OptimizationLevel::Size);
    assert_ne!(OptimizationLevel::Size, OptimizationLevel::Speed);
    assert_ne!(OptimizationLevel::Speed, OptimizationLevel::Balanced);
}

#[test]
fn test_realtime_config() {
    let config = RealtimeConfig {
        max_latency_us: 100, // 100 microseconds
        scheduling_policy: SchedulingPolicy::FixedPriority,
        enable_watchdog: true,
        watchdog_timeout_ms: 1000,
    };
    
    assert_eq!(config.max_latency_us, 100);
    assert!(config.enable_watchdog);
}

#[test]
fn test_scheduling_policy_variants() {
    assert_ne!(SchedulingPolicy::FixedPriority, SchedulingPolicy::RoundRobin);
    assert_ne!(SchedulingPolicy::RoundRobin, SchedulingPolicy::EarliestDeadlineFirst);
}

#[test]
fn test_industrial_protocol_config() {
    let config = IndustrialProtocolConfig {
        protocol_type: "EtherCAT".to_string(),
        cycle_time_us: 1000, // 1ms cycle
        enable_redundancy: true,
        topology: NetworkTopology::Ring,
    };
    
    assert_eq!(config.protocol_type, "EtherCAT");
    assert_eq!(config.cycle_time_us, 1000);
    assert!(config.enable_redundancy);
}

#[test]
fn test_network_topology_variants() {
    assert_ne!(NetworkTopology::Ring, NetworkTopology::Star);
    assert_ne!(NetworkTopology::Star, NetworkTopology::Bus);
    assert_ne!(NetworkTopology::Bus, NetworkTopology::Mesh);
}

#[test]
fn test_cross_compilation_config() {
    let config = CrossCompilationConfig {
        target_triple: "arm-unknown-linux-gnueabihf".to_string(),
        sysroot: Some("/opt/arm-toolchain".to_string()),
        additional_flags: vec!["-march=armv7".to_string()],
        enable_lto: false,
    };
    
    assert_eq!(config.target_triple, "arm-unknown-linux-gnueabihf");
    assert!(config.sysroot.is_some());
    assert_eq!(config.additional_flags.len(), 1);
}

#[test]
fn test_memory_model_config() {
    let config = MemoryModelConfig {
        address_space_bits: 32,
        endianness: Endianness::Little,
        page_size_bytes: 4096,
        enable_mmu: true,
    };
    
    assert_eq!(config.address_space_bits, 32);
    assert_eq!(config.endianness, Endianness::Little);
    assert!(config.enable_mmu);
}

#[test]
fn test_endianness_variants() {
    assert_ne!(Endianness::Little, Endianness::Big);
}

#[test]
fn test_authentication_settings() {
    let settings = AuthenticationSettings {
        username: Some("admin".to_string()),
        password: Some("secret".to_string()),
        certificate_path: None,
        use_kerberos: false,
    };
    
    assert_eq!(settings.username, Some("admin".to_string()));
    assert!(!settings.use_kerberos);
}

#[test]
fn test_connection_type_variants() {
    assert_ne!(ConnectionType::LocalEmulation, ConnectionType::RemoteSSH);
    assert_ne!(ConnectionType::RemoteSSH, ConnectionType::SerialPort);
    assert_ne!(ConnectionType::SerialPort, ConnectionType::NetworkSocket);
}

#[test]
fn test_platform_type_variants() {
    assert_ne!(PlatformType::Mainframe, PlatformType::Embedded);
    assert_ne!(PlatformType::Embedded, PlatformType::Realtime);
    assert_ne!(PlatformType::Realtime, PlatformType::Industrial);
}

#[test]
fn test_config_clone() {
    let config = LegacyRuntimeConfig::default();
    let cloned = config.clone();
    
    assert_eq!(
        config.supported_platforms,
        cloned.supported_platforms,
        "Cloned config should be equal"
    );
}

#[test]
fn test_emulation_settings_clone() {
    let settings = EmulationSettings::default();
    let cloned = settings.clone();
    
    assert_eq!(
        settings.enable_cycle_accurate,
        cloned.enable_cycle_accurate,
        "Cloned settings should be equal"
    );
}

#[test]
fn test_communication_settings_timeout_config_flatten() {
    // Test that TimeoutConfig is properly flattened
    let json = r#"{
        "connection_type": "LocalEmulation",
        "connection_timeout": 30,
        "request_timeout": 60,
        "read_timeout": 5,
        "write_timeout": 5,
        "max_attempts": 3,
        "initial_backoff_ms": 100,
        "max_backoff_ms": 5000,
        "backoff_multiplier": 2.0,
        "jitter": true
    }"#;
    
    let result: Result<CommunicationSettings, _> = serde_json::from_str(json);
    assert!(
        result.is_ok(),
        "Should deserialize with flattened base configs: {:?}",
        result.err()
    );
    
    let settings = result.unwrap();
    assert_eq!(settings.timeouts.connection_timeout, Duration::from_secs(30));
    assert_eq!(settings.retries.max_attempts, 3);
}

#[test]
fn test_legacy_config_with_custom_platforms() {
    let mut config = LegacyRuntimeConfig::default();
    config.supported_platforms = vec![
        PlatformType::Mainframe,
        PlatformType::Embedded,
    ];
    
    assert_eq!(config.supported_platforms.len(), 2);
    assert!(config.supported_platforms.contains(&PlatformType::Mainframe));
}

#[test]
fn test_mainframe_settings_with_tls() {
    let settings = MainframeConnectionSettings {
        host: "mainframe.example.com".to_string(),
        port: 992, // TLS port
        codepage: "IBM037".to_string(),
        use_tls: true,
        timeout_seconds: 60,
    };
    
    assert!(settings.use_tls);
    assert_eq!(settings.port, 992);
}

#[test]
fn test_embedded_config_low_power() {
    let config = EmbeddedPlatformConfig {
        architecture: "ARM Cortex-M0".to_string(),
        clock_speed_hz: 16_000_000, // 16 MHz - low power
        memory_size_bytes: 64 * 1024, // 64 KB
        enable_debugging: false,
        optimization_level: OptimizationLevel::Size,
    };
    
    assert_eq!(config.optimization_level, OptimizationLevel::Size);
    assert_eq!(config.clock_speed_hz, 16_000_000);
}

#[test]
fn test_realtime_config_hard_realtime() {
    let config = RealtimeConfig {
        max_latency_us: 10, // 10 microseconds - hard realtime
        scheduling_policy: SchedulingPolicy::EarliestDeadlineFirst,
        enable_watchdog: true,
        watchdog_timeout_ms: 100,
    };
    
    assert_eq!(config.max_latency_us, 10);
    assert_eq!(config.scheduling_policy, SchedulingPolicy::EarliestDeadlineFirst);
}

#[test]
fn test_industrial_protocol_redundancy() {
    let config = IndustrialProtocolConfig {
        protocol_type: "PROFINET".to_string(),
        cycle_time_us: 500, // 500 microseconds
        enable_redundancy: true,
        topology: NetworkTopology::Mesh,
    };
    
    assert!(config.enable_redundancy);
    assert_eq!(config.topology, NetworkTopology::Mesh);
}

#[test]
fn test_cross_compilation_with_lto() {
    let config = CrossCompilationConfig {
        target_triple: "x86_64-unknown-linux-musl".to_string(),
        sysroot: None,
        additional_flags: vec!["-static".to_string()],
        enable_lto: true,
    };
    
    assert!(config.enable_lto);
    assert!(config.sysroot.is_none());
}

#[test]
fn test_memory_model_64bit() {
    let config = MemoryModelConfig {
        address_space_bits: 64,
        endianness: Endianness::Little,
        page_size_bytes: 4096,
        enable_mmu: true,
    };
    
    assert_eq!(config.address_space_bits, 64);
}

#[test]
fn test_memory_model_big_endian() {
    let config = MemoryModelConfig {
        address_space_bits: 32,
        endianness: Endianness::Big,
        page_size_bytes: 4096,
        enable_mmu: false,
    };
    
    assert_eq!(config.endianness, Endianness::Big);
    assert!(!config.enable_mmu);
}

#[test]
fn test_authentication_with_certificate() {
    let settings = AuthenticationSettings {
        username: None,
        password: None,
        certificate_path: Some("/path/to/cert.pem".to_string()),
        use_kerberos: false,
    };
    
    assert!(settings.certificate_path.is_some());
    assert!(settings.username.is_none());
}

#[test]
fn test_authentication_with_kerberos() {
    let settings = AuthenticationSettings {
        username: Some("user@REALM".to_string()),
        password: None,
        certificate_path: None,
        use_kerberos: true,
    };
    
    assert!(settings.use_kerberos);
    assert!(settings.username.is_some());
}

