//! Workload execution endpoint handlers

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;
use tracing::{debug, info};
use uuid::Uuid;

use crate::state::{ServerEvent, ServerState};
use toadstool_common::constants::timeouts::WORKLOAD_EXECUTION_TIMEOUT;

/// Submit execution endpoint handler
pub async fn submit_execution_handler(
    State(state): State<ServerState>,
    Json(request): Json<serde_json::Value>,
) -> impl IntoResponse {
    debug!("Execution submission requested: {:?}", request);

    let execution_id = Uuid::new_v4();
    let runtime_type = request.get("runtime_type").and_then(|v| v.as_str()).map_or(
        toadstool::RuntimeType::Native,
        |s| match s {
            "container" => toadstool::RuntimeType::Container,
            "wasm" => toadstool::RuntimeType::Wasm,
            "python" => toadstool::RuntimeType::Python,
            _ => toadstool::RuntimeType::Native,
        },
    );

    let execution_info = crate::state::ActiveExecution {
        execution_id,
        runtime_type: runtime_type.clone(),
        started_at: std::time::SystemTime::now(),
        timeout: WORKLOAD_EXECUTION_TIMEOUT,
        status: toadstool::ExecutionStatus::Pending,
        client_info: crate::state::ClientInfo {
            ip_address: None,
            user_agent: None,
            api_key: None,
            authenticated_user: None,
        },
    };

    {
        let mut executions = state.active_executions.write().await;
        executions.insert(execution_id, execution_info);
    }

    if state
        .event_broadcaster
        .send(ServerEvent::ExecutionStarted {
            execution_id,
            runtime_type: runtime_type.clone(),
            timestamp: std::time::SystemTime::now(),
        })
        .is_err()
    {
        tracing::debug!("No event receivers for ExecutionStarted (normal if no clients connected)");
    }

    (
        StatusCode::ACCEPTED,
        Json(json!({
            "execution_id": execution_id,
            "status": "accepted",
            "runtime_type": runtime_type,
            "timestamp": crate::state::timestamp_to_unix_secs(&std::time::SystemTime::now()),
        })),
    )
}

/// Get execution status endpoint handler
pub async fn get_execution_status_handler(
    State(state): State<ServerState>,
    Path(execution_id): Path<Uuid>,
) -> impl IntoResponse {
    debug!("Execution status requested for: {}", execution_id);

    let active_executions = state.active_executions.read().await;

    match active_executions.get(&execution_id) {
        Some(execution) => {
            let response = json!({
                "execution_id": execution.execution_id,
                "status": execution.status,
                "runtime_type": execution.runtime_type,
                "started_at": crate::state::timestamp_to_unix_secs(&execution.started_at),
                "timeout": execution.timeout.as_secs(),
                "timestamp": crate::state::timestamp_to_unix_secs(&std::time::SystemTime::now()),
            });
            (StatusCode::OK, Json(response))
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "Execution not found",
                "execution_id": execution_id,
                "timestamp": crate::state::timestamp_to_unix_secs(&std::time::SystemTime::now()),
            })),
        ),
    }
}

/// Cancel execution endpoint handler
pub async fn cancel_execution_handler(
    State(state): State<ServerState>,
    Path(execution_id): Path<Uuid>,
) -> impl IntoResponse {
    debug!("Execution cancellation requested for: {}", execution_id);

    let mut active_executions = state.active_executions.write().await;

    if let Some(execution_info) = active_executions.get_mut(&execution_id) {
        match execution_info.status {
            toadstool::ExecutionStatus::Running | toadstool::ExecutionStatus::Pending => {
                execution_info.status = toadstool::ExecutionStatus::Cancelled;

                info!(
                    "Marking execution {} as cancelled (runtime {:?} doesn't support direct cancellation)",
                    execution_id, execution_info.runtime_type
                );

                (
                    StatusCode::OK,
                    Json(json!({
                        "execution_id": execution_id,
                        "status": "cancelled",
                        "timestamp": crate::state::timestamp_to_unix_secs(&std::time::SystemTime::now()),
                        "message": "Execution cancelled successfully"
                    })),
                )
            }
            _ => (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "INVALID_STATE",
                    "message": format!("Execution {} cannot be cancelled in current state: {:?}",
                                     execution_id, execution_info.status),
                    "execution_id": execution_id,
                    "current_status": format!("{:?}", execution_info.status)
                })),
            ),
        }
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "EXECUTION_NOT_FOUND",
                "message": format!("Execution {} not found", execution_id),
                "execution_id": execution_id
            })),
        )
    }
}
