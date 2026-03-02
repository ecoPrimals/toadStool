//! Comprehensive tests for UniversalPrimalRegistry
//!
//! Focus: Pure logic testable without external I/O.
//! Covers creation, registration, capability/context lookup, routing, and error paths.

use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use async_trait::async_trait;

use toadstool::universal::{
    NetworkLocation, PrimalCapability, PrimalContext, PrimalEndpoints, PrimalHealth, PrimalRequest,
    PrimalResponse, PrimalType, ResponseStatus, SecurityLevel, UniversalPrimalRegistry,
};

// ============================================================================
// Mock Primal Provider for testing (no I/O, no env)
// ============================================================================

struct MockPrimalProvider {
    instance_id: String,
    primal_type: PrimalType,
    context: PrimalContext,
    capabilities: Vec<PrimalCapability>,
    can_serve_any_context: bool,
    fail_requests: bool,
}

impl MockPrimalProvider {
    fn new(
        id: &str,
        primal_type: PrimalType,
        context: PrimalContext,
        capabilities: Vec<PrimalCapability>,
    ) -> Self {
        Self {
            instance_id: id.to_string(),
            primal_type,
            context,
            capabilities,
            can_serve_any_context: true,
            fail_requests: false,
        }
    }

    fn with_can_serve_context(mut self, can_serve: bool) -> Self {
        self.can_serve_any_context = can_serve;
        self
    }

    fn with_fail_requests(mut self, fail: bool) -> Self {
        self.fail_requests = fail;
        self
    }
}

// TODO(afit): Migrate when trait_variant stabilizes (used as dyn)
#[async_trait]
impl toadstool::universal::UniversalPrimalProvider for MockPrimalProvider {
    fn primal_id(&self) -> &str {
        &self.instance_id
    }

    fn instance_id(&self) -> &str {
        &self.instance_id
    }

    fn context(&self) -> &PrimalContext {
        &self.context
    }

    fn primal_type(&self) -> PrimalType {
        self.primal_type.clone()
    }

    fn capabilities(&self) -> Vec<PrimalCapability> {
        self.capabilities.clone()
    }

    async fn health_check(&self) -> PrimalHealth {
        PrimalHealth::Healthy
    }

    fn endpoints(&self) -> PrimalEndpoints {
        PrimalEndpoints {
            primary: format!("http://{}/api", self.instance_id),
            health: format!("http://{}/health", self.instance_id),
            metrics: Some(format!("http://{}/metrics", self.instance_id)),
            admin: None,
            events_endpoint: None,
            custom: HashMap::new(),
        }
    }

    async fn handle_primal_request(
        &self,
        request: PrimalRequest,
    ) -> toadstool::ToadStoolResult<PrimalResponse> {
        if self.fail_requests {
            return Err(toadstool::ToadStoolError::execution("mock failure"));
        }
        Ok(PrimalResponse {
            request_id: request.id,
            status: ResponseStatus::Success,
            payload: serde_json::json!({"handled_by": self.instance_id}),
            metadata: HashMap::new(),
            timestamp: std::time::SystemTime::now(),
        })
    }

    async fn initialize(&mut self, _config: serde_json::Value) -> toadstool::ToadStoolResult<()> {
        Ok(())
    }

    async fn shutdown(&mut self) -> toadstool::ToadStoolResult<()> {
        Ok(())
    }

    fn can_serve_context(&self, context: &PrimalContext) -> bool {
        if self.can_serve_any_context {
            return true;
        }
        context.user_id == self.context.user_id
    }
}

fn create_test_context(user_id: &str) -> PrimalContext {
    PrimalContext {
        user_id: user_id.to_string(),
        device_id: "test-device".to_string(),
        session_id: Uuid::new_v4().to_string(),
        network_location: NetworkLocation {
            ip_address: "127.0.0.1".to_string(),
            subnet: Some("192.168.1.0/24".to_string()),
            network_id: None,
            geo_location: None,
        },
        security_level: SecurityLevel::Standard,
        metadata: HashMap::new(),
    }
}

// ============================================================================
// Registry Creation and Defaults
// ============================================================================

#[test]
fn test_registry_new() {
    let _registry = UniversalPrimalRegistry::new();
}

