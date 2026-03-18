// SPDX-License-Identifier: AGPL-3.0-or-later
//! Config and helper tests

use super::common::{make_availability, make_orchestrator_config, make_requirements};
use crate::cloud::types::{
    CloudDeploymentResult, CloudJobHandle, CloudOrchestratorConfig, DeploymentStrategy,
    DistributionStrategy, FederatedDeployment, MultiCloudDistribution,
};
use crate::cloud::{HybridSchedulingStrategy, UniversalCloudOrchestrator};
use std::time::SystemTime;
use uuid::Uuid;

#[tokio::test]
async fn test_orchestrator_construction() {
    let config = make_orchestrator_config();
    let orch = UniversalCloudOrchestrator::new(config).await;
    assert!(orch.is_ok());
}

#[tokio::test]
async fn test_orchestrator_config_scheduling_strategies() {
    let config_cost = CloudOrchestratorConfig {
        scheduling_strategy: HybridSchedulingStrategy::CostOptimized,
        ..make_orchestrator_config()
    };
    let orch = UniversalCloudOrchestrator::new(config_cost).await;
    assert!(orch.is_ok());

    let config_perf = CloudOrchestratorConfig {
        scheduling_strategy: HybridSchedulingStrategy::PerformanceOptimized,
        ..make_orchestrator_config()
    };
    let orch2 = UniversalCloudOrchestrator::new(config_perf).await;
    assert!(orch2.is_ok());
}

#[tokio::test]
async fn test_availability_info_make_helper() {
    let avail = make_availability(8.0, 16.0, 100.0);
    assert_eq!(avail.cpu_cores, 8.0);
    assert_eq!(avail.memory_gb, 16.0);
    assert_eq!(avail.storage_gb, 100.0);
    assert_eq!(avail.gpu_count, 0);
}

#[tokio::test]
async fn test_requirements_make_helper() {
    let req = make_requirements(2.0, 4096, 1024);
    assert_eq!(req.cpu.min_cores, 2.0);
    assert_eq!(req.memory.min_bytes, 4096);
    assert_eq!(req.storage.min_bytes, 1024);
}

#[tokio::test]
async fn test_availability_info_fields() {
    let avail = make_availability(4.0, 8.0, 50.0);
    assert_eq!(avail.gpu_count, 0);
    assert!(avail.regions.is_empty());
    assert!(avail.availability_zones.is_empty());
}

#[tokio::test]
async fn test_requirements_memory_conversion() {
    let req = make_requirements(2.0, 16 * 1024 * 1024 * 1024, 100);
    let memory_gb = req.memory.min_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    assert!((memory_gb - 16.0).abs() < 0.01);
}

#[tokio::test]
async fn test_deployment_strategy_variants() {
    let single = DeploymentStrategy::SingleCloud {
        provider_name: "aws".to_string(),
    };
    assert!(matches!(single, DeploymentStrategy::SingleCloud { .. }));

    let multi = DeploymentStrategy::MultiCloud {
        providers: vec!["aws".to_string(), "gcp".to_string()],
        distribution: MultiCloudDistribution {
            providers: vec!["aws".to_string(), "gcp".to_string()],
            strategy: DistributionStrategy::Equal,
        },
    };
    assert!(matches!(multi, DeploymentStrategy::MultiCloud { .. }));

    let fed = DeploymentStrategy::FederatedDeployment {
        federation_nodes: vec!["node-1".to_string()],
    };
    assert!(matches!(
        fed,
        DeploymentStrategy::FederatedDeployment { .. }
    ));

    let fed_deploy = FederatedDeployment {
        federation_id: Uuid::new_v4(),
        nodes: vec![],
        coordination_endpoint: "https://fed.example.com".to_string(),
    };
    assert!(fed_deploy.nodes.is_empty());
}

#[tokio::test]
async fn test_cloud_deployment_result_variants() {
    let single = CloudDeploymentResult::Single {
        provider: "aws".to_string(),
        handle: CloudJobHandle {
            job_id: Uuid::new_v4(),
            provider_job_id: "pj-1".to_string(),
            provider_name: "aws".to_string(),
            created_at: SystemTime::now(),
        },
    };
    assert!(matches!(single, CloudDeploymentResult::Single { .. }));

    let fed = CloudDeploymentResult::Federated {
        deployment: FederatedDeployment {
            federation_id: Uuid::new_v4(),
            nodes: vec![],
            coordination_endpoint: "https://x".to_string(),
        },
    };
    assert!(matches!(fed, CloudDeploymentResult::Federated { .. }));
}
