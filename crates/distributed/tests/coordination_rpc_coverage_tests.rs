// SPDX-License-Identifier: AGPL-3.0-or-later
//! Coverage tests for coordination RPC client: creation, request serialization, error paths

#![allow(clippy::pedantic)]

use std::collections::HashMap;
use std::time::Duration;

use toadstool_common::primal_identity::ServiceEndpoint;
use toadstool_common::service_discovery::DiscoveredService;

use toadstool_distributed::coordination_integration::types::{
    CoordinationOperation, LoadBalancingStrategy, NodeStatus,
};
use toadstool_distributed::coordination_integration::{
    CoordinationClient, CoordinationRequest, CoordinationResponse, HealthCheckRequest,
    LoadBalancingRequest, NodeInfo, ServiceRegistration,
};

fn make_discovered_service_with_endpoint() -> DiscoveredService {
    DiscoveredService {
        id: "test-svc".to_string(),
        name: "test-coordination".to_string(),
        version: "1.0".to_string(),
        capabilities: vec![],
        endpoints: vec![ServiceEndpoint::http("localhost", 8080)],
        metadata: HashMap::new(),
        discovered_at: std::time::SystemTime::now(),
        last_seen: std::time::SystemTime::now(),
        healthy: true,
    }
}

fn make_discovered_service_empty_endpoints() -> DiscoveredService {
    DiscoveredService {
        id: "empty".to_string(),
        name: "empty-svc".to_string(),
        version: "1.0".to_string(),
        capabilities: vec![],
        endpoints: vec![],
        metadata: HashMap::new(),
        discovered_at: std::time::SystemTime::now(),
        last_seen: std::time::SystemTime::now(),
        healthy: true,
    }
}

#[test]
fn test_rpc_service_registration_serialization() {
    let reg = ServiceRegistration {
        service_id: "rpc-svc".to_string(),
        service_name: "RPC Test".to_string(),
        version: "1.0".to_string(),
        capabilities: vec!["compute".to_string()],
        endpoints: vec![],
        metadata: HashMap::new(),
        ttl_seconds: 30,
    };
    let json = serde_json::to_value(&reg);
    assert!(json.is_ok());
}

#[test]
fn test_rpc_health_check_request_serialization() {
    let mut metrics = HashMap::new();
    metrics.insert("cpu".to_string(), 0.75);
    let req = HealthCheckRequest {
        service_id: "hc-svc".to_string(),
        healthy: true,
        timestamp: 12345,
        metrics,
        message: Some("ok".to_string()),
    };
    let json = serde_json::to_value(&req);
    assert!(json.is_ok());
}

#[test]
fn test_rpc_load_balancing_request_serialization() {
    let req = LoadBalancingRequest {
        capability: "storage".to_string(),
        requested_capacity: Some(4),
        strategy: LoadBalancingStrategy::RoundRobin,
        metadata: HashMap::new(),
    };
    let json = serde_json::to_value(&req);
    assert!(json.is_ok());
}

#[test]
fn test_rpc_discover_services_params() {
    let params = serde_json::json!({"capability": "compute"});
    assert_eq!(params["capability"], "compute");
}

#[test]
fn test_rpc_coordination_response_deserialization() {
    let json = serde_json::json!({
        "request_id": "00000000-0000-0000-0000-000000000001",
        "success": true,
        "data": {},
        "metadata": {}
    });
    let parsed: Result<CoordinationResponse, _> = serde_json::from_value(json);
    assert!(parsed.is_ok());
}

#[test]
fn test_rpc_node_info_deserialization() {
    let json = serde_json::json!({
        "node_id": "n1",
        "address": "127.0.0.1:8080",
        "capabilities": ["compute"],
        "status": "Healthy",
        "metadata": {},
        "last_health_check": 12345,
        "response_time_ms": 10
    });
    let parsed: Result<NodeInfo, _> = serde_json::from_value(json.clone());
    assert!(parsed.is_ok());
    let nodes: Vec<NodeInfo> = serde_json::from_value(serde_json::json!([json])).unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].node_id, "n1");
}

#[test]
fn test_rpc_load_balancing_strategy_variants() {
    let strategies = [
        LoadBalancingStrategy::LeastConnections,
        LoadBalancingStrategy::LeastResponseTime,
        LoadBalancingStrategy::Random,
        LoadBalancingStrategy::WeightedRoundRobin {
            weights: vec![1, 2, 3],
        },
        LoadBalancingStrategy::ConsistentHash {
            key: "hash-key".to_string(),
        },
        LoadBalancingStrategy::Custom("custom".to_string()),
    ];
    for strategy in strategies {
        let json = serde_json::to_value(&strategy);
        assert!(json.is_ok());
    }
}

