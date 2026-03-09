// SPDX-License-Identifier: AGPL-3.0-only
//! ByobExecutor trait integration tests

use super::super::*;
use super::common::*;
use std::time::Duration;
use uuid::Uuid;

#[tokio::test]
async fn test_deploy_biome_success() {
    let engine = create_test_runtime_engine();
    let config = create_test_config(8080, vec![80]);
    let executor = ByobComputeExecutor::new(engine, config);
    let request = create_test_deployment_request();
    let response = executor.deploy_biome(request.clone()).await.unwrap();
    assert_eq!(response.deployment_id, request.deployment_id);
    assert!(matches!(response.status, DeploymentStatus::Running));
}

#[tokio::test]
async fn test_deploy_biome_validation_fails() {
    let engine = create_test_runtime_engine();
    let executor = ByobComputeExecutor::new(engine, ByobExecutorConfig::default());
    let mut bad_request = create_test_deployment_request();
    bad_request.services.clear();
    let result = executor.deploy_biome(bad_request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_deployment_status_found() {
    let engine = create_test_runtime_engine();
    let executor = ByobComputeExecutor::new(engine, create_test_config(8080, vec![80]));
    let request = create_test_deployment_request();
    executor.deploy_biome(request.clone()).await.unwrap();
    let status = executor
        .get_deployment_status(request.deployment_id)
        .await
        .unwrap();
    assert_eq!(status.deployment_id, request.deployment_id);
}

#[tokio::test]
async fn test_get_deployment_status_not_found() {
    let engine = create_test_runtime_engine();
    let executor = ByobComputeExecutor::new(engine, ByobExecutorConfig::default());
    let result = executor.get_deployment_status(Uuid::new_v4()).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[tokio::test]
async fn test_stop_deployment_not_found() {
    let engine = create_test_runtime_engine();
    let executor = ByobComputeExecutor::new(engine, ByobExecutorConfig::default());
    let result = executor.stop_deployment(Uuid::new_v4()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_stop_deployment_success() {
    let engine = create_test_runtime_engine();
    let executor = ByobComputeExecutor::new(engine, create_test_config(8080, vec![80]));
    let request = create_test_deployment_request();
    executor.deploy_biome(request.clone()).await.unwrap();
    let result = executor.stop_deployment(request.deployment_id).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_list_deployments_empty() {
    let engine = create_test_runtime_engine();
    let executor = ByobComputeExecutor::new(engine, ByobExecutorConfig::default());
    let list = executor.list_deployments().await.unwrap();
    assert!(list.is_empty());
}

#[tokio::test]
async fn test_list_deployments_with_deployments() {
    let engine = create_test_runtime_engine();
    let executor = ByobComputeExecutor::new(engine, create_test_config(8080, vec![80]));
    let request = create_test_deployment_request();
    executor.deploy_biome(request).await.unwrap();
    let list = executor.list_deployments().await.unwrap();
    assert_eq!(list.len(), 1);
}

#[tokio::test]
async fn test_get_resource_usage_not_found() {
    let engine = create_test_runtime_engine();
    let executor = ByobComputeExecutor::new(engine, ByobExecutorConfig::default());
    let result = executor.get_resource_usage(Uuid::new_v4()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_resource_usage_after_deploy() {
    let engine = create_test_runtime_engine();
    let executor = ByobComputeExecutor::new(engine, create_test_config(8080, vec![80]));
    let request = create_test_deployment_request();
    executor.deploy_biome(request.clone()).await.unwrap();
    let usage = executor
        .get_resource_usage(request.deployment_id)
        .await
        .unwrap();
    assert!(usage.cpu_usage >= 0.0);
}

#[tokio::test]
async fn test_deploy_biome_max_concurrent_limit() {
    let engine = create_test_runtime_engine();
    let config = ByobExecutorConfig {
        max_concurrent_deployments: 1,
        default_network_subnet: "10.0.0.0/24".to_string(),
        resource_monitoring_interval: Duration::from_secs(30),
        health_check_interval: Duration::from_secs(10),
        deployment_timeout: Duration::from_secs(600),
        default_host_port: 8080,
        web_service_ports: vec![80],
        graceful_shutdown_timeout_secs: 30,
    };
    let executor = ByobComputeExecutor::new(engine, config);
    let req1 = create_test_deployment_request();
    executor.deploy_biome(req1).await.unwrap();
    let mut req2 = create_test_deployment_request();
    req2.deployment_id = Uuid::new_v4();
    let result = executor.deploy_biome(req2).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("concurrent"));
}

#[tokio::test]
async fn test_network_info_service_endpoints() {
    let config = create_test_config(8080, vec![80]);
    let executor = ByobComputeExecutor::new(create_test_runtime_engine(), config);
    let request = create_test_deployment_request();
    let network = executor.create_deployment_network(&request);
    for (name, ep) in &network.service_endpoints {
        assert_eq!(ep.name, *name);
        assert!(ep.internal_ip.starts_with("10.0.0."));
        assert!(!ep.ports.is_empty());
    }
}
