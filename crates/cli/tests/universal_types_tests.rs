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
//! Comprehensive tests for CLI universal types

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use toadstool_cli::universal::*;
use uuid::Uuid;

// ============================================================================
// BenchmarkType Tests
// ============================================================================

#[test]
fn test_benchmark_type_cpu_integer() {
    let bench_type = BenchmarkType::CpuInteger;
    assert!(matches!(bench_type, BenchmarkType::CpuInteger));
}

#[test]
fn test_benchmark_type_cpu_float() {
    let bench_type = BenchmarkType::CpuFloat;
    assert!(matches!(bench_type, BenchmarkType::CpuFloat));
}

#[test]
fn test_benchmark_type_memory() {
    let bench_type = BenchmarkType::Memory;
    assert!(matches!(bench_type, BenchmarkType::Memory));
}

#[test]
fn test_benchmark_type_storage() {
    let bench_type = BenchmarkType::Storage;
    assert!(matches!(bench_type, BenchmarkType::Storage));
}

#[test]
fn test_benchmark_type_network() {
    let bench_type = BenchmarkType::Network;
    assert!(matches!(bench_type, BenchmarkType::Network));
}

#[test]
fn test_benchmark_type_gpu() {
    let bench_type = BenchmarkType::Gpu;
    assert!(matches!(bench_type, BenchmarkType::Gpu));
}

#[test]
fn test_benchmark_type_wasm_execution() {
    let bench_type = BenchmarkType::WasmExecution;
    assert!(matches!(bench_type, BenchmarkType::WasmExecution));
}

#[test]
fn test_benchmark_type_container_startup() {
    let bench_type = BenchmarkType::ContainerStartup;
    assert!(matches!(bench_type, BenchmarkType::ContainerStartup));
}

#[test]
fn test_benchmark_type_custom() {
    let bench_type = BenchmarkType::Custom("crypto-mining".to_string());

    if let BenchmarkType::Custom(name) = bench_type {
        assert_eq!(name, "crypto-mining");
    } else {
        panic!("Expected Custom variant");
    }
}

#[test]
fn test_benchmark_type_clone() {
    let bench_type = BenchmarkType::CpuInteger;
    let cloned = bench_type.clone();
    assert!(matches!(cloned, BenchmarkType::CpuInteger));
}

// ============================================================================
// BenchmarkTest Tests
// ============================================================================

#[test]
fn test_benchmark_test_creation() {
    let test = BenchmarkTest {
        name: "CPU Performance".to_string(),
        test_type: BenchmarkType::CpuInteger,
        duration: Duration::from_secs(5),
        score: 1500.0,
        unit: "ops/sec".to_string(),
        details: HashMap::new(),
    };

    assert_eq!(test.name, "CPU Performance");
    assert_eq!(test.score, 1500.0);
}

#[test]
fn test_benchmark_test_with_details() {
    let mut details = HashMap::new();
    details.insert("cores_used".to_string(), serde_json::json!(8));
    details.insert("threads".to_string(), serde_json::json!(16));

    let test = BenchmarkTest {
        name: "Multi-core Test".to_string(),
        test_type: BenchmarkType::CpuFloat,
        duration: Duration::from_secs(10),
        score: 2500.0,
        unit: "GFLOPS".to_string(),
        details,
    };

    assert_eq!(test.details.len(), 2);
    assert_eq!(
        test.details.get("cores_used").unwrap(),
        &serde_json::json!(8)
    );
}

#[test]
fn test_benchmark_test_clone() {
    let test = BenchmarkTest {
        name: "Test".to_string(),
        test_type: BenchmarkType::Memory,
        duration: Duration::from_secs(1),
        score: 100.0,
        unit: "MB/s".to_string(),
        details: HashMap::new(),
    };

    let cloned = test.clone();
    assert_eq!(test.name, cloned.name);
    assert_eq!(test.score, cloned.score);
}

