//! Comprehensive tests for BiomeOS Backend Implementations
//!
//! This test file provides comprehensive coverage for production backend implementations:
//! - BearDogBackend (authentication)
//! - NestGateBackend (storage)
//! - SquirrelBackend (agent deployment)
//!
//! Tests cover construction, API integration patterns, error handling, and validation logic.

use std::sync::Arc;
use toadstool::biomeos_integration::agent_backend::*;
use toadstool::biomeos_integration::auth_backend::*;
use toadstool::biomeos_integration::storage_backend::*;
use toadstool::biomeos_integration::types::*;

// ============================================================================
// BearDogBackend Tests - Authentication Backend (15 tests)
// ============================================================================

#[test]
fn test_beardog_backend_creation() {
    let _backend = BearDogBackend::new("http://beardog:8081");
    // Backend should be constructed successfully
    // (We can't test internal fields directly, but construction validates the API)
}

#[test]
fn test_beardog_backend_creation_with_string() {
    let endpoint = String::from("http://beardog:8081");
    let _backend = BearDogBackend::new(endpoint);
    // Backend should accept String as endpoint
}

#[test]
fn test_beardog_backend_creation_with_str() {
    let _backend = BearDogBackend::new("http://beardog:8081");
    // Backend should accept &str as endpoint
}

#[test]
fn test_beardog_backend_creation_various_endpoints() {
    // Test different endpoint formats
    let endpoints = vec![
        "http://localhost:8081",
        "http://beardog:8081",
        "https://beardog.example.com",
        "http://192.168.1.100:8081",
    ];

    for endpoint in endpoints {
        let _backend = BearDogBackend::new(endpoint);
        // Each endpoint should create a valid backend
    }
}

#[test]
fn test_auth_backend_trait_validate_token_valid() {
    let token = AuthenticationToken {
        id: "test-token-123".to_string(),
        token_type: "Bearer".to_string(),
        token: "test-value".to_string(),
        public_key: "test-public-key".to_string(),
        expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
        issued_at: chrono::Utc::now(),
        issuer: "beardog".to_string(),
        audience: vec!["songbird".to_string()],
        scope: vec!["cross-primal".to_string()],
        claims: std::collections::HashMap::new(),
    };

    let backend = InMemoryAuthBackend::new();
    let result = backend.validate_token(&token);
    assert!(result.is_ok());
}

#[test]
fn test_auth_backend_trait_validate_token_expired() {
    let token = AuthenticationToken {
        id: "expired-token".to_string(),
        token_type: "Bearer".to_string(),
        token: "test-value".to_string(),
        public_key: "test-public-key".to_string(),
        expires_at: chrono::Utc::now() - chrono::Duration::hours(1), // Expired
        issued_at: chrono::Utc::now() - chrono::Duration::hours(2),
        issuer: "beardog".to_string(),
        audience: vec!["songbird".to_string()],
        scope: vec!["cross-primal".to_string()],
        claims: std::collections::HashMap::new(),
    };

    let backend = InMemoryAuthBackend::new();
    let result = backend.validate_token(&token);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("expired"));
}

#[test]
fn test_auth_backend_trait_validate_token_invalid_issuer() {
    let token = AuthenticationToken {
        id: "test-token-123".to_string(),
        token_type: "Bearer".to_string(),
        token: "test-value".to_string(),
        public_key: "test-public-key".to_string(),
        expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
        issued_at: chrono::Utc::now(),
        issuer: "malicious-issuer".to_string(), // Invalid issuer
        audience: vec!["songbird".to_string()],
        scope: vec!["cross-primal".to_string()],
        claims: std::collections::HashMap::new(),
    };

    let backend = InMemoryAuthBackend::new();
    let result = backend.validate_token(&token);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("issuer"));
}

#[test]
fn test_auth_backend_trait_validate_token_invalid_type() {
    let token = AuthenticationToken {
        id: "test-token-123".to_string(),
        token_type: "InvalidType".to_string(), // Invalid token type
        token: "test-value".to_string(),
        public_key: "test-public-key".to_string(),
        expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
        issued_at: chrono::Utc::now(),
        issuer: "beardog".to_string(),
        audience: vec!["songbird".to_string()],
        scope: vec!["cross-primal".to_string()],
        claims: std::collections::HashMap::new(),
    };

    let backend = InMemoryAuthBackend::new();
    let result = backend.validate_token(&token);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("token type"));
}

