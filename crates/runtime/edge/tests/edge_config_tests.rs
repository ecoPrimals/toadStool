// SPDX-License-Identifier: AGPL-3.0-only
//! Comprehensive tests for Edge runtime configuration and connection types

use toadstool_runtime_edge::*;
use std::collections::HashMap;

// ============================================================================
// EdgeRuntimeConfig Tests
// ============================================================================

#[test]
fn test_edge_runtime_config_default() {
    let config = EdgeRuntimeConfig::default();
    
    assert!(config.discovery_enabled);
    assert_eq!(config.discovery_timeout_secs, 30);
    assert_eq!(config.max_devices, 100);
    assert_eq!(config.communication_timeout_ms, 5000);
    assert_eq!(config.cross_compile_cache_path, "/tmp/toadstool_edge_cache");
    assert!(config.auto_provisioning);
}

#[test]
fn test_edge_runtime_config_discovery_settings() {
    let config = EdgeRuntimeConfig::default();
    
    assert!(config.discovery_enabled);
    assert_eq!(config.discovery_timeout_secs, 30);
}

#[test]
fn test_edge_runtime_config_device_limits() {
    let config = EdgeRuntimeConfig::default();
    
    assert_eq!(config.max_devices, 100);
}

#[test]
fn test_edge_runtime_config_communication_timeout() {
    let config = EdgeRuntimeConfig::default();
    
    assert_eq!(config.communication_timeout_ms, 5000);
}

#[test]
fn test_edge_runtime_config_auto_provisioning() {
    let config = EdgeRuntimeConfig::default();
    
    assert!(config.auto_provisioning);
}

#[test]
fn test_edge_runtime_config_security_level_default() {
    let config = EdgeRuntimeConfig::default();
    
    assert!(matches!(config.security_level, EdgeSecurityLevel::Standard));
}

#[test]
fn test_edge_runtime_config_resource_strategy_default() {
    let config = EdgeRuntimeConfig::default();
    
    assert!(matches!(config.resource_strategy, ResourceAllocationStrategy::Adaptive));
}

#[test]
fn test_edge_runtime_config_clone() {
    let config1 = EdgeRuntimeConfig::default();
    let config2 = config1.clone();
    
    assert_eq!(config1.max_devices, config2.max_devices);
    assert_eq!(config1.discovery_timeout_secs, config2.discovery_timeout_secs);
}

// ============================================================================
// EdgeSecurityLevel Tests (4 variants)
// ============================================================================

#[test]
fn test_edge_security_level_minimal() {
    let level = EdgeSecurityLevel::Minimal;
    assert!(matches!(level, EdgeSecurityLevel::Minimal));
}

#[test]
fn test_edge_security_level_standard() {
    let level = EdgeSecurityLevel::Standard;
    assert!(matches!(level, EdgeSecurityLevel::Standard));
}

#[test]
fn test_edge_security_level_high() {
    let level = EdgeSecurityLevel::High;
    assert!(matches!(level, EdgeSecurityLevel::High));
}

#[test]
fn test_edge_security_level_maximum() {
    let level = EdgeSecurityLevel::Maximum;
    assert!(matches!(level, EdgeSecurityLevel::Maximum));
}

#[test]
fn test_edge_security_level_clone() {
    let level1 = EdgeSecurityLevel::High;
    let level2 = level1.clone();
    
    match (level1, level2) {
        (EdgeSecurityLevel::High, EdgeSecurityLevel::High) => {},
        _ => panic!("Clone failed"),
    }
}

// ============================================================================
// ResourceAllocationStrategy Tests (4 variants)
// ============================================================================

#[test]
fn test_resource_allocation_adaptive() {
    let strategy = ResourceAllocationStrategy::Adaptive;
    assert!(matches!(strategy, ResourceAllocationStrategy::Adaptive));
}

#[test]
fn test_resource_allocation_conservative() {
    let strategy = ResourceAllocationStrategy::Conservative;
    assert!(matches!(strategy, ResourceAllocationStrategy::Conservative));
}

#[test]
fn test_resource_allocation_aggressive() {
    let strategy = ResourceAllocationStrategy::Aggressive;
    assert!(matches!(strategy, ResourceAllocationStrategy::Aggressive));
}

