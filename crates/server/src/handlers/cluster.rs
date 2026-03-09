// SPDX-License-Identifier: AGPL-3.0-only
//! Cluster management endpoint handlers

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use tracing::debug;

use crate::state::ServerState;

/// Get cluster status endpoint handler
pub async fn get_cluster_status_handler(State(state): State<ServerState>) -> impl IntoResponse {
    debug!("Cluster status requested");

    let response = json!({
        "cluster_id": "toadstool-cluster",
        "node_id": "toadstool-server",
        "status": "healthy",
        "runtime_engines": state.runtime_engines.read().await.len(),
        "active_executions": state.active_executions.read().await.len(),
        "timestamp": crate::state::timestamp_to_unix_secs(&std::time::SystemTime::now()),
    });

    (StatusCode::OK, Json(response))
}