// ============================================================================
// FederationStatus Tests
// ============================================================================

#[test]
fn test_federation_status_connecting() {
    let status = FederationStatus::Connecting;
    assert!(matches!(status, FederationStatus::Connecting));
}

#[test]
fn test_federation_status_connected() {
    let status = FederationStatus::Connected;
    assert!(matches!(status, FederationStatus::Connected));
}

#[test]
fn test_federation_status_syncing() {
    let status = FederationStatus::Syncing;
    assert!(matches!(status, FederationStatus::Syncing));
}

#[test]
fn test_federation_status_ready() {
    let status = FederationStatus::Ready;
    assert!(matches!(status, FederationStatus::Ready));
}

#[test]
fn test_federation_status_disconnected() {
    let status = FederationStatus::Disconnected;
    assert!(matches!(status, FederationStatus::Disconnected));
}

#[test]
fn test_federation_status_error() {
    let status = FederationStatus::Error("Connection timeout".to_string());

    if let FederationStatus::Error(msg) = status {
        assert_eq!(msg, "Connection timeout");
    } else {
        panic!("Expected Error variant");
    }
}

#[test]
fn test_federation_status_clone() {
    let status = FederationStatus::Ready;
    let cloned = status.clone();
    assert!(matches!(cloned, FederationStatus::Ready));
}

// ============================================================================
// TrustLevel Tests
// ============================================================================

#[test]
fn test_trust_level_unknown() {
    let level = TrustLevel::Unknown;
    assert!(matches!(level, TrustLevel::Unknown));
}

#[test]
fn test_trust_level_untrusted() {
    let level = TrustLevel::Untrusted;
    assert!(matches!(level, TrustLevel::Untrusted));
}

#[test]
fn test_trust_level_verified() {
    let level = TrustLevel::Verified;
    assert!(matches!(level, TrustLevel::Verified));
}

#[test]
fn test_trust_level_sovereign() {
    let level = TrustLevel::Sovereign;
    assert!(matches!(level, TrustLevel::Sovereign));
}

#[test]
fn test_trust_level_clone() {
    let level = TrustLevel::Sovereign;
    let cloned = level.clone();
    assert!(matches!(cloned, TrustLevel::Sovereign));
}

// ============================================================================
// FederationPeer Tests
// ============================================================================

#[test]
fn test_federation_peer_creation() {
    let peer = FederationPeer {
        peer_id: Uuid::new_v4(),
        endpoint: "127.0.0.1:8080".parse().unwrap(),
        capabilities: vec![Arc::from("compute"), Arc::from("storage")],
        shared_resources: vec![Arc::from("cpu")],
        status: FederationStatus::Connected,
        last_heartbeat: std::time::SystemTime::now(),
        trust_level: TrustLevel::Verified,
    };

    assert_eq!(peer.capabilities.len(), 2);
    assert!(peer.capabilities.contains(&Arc::from("compute")));
}

#[test]
fn test_federation_peer_with_multiple_resources() {
    let peer = FederationPeer {
        peer_id: Uuid::new_v4(),
        endpoint: "10.0.0.1:9000".parse().unwrap(),
        capabilities: vec![Arc::from("gpu")],
        shared_resources: vec![Arc::from("nvidia-3090"), Arc::from("amd-mi100")],
        status: FederationStatus::Ready,
        last_heartbeat: std::time::SystemTime::now(),
        trust_level: TrustLevel::Sovereign,
    };

    assert_eq!(peer.shared_resources.len(), 2);
    assert!(matches!(peer.trust_level, TrustLevel::Sovereign));
}

#[test]
fn test_federation_peer_clone() {
    let peer = FederationPeer {
        peer_id: Uuid::new_v4(),
        endpoint: "127.0.0.1:8080".parse().unwrap(),
        capabilities: vec![],
        shared_resources: vec![],
        status: FederationStatus::Connected,
        last_heartbeat: std::time::SystemTime::now(),
        trust_level: TrustLevel::Unknown,
    };

    let cloned = peer.clone();
    assert_eq!(peer.peer_id, cloned.peer_id);
}

