//! Workload execution handlers for primal capability system

use axum::{extract::State, response::IntoResponse, Json};
use tracing::{debug, info};

use crate::types::ApiError;
use crate::ApiState;

/// Execute a workload via the primal capability system
#[utoipa::path(
    post,
    path = "/api/v2/workload/execute",
    request_body = toadstool_distributed::WorkloadRequest,
    responses(
        (status = 200, description = "Workload executed successfully", body = toadstool_distributed::WorkloadResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 500, description = "Execution failed", body = ApiError)
    ),
    tag = "workload"
)]
pub async fn execute_workload(
    State(_state): State<ApiState>,
    Json(request): Json<toadstool_distributed::WorkloadRequest>,
) -> Result<impl IntoResponse, ApiError> {
    use toadstool_distributed::WorkloadExecutor;

    info!(
        "Received workload execution request {} from primal {}",
        request.request_id, request.from_primal
    );

    // Log the required capability
    debug!("Required capability: {}", request.required_capability);

    // Use the WorkloadExecutor from the capability system
    let executor = WorkloadExecutor::new();
    let response = executor.execute(request.clone()).await.map_err(|e| {
        ApiError::new(
            "EXECUTION_ERROR",
            &format!("Failed to execute workload: {}", e),
        )
    })?;

    info!(
        "Workload execution request {} completed with status {:?}",
        response.request_id, response.status
    );

    Ok(Json(response))
}