#[test]
fn test_auth_backend_trait_validate_token_bearer_type() {
    let token = AuthenticationToken {
        id: "test-token-123".to_string(),
        token_type: "Bearer".to_string(), // Valid Bearer type
        token: "test-value".to_string(),
        public_key: "test-public-key".to_string(),
        expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
        issued_at: chrono::Utc::now(),
        issuer: "beardog".to_string(),
        audience: vec!["songbird".to_string()],
        scope: vec!["cross-primal".to_string()],
        claims: std::collections::HashMap::new(),
    };

    let backend = InMemoryAuthBackend::new();
    let result = backend.validate_token(&token);
    assert!(result.is_ok());
}

#[test]
fn test_auth_backend_trait_validate_token_ed25519_type() {
    let token = AuthenticationToken {
        id: "test-token-123".to_string(),
        token_type: "Ed25519".to_string(), // Valid Ed25519 type
        token: "test-value".to_string(),
        public_key: "test-public-key".to_string(),
        expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
        issued_at: chrono::Utc::now(),
        issuer: "beardog".to_string(),
        audience: vec!["songbird".to_string()],
        scope: vec!["cross-primal".to_string()],
        claims: std::collections::HashMap::new(),
    };

    let backend = InMemoryAuthBackend::new();
    let result = backend.validate_token(&token);
    assert!(result.is_ok());
}

#[test]
fn test_inmemory_auth_backend_default() {
    let _backend = InMemoryAuthBackend::default();
    // Default should construct successfully
}

#[test]
fn test_inmemory_auth_backend_new() {
    let _backend = InMemoryAuthBackend::new();
    // new() should construct successfully
}