// ============================================================================
// PlatformStatus Tests
// ============================================================================

#[test]
fn test_platform_status_available() {
    let status = PlatformStatus::Available;
    assert!(matches!(status, PlatformStatus::Available));
}

#[test]
fn test_platform_status_testing() {
    let status = PlatformStatus::Testing;
    assert!(matches!(status, PlatformStatus::Testing));
}

#[test]
fn test_platform_status_degraded() {
    let status = PlatformStatus::Degraded;
    assert!(matches!(status, PlatformStatus::Degraded));
}

#[test]
fn test_platform_status_unavailable() {
    let status = PlatformStatus::Unavailable;
    assert!(matches!(status, PlatformStatus::Unavailable));
}

#[test]
fn test_platform_status_error() {
    let status = PlatformStatus::Error("Platform initialization failed".to_string());

    if let PlatformStatus::Error(msg) = status {
        assert!(msg.contains("failed"));
    } else {
        panic!("Expected Error variant");
    }
}

#[test]
fn test_platform_status_clone() {
    let status = PlatformStatus::Available;
    let cloned = status.clone();
    assert!(matches!(cloned, PlatformStatus::Available));
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_benchmark_types_all_variants() {
    let types = vec![
        BenchmarkType::CpuInteger,
        BenchmarkType::CpuFloat,
        BenchmarkType::Memory,
        BenchmarkType::Storage,
        BenchmarkType::Network,
        BenchmarkType::Gpu,
        BenchmarkType::WasmExecution,
        BenchmarkType::ContainerStartup,
        BenchmarkType::Custom("test".to_string()),
    ];

    assert_eq!(types.len(), 9);
}

#[test]
fn test_trust_level_progression() {
    let levels = [
        TrustLevel::Unknown,
        TrustLevel::Untrusted,
        TrustLevel::Verified,
        TrustLevel::Sovereign,
    ];

    assert_eq!(levels.len(), 4);
}

#[test]
fn test_federation_status_lifecycle() {
    let statuses = [
        FederationStatus::Connecting,
        FederationStatus::Connected,
        FederationStatus::Syncing,
        FederationStatus::Ready,
        FederationStatus::Disconnected,
    ];

    assert_eq!(statuses.len(), 5);
}

#[test]
fn test_platform_status_all_states() {
    let statuses = [
        PlatformStatus::Available,
        PlatformStatus::Testing,
        PlatformStatus::Degraded,
        PlatformStatus::Unavailable,
        PlatformStatus::Error("test".to_string()),
    ];

    assert_eq!(statuses.len(), 5);
}

#[test]
fn test_benchmark_test_serialization() {
    let test = BenchmarkTest {
        name: "Serialization Test".to_string(),
        test_type: BenchmarkType::CpuInteger,
        duration: Duration::from_secs(1),
        score: 100.0,
        unit: "ops".to_string(),
        details: HashMap::new(),
    };

    let json = serde_json::to_string(&test).expect("Failed to serialize");
    let deserialized: BenchmarkTest = serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(test.name, deserialized.name);
    assert_eq!(test.score, deserialized.score);
}

#[test]
fn test_federation_peer_with_sovereign_trust() {
    let peer = FederationPeer {
        peer_id: Uuid::new_v4(),
        endpoint: "192.168.1.100:8080".parse().unwrap(),
        capabilities: vec![Arc::from("full-trust")],
        shared_resources: vec![Arc::from("all")],
        status: FederationStatus::Ready,
        last_heartbeat: std::time::SystemTime::now(),
        trust_level: TrustLevel::Sovereign,
    };

    assert!(matches!(peer.trust_level, TrustLevel::Sovereign));
    assert!(matches!(peer.status, FederationStatus::Ready));
}
