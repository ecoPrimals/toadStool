// SPDX-License-Identifier: AGPL-3.0-or-later
//! Coverage tests for `byob_routes.rs` — route handlers with a mock executor.
//!
//! These tests run the app with [`axum::serve`] on a local listener and assert via minimal HTTP/1.1.

use axum::Router;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::SystemTime;
use toadstool::byob::{
    ByobDeploymentRequest, ByobDeploymentResponse, ByobExecutor, DeploymentStatus, NetworkInfo,
    NetworkUsage, ResourceUsage,
};
use toadstool_runtime_container::ByobApi;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
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

fn test_app() -> Router {
    Router::new()
        .merge(ByobApi::<MockByobExecutor>::routes())
        .with_state(Arc::new(MockByobExecutor))
}

async fn spawn_test_server(app: Router) -> (SocketAddr, JoinHandle<Result<(), std::io::Error>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let make_svc = app.into_make_service();
    let server = tokio::spawn(async move { axum::serve(listener, make_svc).await });
    (addr, server)
}

async fn read_http_status(addr: SocketAddr, request: &[u8]) -> u16 {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    stream.write_all(request).await.unwrap();
    let mut buf = vec![0u8; 2048];
    let n = stream.read(&mut buf).await.unwrap();
    let head = String::from_utf8_lossy(&buf[..n]);
    head.split_whitespace()
        .nth(1)
        .expect("status line")
        .parse()
        .expect("status code")
}

#[tokio::test]
async fn health_check_returns_200() {
    let (addr, server) = spawn_test_server(test_app()).await;
    let req = b"GET /byob/health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    let status = read_http_status(addr, req).await;
    server.abort();
    assert_eq!(status, 200);
}

#[tokio::test]
async fn list_deployments_returns_200() {
    let (addr, server) = spawn_test_server(test_app()).await;
    let req = b"GET /byob/deployments HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    let status = read_http_status(addr, req).await;
    server.abort();
    assert_eq!(status, 200);
}

#[tokio::test]
async fn deploy_biome_returns_200() {
    let (addr, server) = spawn_test_server(test_app()).await;
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
    let payload = serde_json::to_vec(&body).unwrap();
    let mut req = format!(
        "POST /byob/deploy HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        payload.len()
    )
    .into_bytes();
    req.extend_from_slice(&payload);

    let status = read_http_status(addr, &req).await;
    server.abort();
    assert_eq!(status, 200);
}

#[tokio::test]
async fn get_deployment_status_returns_200() {
    let (addr, server) = spawn_test_server(test_app()).await;
    let id = Uuid::new_v4();
    let path = format!("/byob/deployments/{id}");
    let req = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    let status = read_http_status(addr, req.as_bytes()).await;
    server.abort();
    assert_eq!(status, 200);
}

#[tokio::test]
async fn stop_deployment_returns_200() {
    let (addr, server) = spawn_test_server(test_app()).await;
    let id = Uuid::new_v4();
    let path = format!("/byob/deployments/{id}/stop");
    let req = format!(
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    );
    let status = read_http_status(addr, req.as_bytes()).await;
    server.abort();
    assert_eq!(status, 200);
}

#[tokio::test]
async fn get_resource_usage_returns_200() {
    let (addr, server) = spawn_test_server(test_app()).await;
    let id = Uuid::new_v4();
    let path = format!("/byob/deployments/{id}/usage");
    let req = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    let status = read_http_status(addr, req.as_bytes()).await;
    server.abort();
    assert_eq!(status, 200);
}

#[tokio::test]
async fn deploy_biome_invalid_json_returns_error() {
    let (addr, server) = spawn_test_server(test_app()).await;
    let payload = b"not json".as_slice();
    let mut req = format!(
        "POST /byob/deploy HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        payload.len()
    )
    .into_bytes();
    req.extend_from_slice(payload);

    let status = read_http_status(addr, &req).await;
    server.abort();
    assert_ne!(status, 200);
}

#[tokio::test]
async fn unknown_route_returns_404() {
    let (addr, server) = spawn_test_server(test_app()).await;
    let req = b"GET /byob/nonexistent HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    let status = read_http_status(addr, req).await;
    server.abort();
    assert_eq!(status, 404);
}

#[tokio::test]
async fn routes_without_state_can_be_used_by_caller() {
    let executor: Arc<MockByobExecutor> = Arc::new(MockByobExecutor);
    let app = Router::new()
        .merge(ByobApi::<MockByobExecutor>::routes())
        .with_state(executor);
    let (addr, server) = spawn_test_server(app).await;
    let req = b"GET /byob/health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    let status = read_http_status(addr, req).await;
    server.abort();
    assert_eq!(status, 200);
}