#[test]
fn test_resource_allocation_custom() {
    let mut rules = HashMap::new();
    rules.insert("memory".to_string(), 0.8);
    rules.insert("cpu".to_string(), 0.6);
    
    let strategy = ResourceAllocationStrategy::Custom(rules.clone());
    
    match strategy {
        ResourceAllocationStrategy::Custom(r) => {
            assert_eq!(r.len(), 2);
            assert_eq!(r.get("memory"), Some(&0.8));
        }
        _ => panic!("Expected Custom variant"),
    }
}

#[test]
fn test_resource_allocation_clone() {
    let strategy1 = ResourceAllocationStrategy::Adaptive;
    let strategy2 = strategy1.clone();
    
    match (strategy1, strategy2) {
        (ResourceAllocationStrategy::Adaptive, ResourceAllocationStrategy::Adaptive) => {},
        _ => panic!("Clone failed"),
    }
}

// ============================================================================
// DeviceStatus Tests (6 variants)
// ============================================================================

#[test]
fn test_device_status_online() {
    let status = DeviceStatus::Online;
    assert_eq!(status, DeviceStatus::Online);
}

#[test]
fn test_device_status_offline() {
    let status = DeviceStatus::Offline;
    assert_eq!(status, DeviceStatus::Offline);
}

#[test]
fn test_device_status_busy() {
    let status = DeviceStatus::Busy;
    assert_eq!(status, DeviceStatus::Busy);
}

#[test]
fn test_device_status_error() {
    let status = DeviceStatus::Error;
    assert_eq!(status, DeviceStatus::Error);
}

#[test]
fn test_device_status_maintenance() {
    let status = DeviceStatus::Maintenance;
    assert_eq!(status, DeviceStatus::Maintenance);
}

#[test]
fn test_device_status_unknown() {
    let status = DeviceStatus::Unknown;
    assert_eq!(status, DeviceStatus::Unknown);
}

#[test]
fn test_device_status_clone() {
    let status1 = DeviceStatus::Online;
    let status2 = status1.clone();
    assert_eq!(status1, status2);
}

// ============================================================================
// NetworkInterfaceType Tests (8 variants)
// ============================================================================

#[test]
fn test_network_interface_ethernet() {
    let iface = NetworkInterfaceType::Ethernet;
    assert!(matches!(iface, NetworkInterfaceType::Ethernet));
}

#[test]
fn test_network_interface_wifi() {
    let iface = NetworkInterfaceType::WiFi;
    assert!(matches!(iface, NetworkInterfaceType::WiFi));
}

#[test]
fn test_network_interface_bluetooth() {
    let iface = NetworkInterfaceType::Bluetooth;
    assert!(matches!(iface, NetworkInterfaceType::Bluetooth));
}

#[test]
fn test_network_interface_lora() {
    let iface = NetworkInterfaceType::LoRa;
    assert!(matches!(iface, NetworkInterfaceType::LoRa));
}

#[test]
fn test_network_interface_zigbee() {
    let iface = NetworkInterfaceType::Zigbee;
    assert!(matches!(iface, NetworkInterfaceType::Zigbee));
}

#[test]
fn test_network_interface_can() {
    let iface = NetworkInterfaceType::CAN;
    assert!(matches!(iface, NetworkInterfaceType::CAN));
}

#[test]
fn test_network_interface_serial() {
    let iface = NetworkInterfaceType::Serial;
    assert!(matches!(iface, NetworkInterfaceType::Serial));
}

#[test]
fn test_network_interface_usb() {
    let iface = NetworkInterfaceType::USB;
    assert!(matches!(iface, NetworkInterfaceType::USB));
}

// ============================================================================
// ConnectionType Tests (7 variants)
// ============================================================================

#[test]
fn test_connection_type_serial() {
    let conn = ConnectionType::Serial;
    assert!(matches!(conn, ConnectionType::Serial));
}

#[test]
fn test_connection_type_network() {
    let conn = ConnectionType::Network;
    assert!(matches!(conn, ConnectionType::Network));
}

#[test]
fn test_connection_type_usb() {
    let conn = ConnectionType::USB;
    assert!(matches!(conn, ConnectionType::USB));
}

#[test]
fn test_connection_type_bluetooth() {
    let conn = ConnectionType::Bluetooth;
    assert!(matches!(conn, ConnectionType::Bluetooth));
}

#[test]
fn test_connection_type_wifi() {
    let conn = ConnectionType::WiFi;
    assert!(matches!(conn, ConnectionType::WiFi));
}

