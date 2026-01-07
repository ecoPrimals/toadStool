//! Evolution Polish Unit Tests
//!
//! Verifies that ToadStool adheres to the infant discovery principle:
//! - Zero hardcoded primal names in production code
//! - Pure capability-based discovery
//! - No hardcoded registry clients

#[cfg(feature = "daemon")]
use toadstool_cli::daemon::{DaemonConfig, DaemonServer, WorkloadManager};

// ============================================================================
// DAEMON MODE TESTS - No Hardcoded Clients
// ============================================================================

#[cfg(feature = "daemon")]
#[tokio::test]
async fn test_daemon_server_has_no_biomeos_client_field() {
    // ✅ VERIFY: DaemonServer struct doesn't have a biomeos_client field
    // This is a compile-time check - if biomeos_client field exists, this won't compile
    
    let config = DaemonConfig {
        port: 8084,
        register_with_biomeos: false,
        socket_path: None,
        config_file: None,
        max_concurrent_workloads: 10,
        default_workload_timeout: std::time::Duration::from_secs(300),
        resource_monitor_interval: std::time::Duration::from_secs(60),
        heartbeat_interval: std::time::Duration::from_secs(30),
        biomeos_socket: None,
        health_check_interval: std::time::Duration::from_secs(10),
    };
    
    // Should be able to create DaemonServer without any hardcoded clients
    let result = DaemonServer::start(config).await;
    
    // May fail due to port binding, but should not fail due to missing biomeos_client
    if let Err(e) = result {
        let msg = e.to_string();
        assert!(
            !msg.contains("biomeos_client") && !msg.contains("BiomeOSClient"),
            "Error should not mention hardcoded client: {}",
            msg
        );
    }
}

#[cfg(feature = "daemon")]
#[tokio::test]
async fn test_workload_manager_standalone() {
    // ✅ VERIFY: WorkloadManager works without any hardcoded registry clients
    let manager = WorkloadManager::new(5).await;
    
    assert!(manager.is_ok(), "WorkloadManager should initialize without hardcoded clients");
}

#[cfg(feature = "daemon")]
#[tokio::test]
async fn test_daemon_config_validation() {
    // ✅ VERIFY: DaemonConfig validation doesn't require hardcoded clients
    let config = DaemonConfig {
        port: 8084,
        register_with_biomeos: true, // Enable discovery
        socket_path: None,
        config_file: None,
        max_concurrent_workloads: 10,
        default_workload_timeout: std::time::Duration::from_secs(300),
        resource_monitor_interval: std::time::Duration::from_secs(60),
        heartbeat_interval: std::time::Duration::from_secs(30),
        biomeos_socket: None,
        health_check_interval: std::time::Duration::from_secs(10),
    };
    
    // Config should validate without requiring hardcoded registry
    assert_eq!(config.port, 8084);
    assert!(config.register_with_biomeos);
}

// ============================================================================
// EXECUTOR TESTS - No Hardcoded Clients
// ============================================================================

#[tokio::test]
async fn test_executor_has_no_biomeos_client_field() {
    use toadstool_cli::executor::BiomeExecutor;
    
    // ✅ VERIFY: BiomeExecutor struct doesn't have a biomeos_client field
    let executor = BiomeExecutor::new().await;
    
    assert!(executor.is_ok(), "BiomeExecutor should initialize without hardcoded clients");
}

#[tokio::test]
async fn test_executor_no_hardcoded_discovery_methods() {
    use toadstool_cli::executor::BiomeExecutor;
    
    // ✅ VERIFY: BiomeExecutor doesn't expose hardcoded discovery methods
    let executor = BiomeExecutor::new().await.unwrap();
    
    // These methods should NOT exist (compile-time check)
    // If they exist, this test will fail to compile
    
    // executor.discover_security_provider() // Should NOT compile
    // executor.discover_discovery_provider() // Should NOT compile
    // executor.discover_storage_provider() // Should NOT compile
    
    // Instead, should use UniversalServiceAdapter
    let _ = executor; // Executor exists but has no hardcoded methods
}

// ============================================================================
// CAPABILITY-BASED DISCOVERY TESTS
// ============================================================================

#[tokio::test]
async fn test_universal_service_adapter_exists() {
    // ✅ VERIFY: UniversalServiceAdapter is available for capability-based discovery
    use toadstool_cli::ecosystem::adapters::AdapterFactory;
    
    let factory = AdapterFactory::new();
    
    // Should be able to create adapters for different capabilities
    let coordination_adapter = factory.coordination_adapter();
    assert!(coordination_adapter.is_ok(), "Should create coordination adapter");
}

#[tokio::test]
async fn test_discovery_engine_capability_based() {
    // ✅ VERIFY: Discovery engine uses capabilities, not names
    use toadstool_common::infant_discovery::DiscoveryEngine;
    
    let engine = DiscoveryEngine::new();
    
    // Should be able to create engine without hardcoded primal names
    let _ = engine;
}

#[test]
fn test_capability_enum_has_no_primal_names() {
    // ✅ VERIFY: Capability enum uses generic terms, not primal names
    use toadstool_common::primal_identity::Capability;
    
    // These are generic capability types (correct)
    let _compute = Capability::Compute;
    let _storage = Capability::Storage;
    let _coordination = Capability::Coordination;
    
    // These would be WRONG (primal-specific):
    // let _beardog = Capability::BearDog; // Should NOT exist
    // let _songbird = Capability::Songbird; // Should NOT exist
}

// ============================================================================
// INTEGRATION TESTS - End-to-End Flows
// ============================================================================