#[tokio::test]
async fn test_inmemory_auth_backend_initialize_default() {
    let backend = InMemoryAuthBackend::new();
    let result = backend.initialize().await;
    // InMemory backend initialize should be a no-op and succeed
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_inmemory_auth_backend_token_request_fields() {
    let backend = InMemoryAuthBackend::new();
    let request = TokenRequest {
        requesting_primal: "toadstool".to_string(),
        scope: vec!["cross-primal".to_string(), "read".to_string()],
        audience: vec!["songbird".to_string(), "nestgate".to_string()],
        timestamp: chrono::Utc::now(),
    };

    let token = backend.request_token(&request).await.unwrap();

    // Validate token fields
    assert!(token.id.contains("toadstool"));
    assert_eq!(token.token_type, "Bearer");
    assert!(token.token.contains("toadstool"));
    assert_eq!(token.public_key, "test-public-key");
    assert!(token.expires_at > chrono::Utc::now());
    assert_eq!(token.issuer, "beardog");
    assert!(token.audience.contains(&"songbird".to_string()));
    assert!(token.scope.contains(&"cross-primal".to_string()));
}

#[tokio::test]
async fn test_inmemory_auth_backend_refresh_token_fields() {
    let backend = InMemoryAuthBackend::new();
    let request = TokenRefreshRequest {
        requesting_primal: "toadstool".to_string(),
        timestamp: chrono::Utc::now(),
    };

    let token = backend.refresh_token(&request).await.unwrap();

    // Validate refreshed token fields
    assert!(token.id.contains("refreshed"));
    assert!(token.id.contains("toadstool"));
    assert_eq!(token.token_type, "Bearer");
    assert!(token.token.contains("refreshed"));
    assert_eq!(token.issuer, "beardog");
    assert!(token.expires_at > chrono::Utc::now());
}

// ============================================================================
// NestGateBackend Tests - Storage Backend (12 tests)
// ============================================================================

#[test]
fn test_nestgate_backend_creation() {
    let _backend = NestGateBackend::new("http://nestgate:8082", "standard", true, 3);
    // Backend should be constructed successfully
}

#[test]
fn test_nestgate_backend_creation_with_string() {
    let endpoint = String::from("http://nestgate:8082");
    let storage_tier = String::from("premium");
    let _backend = NestGateBackend::new(endpoint, storage_tier, false, 1);
    // Backend should accept String as parameters
}

#[test]
fn test_nestgate_backend_creation_with_str() {
    let _backend = NestGateBackend::new("http://nestgate:8082", "hot", true, 2);
    // Backend should accept &str as parameters
}

#[test]
fn test_nestgate_backend_creation_various_configs() {
    let configs = vec![
        ("http://localhost:8082", "standard", false, 1),
        ("http://nestgate:8082", "premium", true, 3),
        ("https://nestgate.example.com", "hot", true, 5),
        ("http://192.168.1.100:8082", "cold", false, 1),
    ];

    for (endpoint, tier, replication, factor) in configs {
        let _backend = NestGateBackend::new(endpoint, tier, replication, factor);
        // Each config should create a valid backend
    }
}

#[test]
fn test_volume_status_creating() {
    let status = VolumeStatus::Creating;
    assert_eq!(status, VolumeStatus::Creating);
}

#[test]
fn test_volume_status_available() {
    let status = VolumeStatus::Available;
    assert_eq!(status, VolumeStatus::Available);
}

#[test]
fn test_volume_status_attaching() {
    let status = VolumeStatus::Attaching;
    assert_eq!(status, VolumeStatus::Attaching);
}

#[test]
fn test_volume_status_in_use() {
    let status = VolumeStatus::InUse;
    assert_eq!(status, VolumeStatus::InUse);
}

#[test]
fn test_volume_status_detaching() {
    let status = VolumeStatus::Detaching;
    assert_eq!(status, VolumeStatus::Detaching);
}

#[test]
fn test_volume_status_deleting() {
    let status = VolumeStatus::Deleting;
    assert_eq!(status, VolumeStatus::Deleting);
}

#[test]
fn test_volume_status_error() {
    let status = VolumeStatus::Error("Disk full".to_string());
    match status {
        VolumeStatus::Error(msg) => assert_eq!(msg, "Disk full"),
        _ => panic!("Expected Error variant"),
    }
}

#[test]
fn test_volume_status_equality() {
    assert_eq!(VolumeStatus::Creating, VolumeStatus::Creating);
    assert_eq!(VolumeStatus::Available, VolumeStatus::Available);
    assert_ne!(VolumeStatus::Creating, VolumeStatus::Available);
    assert_eq!(
        VolumeStatus::Error("test".to_string()),
        VolumeStatus::Error("test".to_string())
    );
}

// ============================================================================
// SquirrelBackend Tests - Agent Backend (15 tests)
// ============================================================================

#[test]
fn test_squirrel_backend_creation() {
    let _backend = SquirrelBackend::new("http://squirrel:7070", "local", "container", true);
    // Backend should be constructed successfully
}

#[test]
fn test_squirrel_backend_creation_with_strings() {
    let endpoint = String::from("http://squirrel:7070");
    let registry = String::from("huggingface");
    let runtime = String::from("process");
    let _backend = SquirrelBackend::new(endpoint, registry, runtime, false);
    // Backend should accept String types
}

#[test]
fn test_squirrel_backend_creation_various_configs() {
    let configs = vec![
        ("http://localhost:7070", "local", "container", true),
        ("http://squirrel:7070", "huggingface", "process", false),
        ("https://squirrel.example.com", "ollama", "docker", true),
    ];

    for (endpoint, registry, runtime, mcp) in configs {
        let _backend = SquirrelBackend::new(endpoint, registry, runtime, mcp);
        // Each config should create a valid backend
    }
}

#[test]
fn test_agent_status_deploying() {
    let status = AgentStatus::Deploying;
    assert_eq!(status, AgentStatus::Deploying);
}

#[test]
fn test_agent_status_running() {
    let status = AgentStatus::Running;
    assert_eq!(status, AgentStatus::Running);
}

#[test]
fn test_agent_status_scaling() {
    let status = AgentStatus::Scaling;
    assert_eq!(status, AgentStatus::Scaling);
}

#[test]
fn test_agent_status_updating() {
    let status = AgentStatus::Updating;
    assert_eq!(status, AgentStatus::Updating);
}

#[test]
fn test_agent_status_terminating() {
    let status = AgentStatus::Terminating;
    assert_eq!(status, AgentStatus::Terminating);
}

#[test]
fn test_agent_status_failed() {
    let status = AgentStatus::Failed("Out of memory".to_string());
    match status {
        AgentStatus::Failed(msg) => assert_eq!(msg, "Out of memory"),
        _ => panic!("Expected Failed variant"),
    }
}

#[test]
fn test_agent_status_stopped() {
    let status = AgentStatus::Stopped;
    assert_eq!(status, AgentStatus::Stopped);
}

#[test]
fn test_agent_status_equality() {
    assert_eq!(AgentStatus::Running, AgentStatus::Running);
    assert_eq!(AgentStatus::Deploying, AgentStatus::Deploying);
    assert_ne!(AgentStatus::Running, AgentStatus::Stopped);
    assert_eq!(
        AgentStatus::Failed("test".to_string()),
        AgentStatus::Failed("test".to_string())
    );
}

#[test]
fn test_model_status_loading() {
    let status = ModelStatus::Loading;
    assert_eq!(status, ModelStatus::Loading);
}

#[test]
fn test_model_status_ready() {
    let status = ModelStatus::Ready;
    assert_eq!(status, ModelStatus::Ready);
}

#[test]
fn test_model_status_updating() {
    let status = ModelStatus::Updating;
    assert_eq!(status, ModelStatus::Updating);
}

#[test]
fn test_model_status_unloading() {
    let status = ModelStatus::Unloading;
    assert_eq!(status, ModelStatus::Unloading);
}

// ============================================================================
// InMemoryBackend Tests - Storage Test Backend (10 tests)
// ============================================================================

#[test]
fn test_inmemory_storage_backend_creation() {
    let _backend = InMemoryBackend::new("standard");
    // Backend should be constructed successfully
}

#[test]
fn test_inmemory_storage_backend_creation_with_string() {
    let tier = String::from("premium");
    let _backend = InMemoryBackend::new(tier);
    // Backend should accept String as storage_tier
}

#[tokio::test]
async fn test_inmemory_storage_backend_initialize() {
    let backend = InMemoryBackend::new("standard");
    let result = backend.initialize().await;
    // InMemory backend initialize should succeed
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_inmemory_storage_backend_provision_basic() {
    let backend = Arc::new(InMemoryBackend::new("standard"));
    let config = VolumeConfig {
        name: "test-volume".to_string(),
        size: "10Gi".to_string(),
        storage_class: Some("standard".to_string()),
        access_modes: vec!["ReadWriteOnce".to_string()],
        mount_path: Some("/mnt/data".to_string()),
        backup_policy: None,
    };

    let result = backend.provision_volume(&config).await;
    assert!(result.is_ok());

    let info = result.unwrap();
    assert_eq!(info.name, "test-volume");
    assert_eq!(info.status, "Available");
}

#[tokio::test]
async fn test_inmemory_storage_backend_provision_multiple() {
    let backend = Arc::new(InMemoryBackend::new("standard"));

    for i in 0..5 {
        let config = VolumeConfig {
            name: format!("volume-{}", i),
            size: "1Gi".to_string(),
            storage_class: None,
            access_modes: vec!["ReadWriteOnce".to_string()],
            mount_path: None,
            backup_policy: None,
        };

        let result = backend.provision_volume(&config).await;
        assert!(result.is_ok());
    }

    // List should show all 5 volumes
    let volumes = backend.list_volumes().await.unwrap();
    assert_eq!(volumes.len(), 5);
}

#[tokio::test]
async fn test_inmemory_storage_backend_volume_lifecycle() {
    let backend = Arc::new(InMemoryBackend::new("premium"));
    let config = VolumeConfig {
        name: "lifecycle-test".to_string(),
        size: "5Gi".to_string(),
        storage_class: Some("fast".to_string()),
        access_modes: vec!["ReadWriteOnce".to_string()],
        mount_path: Some("/data".to_string()),
        backup_policy: Some("daily".to_string()),
    };

    // Provision
    let volume = backend.provision_volume(&config).await.unwrap();
    assert_eq!(volume.status, "Available");

    // Get status
    let status = backend.get_volume_status(&volume.name).await.unwrap();
    assert_eq!(status, VolumeStatus::Available);

    // Mount (InMemoryBackend is a simplified test implementation,
    // so it doesn't track mount state - just verifies volume exists)
    let mount_result = backend
        .mount_volume(&volume.name, "test-service", "/mnt/test")
        .await;
    assert!(mount_result.is_ok());

    // Unmount (also simplified - just verifies volume exists)
    let unmount_result = backend.unmount_volume(&volume.name, "test-service").await;
    assert!(unmount_result.is_ok());

    // Status should still be Available (test backend doesn't track mount state)
    let status = backend.get_volume_status(&volume.name).await.unwrap();
    assert_eq!(status, VolumeStatus::Available);

    // Delete
    backend.delete_volume(&volume.name).await.unwrap();
    let status_result = backend.get_volume_status(&volume.name).await;
    assert!(status_result.is_err());
}

#[tokio::test]
async fn test_inmemory_storage_backend_list_empty() {
    let backend = Arc::new(InMemoryBackend::new("standard"));
    let volumes = backend.list_volumes().await.unwrap();
    assert_eq!(volumes.len(), 0);
}

#[tokio::test]
async fn test_inmemory_storage_backend_mount_nonexistent() {
    let backend = Arc::new(InMemoryBackend::new("standard"));
    let result = backend
        .mount_volume("nonexistent", "test-service", "/mnt")
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_inmemory_storage_backend_unmount_nonexistent() {
    let backend = Arc::new(InMemoryBackend::new("standard"));
    let result = backend.unmount_volume("nonexistent", "test-service").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_inmemory_storage_backend_delete_nonexistent() {
    let backend = Arc::new(InMemoryBackend::new("standard"));
    let result = backend.delete_volume("nonexistent").await;
    assert!(result.is_err());
}

// ============================================================================
// InMemoryAgentBackend Tests - Agent Test Backend (10 tests)
// ============================================================================

#[test]
fn test_inmemory_agent_backend_creation() {
    let _backend = InMemoryAgentBackend::new();
    // Backend should be constructed successfully
}

#[test]
fn test_inmemory_agent_backend_default() {
    let _backend = InMemoryAgentBackend::default();
    // Default should construct successfully
}

#[tokio::test]
async fn test_inmemory_agent_backend_initialize() {
    let backend = InMemoryAgentBackend::new();
    let result = backend.initialize().await;
    // InMemory backend initialize should succeed
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_inmemory_agent_backend_deploy_basic() {
    let backend = InMemoryAgentBackend::new();
    let config = AgentConfig {
        name: "basic-agent".to_string(),
        model: "gpt-4".to_string(),
        capabilities: vec!["chat".to_string(), "reasoning".to_string()],
        resources: None,
        environment: std::collections::HashMap::new(),
        config: std::collections::HashMap::new(),
    };

    let result = backend.deploy_agent(&config).await;
    assert!(result.is_ok());

    let info = result.unwrap();
    assert_eq!(info.name, "basic-agent");
    assert_eq!(info.model, "gpt-4");
    assert_eq!(info.status, AgentStatus::Running);
    assert_eq!(info.replicas, 1);
}

#[tokio::test]
async fn test_inmemory_agent_backend_scale_agent() {
    let backend = InMemoryAgentBackend::new();
    let config = AgentConfig {
        name: "scalable-agent".to_string(),
        model: "llama-3".to_string(),
        capabilities: vec!["chat".to_string()],
        resources: None,
        environment: std::collections::HashMap::new(),
        config: std::collections::HashMap::new(),
    };

    // Deploy
    backend.deploy_agent(&config).await.unwrap();

    // Scale up
    backend.scale_agent("scalable-agent", 5).await.unwrap();

    // Check scaling
    let status = backend.get_agent_status("scalable-agent").await.unwrap();
    assert_eq!(status, AgentStatus::Running);
}

#[tokio::test]
async fn test_inmemory_agent_backend_stop_agent() {
    let backend = InMemoryAgentBackend::new();
    let config = AgentConfig {
        name: "stoppable-agent".to_string(),
        model: "claude-3".to_string(),
        capabilities: vec!["coding".to_string()],
        resources: None,
        environment: std::collections::HashMap::new(),
        config: std::collections::HashMap::new(),
    };

    // Deploy
    backend.deploy_agent(&config).await.unwrap();

    // Stop
    backend.stop_agent("stoppable-agent").await.unwrap();

    // Check status
    let status = backend.get_agent_status("stoppable-agent").await.unwrap();
    assert_eq!(status, AgentStatus::Stopped);
}

#[tokio::test]
async fn test_inmemory_agent_backend_remove_agent() {
    let backend = InMemoryAgentBackend::new();
    let config = AgentConfig {
        name: "removable-agent".to_string(),
        model: "mistral".to_string(),
        capabilities: vec!["chat".to_string()],
        resources: None,
        environment: std::collections::HashMap::new(),
        config: std::collections::HashMap::new(),
    };

    // Deploy
    backend.deploy_agent(&config).await.unwrap();

    // Remove
    backend.remove_agent("removable-agent").await.unwrap();

    // Verify removed
    let result = backend.get_agent_status("removable-agent").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_inmemory_agent_backend_list_empty() {
    let backend = InMemoryAgentBackend::new();
    let agents = backend.list_agents().await.unwrap();
    assert_eq!(agents.len(), 0);
}

#[tokio::test]
async fn test_inmemory_agent_backend_list_multiple() {
    let backend = InMemoryAgentBackend::new();

    for i in 0..3 {
        let config = AgentConfig {
            name: format!("agent-{}", i),
            model: "test-model".to_string(),
            capabilities: vec!["chat".to_string()],
            resources: None,
            environment: std::collections::HashMap::new(),
            config: std::collections::HashMap::new(),
        };
        backend.deploy_agent(&config).await.unwrap();
    }

    let agents = backend.list_agents().await.unwrap();
    assert_eq!(agents.len(), 3);
}

#[tokio::test]
async fn test_inmemory_agent_backend_health_check() {
    let backend = InMemoryAgentBackend::new();
    let result = backend.health_check().await;
    assert!(result.is_ok());
}