#[test]
fn test_connection_type_lora() {
    let conn = ConnectionType::LoRa;
    assert!(matches!(conn, ConnectionType::LoRa));
}

#[test]
fn test_connection_type_can() {
    let conn = ConnectionType::CAN;
    assert!(matches!(conn, ConnectionType::CAN));
}

// ============================================================================
// AuthenticationMethod Tests (5 variants)
// ============================================================================

#[test]
fn test_authentication_method_none() {
    let method = AuthenticationMethod::None;
    assert!(matches!(method, AuthenticationMethod::None));
}

#[test]
fn test_authentication_method_password() {
    let method = AuthenticationMethod::Password;
    assert!(matches!(method, AuthenticationMethod::Password));
}

#[test]
fn test_authentication_method_key() {
    let method = AuthenticationMethod::Key;
    assert!(matches!(method, AuthenticationMethod::Key));
}

#[test]
fn test_authentication_method_certificate() {
    let method = AuthenticationMethod::Certificate;
    assert!(matches!(method, AuthenticationMethod::Certificate));
}

#[test]
fn test_authentication_method_token() {
    let method = AuthenticationMethod::Token;
    assert!(matches!(method, AuthenticationMethod::Token));
}

// ============================================================================
// EncryptionAlgorithm Tests (5 variants)
// ============================================================================

#[test]
fn test_encryption_algorithm_none() {
    let algo = EncryptionAlgorithm::None;
    assert!(matches!(algo, EncryptionAlgorithm::None));
}

#[test]
fn test_encryption_algorithm_aes() {
    let algo = EncryptionAlgorithm::AES;
    assert!(matches!(algo, EncryptionAlgorithm::AES));
}

#[test]
fn test_encryption_algorithm_chacha20() {
    let algo = EncryptionAlgorithm::ChaCha20;
    assert!(matches!(algo, EncryptionAlgorithm::ChaCha20));
}

#[test]
fn test_encryption_algorithm_rsa() {
    let algo = EncryptionAlgorithm::RSA;
    assert!(matches!(algo, EncryptionAlgorithm::RSA));
}

#[test]
fn test_encryption_algorithm_ecc() {
    let algo = EncryptionAlgorithm::ECC;
    assert!(matches!(algo, EncryptionAlgorithm::ECC));
}

// ============================================================================
// EncryptionMode Tests (5 variants)
// ============================================================================

#[test]
fn test_encryption_mode_none() {
    let mode = EncryptionMode::None;
    assert!(matches!(mode, EncryptionMode::None));
}

#[test]
fn test_encryption_mode_gcm() {
    let mode = EncryptionMode::GCM;
    assert!(matches!(mode, EncryptionMode::GCM));
}

#[test]
fn test_encryption_mode_cbc() {
    let mode = EncryptionMode::CBC;
    assert!(matches!(mode, EncryptionMode::CBC));
}

#[test]
fn test_encryption_mode_ctr() {
    let mode = EncryptionMode::CTR;
    assert!(matches!(mode, EncryptionMode::CTR));
}

#[test]
fn test_encryption_mode_ecb() {
    let mode = EncryptionMode::ECB;
    assert!(matches!(mode, EncryptionMode::ECB));
}

// ============================================================================
// ResourceUsage Tests
// ============================================================================

#[test]
fn test_resource_usage_creation() {
    let usage = ResourceUsage {
        cpu_percent: 45.5,
        memory_bytes: 1024 * 1024,
        storage_bytes: 1024 * 1024 * 10,
        network_bytes_sent: 5000,
        network_bytes_received: 10000,
    };
    
    assert_eq!(usage.cpu_percent, 45.5);
    assert_eq!(usage.memory_bytes, 1024 * 1024);
    assert_eq!(usage.storage_bytes, 1024 * 1024 * 10);
    assert_eq!(usage.network_bytes_sent, 5000);
    assert_eq!(usage.network_bytes_received, 10000);
}

#[test]
fn test_resource_usage_clone() {
    let usage1 = ResourceUsage {
        cpu_percent: 50.0,
        memory_bytes: 2048,
        storage_bytes: 4096,
        network_bytes_sent: 1000,
        network_bytes_received: 2000,
    };
    
    let usage2 = usage1.clone();
    
    assert_eq!(usage1.cpu_percent, usage2.cpu_percent);
    assert_eq!(usage1.memory_bytes, usage2.memory_bytes);
}

