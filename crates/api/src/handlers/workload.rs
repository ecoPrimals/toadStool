//! Workload execution handlers for primal capability system

use axum::{extract::State, response::IntoResponse, Json};
use tracing::{debug, info};

use crate::types::ApiError;
use crate::ApiState;

/// Execute a workload via the primal capability system
///
/// This endpoint receives workload requests from any primal in the ecosystem
/// (Songbird, Squirrel, BearDog, etc.) and executes them based on ToadStool's
/// available capabilities.
#[utoipa::path(
    post,
    path = "/api/v2/workload/execute",
    request_body = toadstool_distributed::primal_capabilities::WorkloadRequest,
    responses(
        (status = 200, description = "Workload executed successfully", body = toadstool_distributed::primal_capabilities::WorkloadResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 500, description = "Execution failed", body = ApiError),
        (status = 503, description = "Capability provider not configured", body = ApiError)
    ),
    tag = "workload"
)]
pub async fn execute_workload(
    State(state): State<ApiState>,
    Json(request): Json<toadstool_distributed::primal_capabilities::WorkloadRequest>,
) -> Result<impl IntoResponse, ApiError> {
    info!(
        "Received workload execution request {} from primal {}",
        request.request_id, request.from_primal
    );

    // Log the required capability
    debug!("Required capability: {}", request.required_capability);

    // Check if capability provider is configured
    let provider = state.capability_provider.as_ref().ok_or_else(|| {
        ApiError::new(
            "CAPABILITY_PROVIDER_NOT_CONFIGURED",
            "Capability provider is not initialized. Primal integration is disabled.",
        )
    })?;

    // Execute workload through the capability provider
    // ✅ OPTIMIZED: Move request instead of cloning (zero-copy)
    let response = provider.handle_workload(request).await.map_err(|e| {
        ApiError::new(
            "EXECUTION_ERROR",
            &format!("Failed to execute workload: {e}"),
        )
    })?;

    info!(
        "Workload execution request {} completed with status {:?}",
        response.request_id, response.status
    );

    Ok(Json(response))
}