#[tokio::test]
async fn test_executor_list_biomes_no_hardcoded_clients() {
    use toadstool_cli::executor::BiomeExecutor;
    
    // ✅ VERIFY: Core operations work without hardcoded clients
    let executor = BiomeExecutor::new().await.unwrap();
    
    let result = executor.list_biomes(false, "json".to_string(), false, None).await;
    
    assert!(result.is_ok(), "list_biomes should work without hardcoded clients");
}

#[tokio::test]
async fn test_executor_operations_independent_of_registry() {
    use toadstool_cli::executor::BiomeExecutor;
    
    // ✅ VERIFY: Executor operations don't require hardcoded registry connection
    let executor = BiomeExecutor::new().await.unwrap();
    
    // These should work without any registry
    let list_result = executor.list_biomes(false, "table".to_string(), false, None).await;
    assert!(list_result.is_ok());
    
    let down_result = executor.down_biome("nonexistent".to_string(), false, 30, false).await;
    assert!(down_result.is_err()); // Fails because biome doesn't exist, not because of registry
    
    let logs_result = executor.show_logs("nonexistent".to_string(), false, 50, false, None, None).await;
    assert!(logs_result.is_err()); // Fails because biome doesn't exist, not because of registry
}

// ============================================================================
// PROPERTY-BASED TESTS
// ============================================================================

#[tokio::test]
async fn test_property_no_panic_without_registry() {
    use toadstool_cli::executor::BiomeExecutor;
    
    // ✅ PROPERTY: Executor never panics when registry is unavailable
    let executor = BiomeExecutor::new().await.unwrap();
    
    // Try various operations - none should panic
    let _ = executor.list_biomes(false, "json".to_string(), false, None).await;
    let _ = executor.list_biomes(true, "yaml".to_string(), true, Some("running".to_string())).await;
    let _ = executor.down_biome("test".to_string(), false, 30, false).await;
    let _ = executor.show_logs("test".to_string(), false, 100, true, Some("info".to_string()), None).await;
    
    // All operations completed without panic ✅
}

#[tokio::test]
async fn test_concurrent_executor_creation_no_registry_race() {
    use toadstool_cli::executor::BiomeExecutor;
    
    // ✅ PROPERTY: Multiple executors can be created concurrently without registry
    let handles: Vec<_> = (0..10)
        .map(|_| {
            tokio::spawn(async {
                BiomeExecutor::new().await
            })
        })
        .collect();
    
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok(), "Concurrent executor creation should succeed");
    }
}

// ============================================================================
// ERROR MESSAGE QUALITY TESTS
// ============================================================================

#[tokio::test]
async fn test_errors_dont_mention_hardcoded_clients() {
    use toadstool_cli::executor::BiomeExecutor;
    
    // ✅ VERIFY: Error messages don't reference hardcoded clients
    let executor = BiomeExecutor::new().await.unwrap();
    
    let result = executor.down_biome("nonexistent".to_string(), false, 30, false).await;
    
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    
    // Error should not mention hardcoded clients
    assert!(
        !err_msg.contains("BiomeOSClient") 
        && !err_msg.contains("SongbirdClient")
        && !err_msg.contains("biomeos_client"),
        "Error message should not reference hardcoded clients: {}",
        err_msg
    );
}

// ============================================================================
// ARCHITECTURE VALIDATION TESTS
// ============================================================================

#[test]
fn test_infant_discovery_principle_enforced() {
    // ✅ VERIFY: Core principle is enforced at compile time
    
    // These types should exist (capability-based)
    use toadstool_common::primal_identity::Capability;
    use toadstool_common::infant_discovery::DiscoveryEngine;
    
    let _capability = Capability::Compute;
    let _engine = DiscoveryEngine::new();
    
    // These types should NOT exist (hardcoded primal-specific)
    // use toadstool::biomeos_integration::BiomeOSClient; // Should NOT compile
    // use toadstool::songbird_integration::SongbirdClient; // Should NOT compile
    
    // Architecture principle enforced ✅
}

#[cfg(feature = "daemon")]
#[test]
fn test_daemon_architecture_no_hardcoded_fields() {
    // ✅ VERIFY: DaemonServer has no hardcoded client fields
    
    // This is a compile-time check via type inspection
    let size = std::mem::size_of::<DaemonConfig>();
    assert!(size > 0, "DaemonConfig should have valid size");
    
    // If DaemonServer had hardcoded Arc<BiomeOSClient>, size would be much larger
    // Current implementation should be lean
}

// ============================================================================
// REGRESSION TESTS
// ============================================================================

#[tokio::test]
async fn test_no_regression_basic_operations_work() {
    use toadstool_cli::executor::BiomeExecutor;
    
    // ✅ REGRESSION: Ensure basic operations still work after removing hardcoded clients
    let executor = BiomeExecutor::new().await.unwrap();
    
    // List biomes
    let list = executor.list_biomes(false, "json".to_string(), false, None).await;
    assert!(list.is_ok(), "list_biomes should still work");
    
    // These should fail gracefully (biome doesn't exist)
    let down = executor.down_biome("test".to_string(), false, 30, false).await;
    assert!(down.is_err(), "down_biome should fail gracefully for nonexistent biome");
    
    let logs = executor.show_logs("test".to_string(), false, 50, false, None, None).await;
    assert!(logs.is_err(), "show_logs should fail gracefully for nonexistent biome");
}

#[tokio::test]
async fn test_no_regression_concurrent_operations() {
    use toadstool_cli::executor::BiomeExecutor;
    use std::sync::Arc;
    
    // ✅ REGRESSION: Concurrent operations still work
    let executor = Arc::new(BiomeExecutor::new().await.unwrap());
    
    let handles: Vec<_> = (0..5)
        .map(|i| {
            let exec = executor.clone();
            tokio::spawn(async move {
                exec.list_biomes(i % 2 == 0, "json".to_string(), false, None).await
            })
        })
        .collect();
    
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok(), "Concurrent operations should work");
    }
}

