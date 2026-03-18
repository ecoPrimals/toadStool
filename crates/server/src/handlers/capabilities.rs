// SPDX-License-Identifier: AGPL-3.0-or-later
//! Capability and runtime engine endpoint handlers

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde_json::json;
use tracing::debug;

use crate::state::ServerState;

/// List runtime engines endpoint handler
pub async fn list_runtime_engines_handler(State(state): State<ServerState>) -> impl IntoResponse {
    debug!("Runtime engines list requested");

    let runtime_engines = state.runtime_engines.read().await;
    let engines: Vec<serde_json::Value> = runtime_engines
        .keys()
        .map(|runtime_type| {
            json!({
                "runtime_type": runtime_type,
                "status": "active",
            })
        })
        .collect();

    let response = json!({
        "runtime_engines": engines,
        "total_count": engines.len(),
        "timestamp": crate::state::timestamp_to_unix_secs(&std::time::SystemTime::now()),
    });

    (StatusCode::OK, Json(response))
}
