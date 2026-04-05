// SPDX-License-Identifier: AGPL-3.0-or-later
//! Coverage tests for `byob_routes.rs` — route handlers with a mock executor.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;

use async_trait::async_trait;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use toadstool::byob::{
    ByobDeploymentRequest, ByobDeploymentResponse, ByobExecutor, DeploymentStatus, NetworkInfo,
    NetworkUsage, ResourceUsage,
};
use toadstool_runtime_container::ByobApi;
use tower::ServiceExt;
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

#[async_trait]
impl ByobExecutor for MockByobExecutor {
    async fn deploy_biome(
        &self,
        _request: ByobDeploymentRequest,
    ) -> toadstool::error::ToadStoolResult<ByobDeploymentResponse> {
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

    async fn list_deployments(
        &self,
    ) -> toadstool::error::ToadStoolResult<Vec<ByobDeploymentResponse>> {
        Ok(vec![])
    }

    async fn get_deployment_status(
        &self,
        deployment_id: Uuid,
    ) -> toadstool::error::ToadStoolResult<ByobDeploymentResponse> {
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

    async fn stop_deployment(&self, _deployment_id: Uuid) -> toadstool::error::ToadStoolResult<()> {
        Ok(())
    }

    async fn get_resource_usage(
        &self,
        _deployment_id: Uuid,
    ) -> toadstool::error::ToadStoolResult<ResourceUsage> {
        Ok(mock_resource_usage())
    }
}

fn app() -> axum::Router {
    let executor: Arc<dyn ByobExecutor> = Arc::new(MockByobExecutor);
    ByobApi::new(executor).router()
}

#[tokio::test]
async fn health_check_returns_200() {
    let req = Request::builder()
        .uri("/byob/health")
        .body(Body::empty())
        .unwrap();
    let resp = app().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn list_deployments_returns_200() {
    let req = Request::builder()
        .uri("/byob/deployments")
        .body(Body::empty())
        .unwrap();
    let resp = app().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn deploy_biome_returns_200() {
    let body = serde_json::json!({
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
    let req = Request::builder()
        .method("POST")
        .uri("/byob/deploy")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let resp = app().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn get_deployment_status_returns_200() {
    let id = Uuid::new_v4();
    let req = Request::builder()
        .uri(format!("/byob/deployments/{id}"))
        .body(Body::empty())
        .unwrap();
    let resp = app().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn stop_deployment_returns_200() {
    let id = Uuid::new_v4();
    let req = Request::builder()
        .method("POST")
        .uri(format!("/byob/deployments/{id}/stop"))
        .body(Body::empty())
        .unwrap();
    let resp = app().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn get_resource_usage_returns_200() {
    let id = Uuid::new_v4();
    let req = Request::builder()
        .uri(format!("/byob/deployments/{id}/usage"))
        .body(Body::empty())
        .unwrap();
    let resp = app().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn deploy_biome_invalid_json_returns_error() {
    let req = Request::builder()
        .method("POST")
        .uri("/byob/deploy")
        .header("content-type", "application/json")
        .body(Body::from(b"not json".to_vec()))
        .unwrap();
    let resp = app().oneshot(req).await.unwrap();
    assert_ne!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn unknown_route_returns_404() {
    let req = Request::builder()
        .uri("/byob/nonexistent")
        .body(Body::empty())
        .unwrap();
    let resp = app().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn routes_without_state_can_be_used_by_caller() {
    let executor: Arc<dyn ByobExecutor> = Arc::new(MockByobExecutor);
    let router = ByobApi::routes().with_state(executor);
    let req = Request::builder()
        .uri("/byob/health")
        .body(Body::empty())
        .unwrap();
    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