#[test]
fn test_registry_default() {
    let registry = UniversalPrimalRegistry::default();
    // Default should be equivalent to new
    let _ = registry;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_registry_empty_get_all_providers() {
    let registry = UniversalPrimalRegistry::new();
    let providers = registry.get_all_providers().await;
    assert!(
        providers.is_empty(),
        "New registry should have no providers"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_registry_empty_find_by_capability() {
    let registry = UniversalPrimalRegistry::new();
    let capability = PrimalCapability::NativeExecution {
        architectures: vec!["x86_64".to_string()],
    };
    let providers = registry.find_by_capability(&capability).await;
    assert!(providers.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_registry_empty_find_by_context() {
    let registry = UniversalPrimalRegistry::new();
    let context = create_test_context("user-1");
    let providers = registry.find_by_context(&context).await;
    assert!(providers.is_empty());
}

// ============================================================================
// Registration
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_registry_register_single_provider() {
    let registry = UniversalPrimalRegistry::new();
    let provider = Arc::new(MockPrimalProvider::new(
        "primal-1",
        PrimalType::Compute,
        create_test_context("user-1"),
        vec![PrimalCapability::NativeExecution {
            architectures: vec!["x86_64".to_string()],
        }],
    ));

    let result = registry.register_primal(provider).await;
    assert!(result.is_ok());

    let providers = registry.get_all_providers().await;
    assert_eq!(providers.len(), 1);
    assert_eq!(providers[0].instance_id(), "primal-1");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_registry_register_multiple_providers() {
    let registry = UniversalPrimalRegistry::new();

    for i in 1..=5 {
        let provider = Arc::new(MockPrimalProvider::new(
            &format!("primal-{i}"),
            PrimalType::Compute,
            create_test_context(&format!("user-{i}")),
            vec![PrimalCapability::NativeExecution {
                architectures: vec!["x86_64".to_string()],
            }],
        ));
        registry.register_primal(provider).await.unwrap();
    }

    let providers = registry.get_all_providers().await;
    assert_eq!(providers.len(), 5);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_registry_register_overwrites_same_instance_id() {
    let registry = UniversalPrimalRegistry::new();

    let provider1 = Arc::new(MockPrimalProvider::new(
        "same-id",
        PrimalType::Compute,
        create_test_context("user-1"),
        vec![PrimalCapability::NativeExecution {
            architectures: vec!["x86_64".to_string()],
        }],
    ));
    registry.register_primal(provider1).await.unwrap();

    let provider2 = Arc::new(MockPrimalProvider::new(
        "same-id",
        PrimalType::Security,
        create_test_context("user-2"),
        vec![PrimalCapability::Authentication {
            methods: vec!["oauth".to_string()],
        }],
    ));
    registry.register_primal(provider2).await.unwrap();

    let providers = registry.get_all_providers().await;
    assert_eq!(providers.len(), 1);
    assert_eq!(providers[0].primal_type(), PrimalType::Security);
}

// ============================================================================
// Capability Lookup
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_registry_find_by_capability_native_execution() {
    let registry = UniversalPrimalRegistry::new();
    let capability = PrimalCapability::NativeExecution {
        architectures: vec!["x86_64".to_string(), "aarch64".to_string()],
    };
    let provider = Arc::new(MockPrimalProvider::new(
        "native-1",
        PrimalType::Compute,
        create_test_context("user-1"),
        vec![capability.clone()],
    ));
    registry.register_primal(provider).await.unwrap();

    let found = registry.find_by_capability(&capability).await;
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].instance_id(), "native-1");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_registry_find_by_capability_wasm() {
    let registry = UniversalPrimalRegistry::new();
    let capability = PrimalCapability::WasmExecution { wasi_support: true };
    let provider = Arc::new(MockPrimalProvider::new(
        "wasm-1",
        PrimalType::Compute,
        create_test_context("user-1"),
        vec![capability.clone()],
    ));
    registry.register_primal(provider).await.unwrap();

    let found = registry.find_by_capability(&capability).await;
    assert_eq!(found.len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_registry_find_by_capability_multiple_with_same_cap() {
    let registry = UniversalPrimalRegistry::new();
    let capability = PrimalCapability::ContainerRuntime {
        orchestrators: vec!["docker".to_string()],
    };

    let p1 = Arc::new(MockPrimalProvider::new(
        "container-1",
        PrimalType::Compute,
        create_test_context("user-1"),
        vec![capability.clone()],
    ));
    let p2 = Arc::new(MockPrimalProvider::new(
        "container-2",
        PrimalType::Compute,
        create_test_context("user-2"),
        vec![capability.clone()],
    ));
    registry.register_primal(p1).await.unwrap();
    registry.register_primal(p2).await.unwrap();

    let found = registry.find_by_capability(&capability).await;
    assert_eq!(found.len(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_registry_find_by_capability_no_match() {
    let registry = UniversalPrimalRegistry::new();
    let provider = Arc::new(MockPrimalProvider::new(
        "primal-1",
        PrimalType::Compute,
        create_test_context("user-1"),
        vec![PrimalCapability::NativeExecution {
            architectures: vec!["x86_64".to_string()],
        }],
    ));
    registry.register_primal(provider).await.unwrap();

    let other_capability = PrimalCapability::GpuAcceleration { cuda_support: true };
    let found = registry.find_by_capability(&other_capability).await;
    assert!(found.is_empty());
}

// ============================================================================
// Context Lookup
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_registry_find_by_context() {
    let registry = UniversalPrimalRegistry::new();
    let context = create_test_context("alice");
    let provider = Arc::new(MockPrimalProvider::new(
        "alice-primal",
        PrimalType::Compute,
        context.clone(),
        vec![PrimalCapability::NativeExecution {
            architectures: vec!["x86_64".to_string()],
        }],
    ));
    registry.register_primal(provider).await.unwrap();

    let found = registry.find_by_context(&context).await;
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].instance_id(), "alice-primal");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_registry_find_by_context_filters_can_serve() {
    let registry = UniversalPrimalRegistry::new();
    let provider = Arc::new(
        MockPrimalProvider::new(
            "restricted-primal",
            PrimalType::Compute,
            create_test_context("bob"),
            vec![PrimalCapability::NativeExecution {
                architectures: vec!["x86_64".to_string()],
            }],
        )
        .with_can_serve_context(false),
    );
    registry.register_primal(provider).await.unwrap();

    // Query with different user - can_serve_context returns false
    let alice_context = create_test_context("alice");
    let found = registry.find_by_context(&alice_context).await;
    assert!(found.is_empty());

    // Query with same user - can_serve_context checks user_id match
    let bob_context = create_test_context("bob");
    let found = registry.find_by_context(&bob_context).await;
    assert_eq!(found.len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_registry_find_by_context_unknown_user_empty() {
    let registry = UniversalPrimalRegistry::new();
    let provider = Arc::new(MockPrimalProvider::new(
        "primal-1",
        PrimalType::Compute,
        create_test_context("user-1"),
        vec![],
    ));
    registry.register_primal(provider).await.unwrap();

    let unknown_context = create_test_context("unknown-user-999");
    let found = registry.find_by_context(&unknown_context).await;
    // Context index uses user_id - unknown user has no index entry
    assert!(found.is_empty());
}

// ============================================================================
// Request Routing
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_registry_route_request_success() {
    let registry = UniversalPrimalRegistry::new();
    let provider = Arc::new(MockPrimalProvider::new(
        "target-primal",
        PrimalType::Compute,
        create_test_context("user-1"),
        vec![],
    ));
    registry.register_primal(provider).await.unwrap();

    let request = PrimalRequest {
        id: Uuid::new_v4(),
        source: "source".to_string(),
        target: "target-primal".to_string(),
        request_type: "test".to_string(),
        payload: serde_json::json!({}),
        context: create_test_context("user-1"),
        metadata: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
    };

    let result = registry.route_request(request).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(matches!(response.status, ResponseStatus::Success));
    assert_eq!(
        response.payload.get("handled_by").and_then(|v| v.as_str()),
        Some("target-primal")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_registry_route_request_target_not_found() {
    let registry = UniversalPrimalRegistry::new();
    let provider = Arc::new(MockPrimalProvider::new(
        "registered-primal",
        PrimalType::Compute,
        create_test_context("user-1"),
        vec![],
    ));
    registry.register_primal(provider).await.unwrap();

    let request = PrimalRequest {
        id: Uuid::new_v4(),
        source: "source".to_string(),
        target: "non-existent-primal".to_string(),
        request_type: "test".to_string(),
        payload: serde_json::json!({}),
        context: create_test_context("user-1"),
        metadata: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
    };

    let result = registry.route_request(request).await;
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_registry_route_request_provider_returns_error() {
    let registry = UniversalPrimalRegistry::new();
    let provider = Arc::new(
        MockPrimalProvider::new(
            "failing-primal",
            PrimalType::Compute,
            create_test_context("user-1"),
            vec![],
        )
        .with_fail_requests(true),
    );
    registry.register_primal(provider).await.unwrap();

    let request = PrimalRequest {
        id: Uuid::new_v4(),
        source: "source".to_string(),
        target: "failing-primal".to_string(),
        request_type: "test".to_string(),
        payload: serde_json::json!({}),
        context: create_test_context("user-1"),
        metadata: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
    };

    let result = registry.route_request(request).await;
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_registry_route_request_empty_registry() {
    let registry = UniversalPrimalRegistry::new();

    let request = PrimalRequest {
        id: Uuid::new_v4(),
        source: "source".to_string(),
        target: "any-target".to_string(),
        request_type: "test".to_string(),
        payload: serde_json::json!({}),
        context: create_test_context("user-1"),
        metadata: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
    };

    let result = registry.route_request(request).await;
    assert!(result.is_err());
}

// ============================================================================
// Type Index
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_registry_provider_primal_type_preserved() {
    let registry = UniversalPrimalRegistry::new();
    let provider = Arc::new(MockPrimalProvider::new(
        "storage-1",
        PrimalType::Storage,
        create_test_context("user-1"),
        vec![],
    ));
    registry.register_primal(provider).await.unwrap();

    let providers = registry.get_all_providers().await;
    assert_eq!(providers[0].primal_type(), PrimalType::Storage);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_registry_custom_primal_type() {
    let registry = UniversalPrimalRegistry::new();
    let provider = Arc::new(MockPrimalProvider::new(
        "custom-1",
        PrimalType::Custom("analytics".to_string()),
        create_test_context("user-1"),
        vec![],
    ));
    registry.register_primal(provider).await.unwrap();

    let providers = registry.get_all_providers().await;
    assert_eq!(
        providers[0].primal_type(),
        PrimalType::Custom("analytics".to_string())
    );
}