#[test]
fn test_rpc_node_status_variants() {
    let _ = NodeStatus::Healthy;
    let _ = NodeStatus::Degraded;
    let _ = NodeStatus::Unhealthy;
    let _ = NodeStatus::Unknown;
}

#[test]
fn test_rpc_coordination_operation_serialization() {
    let req = CoordinationRequest {
        request_id: uuid::Uuid::new_v4(),
        operation: CoordinationOperation::DiscoverServices {
            capability: "compute".to_string(),
        },
        metadata: serde_json::json!({}),
    };
    let json = serde_json::to_value(&req);
    assert!(json.is_ok());
}

#[tokio::test]
async fn test_rpc_client_new_empty_endpoints_fails() {
    let service = make_discovered_service_empty_endpoints();
    let result = CoordinationClient::new(&service).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("No endpoints") || err.to_string().contains("empty-svc"),
        "Error: {}",
        err
    );
}

#[tokio::test]
#[ignore = "Requires coordination socket discovery - may block"]
async fn test_rpc_client_new_with_endpoint_succeeds() {
    let service = make_discovered_service_with_endpoint();
    let result = CoordinationClient::new(&service).await;
    assert!(result.is_ok());
}

#[tokio::test]
#[ignore = "Requires coordination socket discovery - may block"]
async fn test_rpc_client_with_timeout() {
    let service = make_discovered_service_with_endpoint();
    let timeout = Duration::from_secs(5);
    let result = CoordinationClient::with_timeout(&service, timeout).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_rpc_register_service_unavailable() {
    let nonexistent = std::path::PathBuf::from("/tmp/nonexistent-coordination-rpc-12345.sock");
    let client = CoordinationClient::new_for_testing(nonexistent, Duration::from_millis(100));
    let reg = ServiceRegistration {
        service_id: "test-svc".to_string(),
        service_name: "Test".to_string(),
        version: "1.0".to_string(),
        capabilities: vec!["compute".to_string()],
        endpoints: vec![],
        metadata: HashMap::new(),
        ttl_seconds: 30,
    };
    let result = client.register_service(reg).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_rpc_discover_services_unavailable() {
    let nonexistent = std::path::PathBuf::from("/tmp/nonexistent-coordination-discover-rpc.sock");
    let client = CoordinationClient::new_for_testing(nonexistent, Duration::from_millis(100));
    let result = client.discover_services("compute").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_rpc_health_check_unavailable() {
    let nonexistent = std::path::PathBuf::from("/tmp/nonexistent-coordination-health-rpc.sock");
    let client = CoordinationClient::new_for_testing(nonexistent, Duration::from_millis(100));
    let result = client.health_check().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_rpc_report_health_unavailable() {
    let nonexistent = std::path::PathBuf::from("/tmp/nonexistent-coordination-report-rpc.sock");
    let client = CoordinationClient::new_for_testing(nonexistent, Duration::from_millis(100));
    let req = HealthCheckRequest {
        service_id: "svc".to_string(),
        healthy: true,
        timestamp: 12345,
        metrics: HashMap::new(),
        message: Some("ok".to_string()),
    };
    let result = client.report_health(req).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_rpc_get_load_balancing_unavailable() {
    let nonexistent = std::path::PathBuf::from("/tmp/nonexistent-coordination-lb-rpc.sock");
    let client = CoordinationClient::new_for_testing(nonexistent, Duration::from_millis(100));
    let req = LoadBalancingRequest {
        capability: "storage".to_string(),
        requested_capacity: Some(4),
        strategy: LoadBalancingStrategy::RoundRobin,
        metadata: HashMap::new(),
    };
    let result = client.get_load_balancing(req).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_rpc_execute_unavailable() {
    let nonexistent = std::path::PathBuf::from("/tmp/nonexistent-coordination-execute-rpc.sock");
    let client = CoordinationClient::new_for_testing(nonexistent, Duration::from_millis(100));
    let req = CoordinationRequest {
        request_id: uuid::Uuid::new_v4(),
        operation: CoordinationOperation::DiscoverServices {
            capability: "compute".to_string(),
        },
        metadata: serde_json::json!({}),
    };
    let result = client.execute(req).await;
    assert!(result.is_err());
}
