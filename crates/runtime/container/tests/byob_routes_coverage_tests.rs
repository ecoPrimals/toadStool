// SPDX-License-Identifier: AGPL-3.0-or-later
//! Coverage tests for BYOB JSON-RPC dispatch — route handlers with a mock executor.
//!
//! These tests validate the JSON-RPC dispatch layer directly (no network),
//! confirming method routing, param parsing, and error mapping.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use toadstool::byob::{
    ByobDeploymentRequest, ByobDeploymentResponse, ByobExecutor, DeploymentStatus, NetworkInfo,
    NetworkUsage, ResourceUsage,
};
use toadstool_runtime_container::ByobApi;
use uuid::Uuid;

struct MockByobExecutor;

fn mock_resource_usage() -> ResourceUsage {
    ResourceUsage {
        cpu_usage: 0.0,
        memory_usage: 0,
        storage_usage: 0,
        gpu_usage: 0,
        network_usage: NetworkUsage {
            bytes_sent: 0,
            bytes_received: 0,
            packets_sent: 0,
            packets_received: 0,
        },
    }
}

fn mock_network_info() -> NetworkInfo {
    NetworkInfo {
        network_name: "test".into(),
        subnet_cidr: "10.0.0.0/24".into(),
        gateway_ip: "10.0.0.1".into(),
        service_endpoints: HashMap::new(),
    }
}

impl ByobExecutor for MockByobExecutor {
    fn deploy_biome(
        &self,
        _request: ByobDeploymentRequest,
    ) -> impl std::future::Future<Output = toadstool::error::ToadStoolResult<ByobDeploymentResponse>>
    + Send
    + '_ {
        async move {
            let now = SystemTime::now();
            Ok(ByobDeploymentResponse {
                deployment_id: Uuid::new_v4(),
                status: DeploymentStatus::Running,
                service_statuses: HashMap::new(),
                resource_usage: mock_resource_usage(),
                network_info: mock_network_info(),
                created_at: now,
                updated_at: now,
            })
        }
    }

    fn list_deployments(
        &self,
    ) -> impl std::future::Future<
        Output = toadstool::error::ToadStoolResult<Vec<ByobDeploymentResponse>>,
    > + Send
    + '_ {
        async move { Ok(vec![]) }
    }

    fn get_deployment_status(
        &self,
        deployment_id: Uuid,
    ) -> impl std::future::Future<Output = toadstool::error::ToadStoolResult<ByobDeploymentResponse>>
    + Send
    + '_ {
        async move {
            let now = SystemTime::now();
            Ok(ByobDeploymentResponse {
                deployment_id,
                status: DeploymentStatus::Running,
                service_statuses: HashMap::new(),
                resource_usage: mock_resource_usage(),
                network_info: mock_network_info(),
                created_at: now,
                updated_at: now,
            })
        }
    }

    fn stop_deployment(
        &self,
        _deployment_id: Uuid,
    ) -> impl std::future::Future<Output = toadstool::error::ToadStoolResult<()>> + Send + '_ {
        async move { Ok(()) }
    }

    fn get_resource_usage(
        &self,
        _deployment_id: Uuid,
    ) -> impl std::future::Future<Output = toadstool::error::ToadStoolResult<ResourceUsage>> + Send + '_
    {
        async move { Ok(mock_resource_usage()) }
    }
}

fn test_api() -> ByobApi<MockByobExecutor> {
    ByobApi::new(Arc::new(MockByobExecutor))
}

fn jsonrpc_request(method: &str, params: Option<serde_json::Value>) -> String {
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": 1,
    });
    serde_json::to_string(&req).unwrap()
}

#[tokio::test]
async fn health_check_returns_success() {
    let api = test_api();
    let req = jsonrpc_request("byob.health", None);
    let resp = api.dispatch(&req).await;
    assert!(resp.result.is_some());
    assert!(resp.error.is_none());
    let result = resp.result.unwrap();
    assert_eq!(result["status"], "healthy");
}

#[tokio::test]
async fn list_deployments_returns_success() {
    let api = test_api();
    let req = jsonrpc_request("byob.list_deployments", None);
    let resp = api.dispatch(&req).await;
    assert!(resp.result.is_some());
    assert!(resp.error.is_none());
}

#[tokio::test]
async fn deploy_biome_returns_success() {
    let api = test_api();
    let params = serde_json::json!({
        "deployment_id": "550e8400-e29b-41d4-a716-446655440000",
        "team_id": "test-team",
        "deployment_name": "test-deploy",
        "services": {},
        "resource_quotas": {
            "max_cpu_cores": 10.0,
            "max_memory_bytes": 10_000_000_000_u64,
            "max_storage_bytes": 50_000_000_000_u64,
            "max_gpu_count": 0,
            "max_concurrent_services": 10
        },
        "security_config": {
            "isolation_level": "high",
            "network_policies": [],
            "volume_policies": [],
            "resource_policies": []
        },
        "network_config": {
            "network_name": "test-net",
            "subnet_cidr": "10.0.1.0/24",
            "dns_config": null,
            "load_balancer": null
        },
        "created_at": 1_704_067_200_u64
    });
    let req = jsonrpc_request("byob.deploy", Some(params));
    let resp = api.dispatch(&req).await;
    assert!(resp.result.is_some(), "expected success, got {:?}", resp.error);
    assert!(resp.error.is_none());
}

#[tokio::test]
async fn get_deployment_status_returns_success() {
    let api = test_api();
    let id = Uuid::new_v4();
    let params = serde_json::json!({ "deployment_id": id.to_string() });
    let req = jsonrpc_request("byob.get_deployment", Some(params));
    let resp = api.dispatch(&req).await;
    assert!(resp.result.is_some());
    assert!(resp.error.is_none());
}

#[tokio::test]
async fn stop_deployment_returns_success() {
    let api = test_api();
    let id = Uuid::new_v4();
    let params = serde_json::json!({ "deployment_id": id.to_string() });
    let req = jsonrpc_request("byob.stop_deployment", Some(params));
    let resp = api.dispatch(&req).await;
    assert!(resp.result.is_some());
    assert!(resp.error.is_none());
}

#[tokio::test]
async fn get_resource_usage_returns_success() {
    let api = test_api();
    let id = Uuid::new_v4();
    let params = serde_json::json!({ "deployment_id": id.to_string() });
    let req = jsonrpc_request("byob.get_resource_usage", Some(params));
    let resp = api.dispatch(&req).await;
    assert!(resp.result.is_some());
    assert!(resp.error.is_none());
}

#[tokio::test]
async fn deploy_biome_invalid_json_returns_error() {
    let api = test_api();
    let resp = api.dispatch("not json at all").await;
    assert!(resp.error.is_some());
    assert_eq!(resp.error.unwrap().code, -32700); // parse error
}

#[tokio::test]
async fn unknown_method_returns_error() {
    let api = test_api();
    let req = jsonrpc_request("byob.nonexistent", None);
    let resp = api.dispatch(&req).await;
    assert!(resp.error.is_some());
    assert_eq!(resp.error.unwrap().code, -32601); // method not found
}

#[tokio::test]
async fn info_returns_version() {
    let api = test_api();
    let req = jsonrpc_request("byob.info", None);
    let resp = api.dispatch(&req).await;
    assert!(resp.result.is_some());
    let result = resp.result.unwrap();
    assert_eq!(result["service"], "toadstool-byob-server");
    assert_eq!(result["transport"], "json-rpc-2.0");
}
