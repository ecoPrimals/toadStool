//! Cluster status and management handlers

use std::time::SystemTime;

use axum::{extract::State, response::IntoResponse, Json};
use tracing::debug;

use crate::constants::DEFAULT_RUNTIME_TYPE;
use crate::types::{
    ApiError, ClusterCapacity, ClusterNodeInfo, ClusterStatusResponse, ExecutionStatus, NodeStatus,
};
use crate::ApiState;

use super::helpers::get_local_node_resources;

/// Get cluster status
#[utoipa::path(
    get,
    path = "/api/v2/cluster/status",
    responses(
        (status = 200, description = "Cluster status", body = ClusterStatusResponse),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "cluster"
)]
pub async fn get_cluster_status(
    State(state): State<ApiState>,
) -> Result<impl IntoResponse, ApiError> {
    debug!("Getting cluster status");

    let executions = state.executions.read().await;
    let active_executions = executions
        .values()
        .filter(|exec| {
            matches!(
                exec.status,
                ExecutionStatus::Running | ExecutionStatus::Queued
            )
        })
        .count() as u32;
    let queued_executions = executions
        .values()
        .filter(|exec| matches!(exec.status, ExecutionStatus::Queued))
        .count() as u32;

    // Collect cluster information
    let node_details = {
        let mut nodes = Vec::new();

        // Get information about the local node
        let config = toadstool_config::env_config::EnvironmentConfig::from_env();
        let local_node = ClusterNodeInfo {
            id: format!("local-node-{}", std::process::id()),
            address: config.network.bind_address.clone(),
            status: NodeStatus::Healthy,
            capabilities: vec![
                DEFAULT_RUNTIME_TYPE.to_string(),
                "container".to_string(),
                "wasm".to_string(),
                "python".to_string(),
            ],
            resources: get_local_node_resources().await,
        };
        nodes.push(local_node);

        nodes
    };

    let total_capacity = ClusterCapacity {
        cpu_cores: node_details.iter().map(|n| n.resources.cpu_cores).sum(),
        memory_gb: node_details.iter().map(|n| n.resources.memory_gb).sum(),
        storage_gb: node_details.iter().map(|n| n.resources.storage_gb).sum(),
        gpu_count: node_details.iter().map(|n| n.resources.gpu_count).sum(),
    };

    // Calculate current utilization
    let current_utilization = {
        let active_count = active_executions + queued_executions;
        let base_utilization = (f64::from(active_count) / 100.0).min(1.0);
        ClusterCapacity {
            cpu_cores: (base_utilization * 100.0) as u32,
            memory_gb: (base_utilization * 80.0) as u32,
            storage_gb: (base_utilization * 30.0) as u32,
            gpu_count: 0,
        }
    };

    let response = ClusterStatusResponse {
        cluster_id: "toadstool-cluster-1".to_string(),
        total_nodes: node_details.len() as u32,
        healthy_nodes: node_details
            .iter()
            .filter(|n| n.status == NodeStatus::Healthy)
            .count() as u32,
        cluster_load: 45.5,
        active_executions,
        queued_executions,
        total_capacity,
        used_capacity: current_utilization,
        node_details,
        last_updated: SystemTime::now(),
    };

    Ok(Json(response))
}
