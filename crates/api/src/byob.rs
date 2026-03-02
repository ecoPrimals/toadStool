//! # BYOB HTTP API for Toadstool
//!
//! HTTP API endpoints for handling BYOB deployment requests from Songbird.
//! Provides `RESTful` endpoints for deploying, monitoring, and managing team biomes.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

use toadstool::{
    byob::{ByobDeploymentRequest, ByobDeploymentResponse, ByobExecutor, ResourceUsage},
    ToadStoolError,
};

/// HTTP API for BYOB operations
pub struct ByobApi {
    executor: Arc<dyn ByobExecutor>,
}

impl ByobApi {
    /// Create a new BYOB API
    pub fn new(executor: Arc<dyn ByobExecutor>) -> Self {
        Self { executor }
    }

    /// Create router for BYOB API
    pub fn router(&self) -> Router<Arc<dyn ByobExecutor>> {
        Router::new()
            .route("/byob/deploy", post(deploy_biome))
            .route("/byob/deployments", get(list_deployments))
            .route(
                "/byob/deployments/:deployment_id",
                get(get_deployment_status),
            )
            .route(
                "/byob/deployments/:deployment_id/stop",
                post(stop_deployment),
            )
            .route(
                "/byob/deployments/:deployment_id/usage",
                get(get_resource_usage),
            )
            .route("/byob/health", get(health_check))
            .with_state(Arc::clone(&self.executor))
    }
}

/// Deploy a team biome
async fn deploy_biome(
    State(executor): State<Arc<dyn ByobExecutor>>,
    Json(request): Json<ByobDeploymentRequest>,
) -> Result<Json<ByobDeploymentResponse>, ApiError> {
    info!(
        "Received BYOB deployment request for team {}",
        request.team_id
    );

    match executor.deploy_biome(request).await {
        Ok(response) => {
            info!(
                "BYOB deployment {} completed successfully",
                response.deployment_id
            );
            Ok(Json(response))
        }
        Err(e) => {
            error!("BYOB deployment failed: {:?}", e);
            Err(ApiError::from(e))
        }
    }
}

/// List all active deployments
async fn list_deployments(
    State(executor): State<Arc<dyn ByobExecutor>>,
) -> Result<Json<Vec<ByobDeploymentResponse>>, ApiError> {
    match executor.list_deployments().await {
        Ok(deployments) => Ok(Json(deployments)),
        Err(e) => {
            error!("Failed to list deployments: {:?}", e);
            Err(ApiError::from(e))
        }
    }
}

/// Get deployment status
async fn get_deployment_status(
    State(executor): State<Arc<dyn ByobExecutor>>,
    Path(deployment_id): Path<Uuid>,
) -> Result<Json<ByobDeploymentResponse>, ApiError> {
    match executor.get_deployment_status(deployment_id).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            error!("Failed to get deployment status: {:?}", e);
            Err(ApiError::from(e))
        }
    }
}

/// Stop a deployment
///
/// # Errors
///
/// Returns `ApiError` if:
/// - Deployment ID is not found
/// - Deployment is already stopped
/// - Stop operation fails due to system error
async fn stop_deployment(
    State(executor): State<Arc<dyn ByobExecutor>>,
    Path(deployment_id): Path<Uuid>,
) -> Result<Json<StopDeploymentResponse>, ApiError> {
    match executor.stop_deployment(deployment_id).await {
        Ok(()) => Ok(Json(StopDeploymentResponse {
            deployment_id,
            message: "Deployment stopped successfully".to_string(),
        })),
        Err(e) => {
            error!("Failed to stop deployment: {:?}", e);
            Err(ApiError::from(e))
        }
    }
}

/// Get resource usage for a deployment
///
/// # Errors
///
/// Returns `ApiError` if:
/// - Deployment ID is not found
/// - Resource metrics are unavailable
/// - Metrics collection fails
async fn get_resource_usage(
    State(executor): State<Arc<dyn ByobExecutor>>,
    Path(deployment_id): Path<Uuid>,
) -> Result<Json<ResourceUsage>, ApiError> {
    match executor.get_resource_usage(deployment_id).await {
        Ok(usage) => Ok(Json(usage)),
        Err(e) => {
            error!("Failed to get resource usage: {:?}", e);
            Err(ApiError::from(e))
        }
    }
}

/// Health check endpoint
///
/// # Errors
///
/// This function is infallible and always returns Ok.
/// The Result type is maintained for API consistency.
async fn health_check() -> Result<Json<HealthResponse>, ApiError> {
    Ok(Json(HealthResponse {
        status: "healthy".to_string(),
        message: "Toadstool BYOB API is operational".to_string(),
    }))
}

/// Stop deployment response
#[derive(Debug, Serialize, Deserialize)]
pub struct StopDeploymentResponse {
    pub deployment_id: Uuid,
    pub message: String,
}

/// Health check response
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub message: String,
}

/// API error type
#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
}

impl ApiError {
    #[must_use]
    pub fn new(status: StatusCode, message: &str) -> Self {
        Self {
            status,
            message: message.to_string(),
        }
    }
}

impl From<ToadStoolError> for ApiError {
    fn from(err: ToadStoolError) -> Self {
        // Match on the unified error system's specialized error types
        use toadstool::error::{ConfigError, ResourceError, SystemError};

        match err {
            // Resource errors -> NOT_FOUND
            ToadStoolError::Resource(ResourceError::NotFound { .. }) => Self {
                status: StatusCode::NOT_FOUND,
                message: err.to_string(),
            },
            // Config validation errors -> BAD_REQUEST
            ToadStoolError::Configuration(ConfigError::ValidationError { .. }) => Self {
                status: StatusCode::BAD_REQUEST,
                message: err.to_string(),
            },
            // Resource allocation/limit errors -> INSUFFICIENT_STORAGE
            ToadStoolError::Resource(ResourceError::AllocationFailure { .. })
            | ToadStoolError::Resource(ResourceError::LimitExceeded { .. })
            | ToadStoolError::Resource(ResourceError::Insufficient { .. }) => Self {
                status: StatusCode::INSUFFICIENT_STORAGE,
                message: err.to_string(),
            },
            // Not supported errors -> NOT_IMPLEMENTED
            ToadStoolError::System(SystemError::NotSupported { .. }) => Self {
                status: StatusCode::NOT_IMPLEMENTED,
                message: err.to_string(),
            },
            // Lightweight NotFound variant (Session 24) -> 404 Not Found
            ToadStoolError::NotFound(_) => Self {
                status: StatusCode::NOT_FOUND,
                message: err.to_string(),
            },
            // All other errors (including Runtime) -> INTERNAL_SERVER_ERROR
            _ => Self {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: err.to_string(),
            },
        }
    }
}

impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let body = Json(serde_json::json!({
            "error": self.message
        }));
        (self.status, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_health_check() {
        let response = health_check().await.unwrap();
        assert_eq!(response.status, "healthy");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_health_response_message() {
        let response = health_check().await.unwrap();
        assert!(response.message.contains("operational"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_api_error_conversion() {
        let error = ToadStoolError::not_found("test".to_string());
        let api_error = ApiError::from(error);
        assert_eq!(api_error.status, StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_api_error_new() {
        let error = ApiError::new(StatusCode::BAD_REQUEST, "Invalid input");
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
        assert_eq!(error.message, "Invalid input");
    }

    #[test]
    fn test_stop_deployment_response_serialization() {
        let response = StopDeploymentResponse {
            deployment_id: Uuid::new_v4(),
            message: "Stopped".to_string(),
        };
        let json = serde_json::to_string(&response).expect("serialize");
        assert!(json.contains("Stopped"));
    }

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: "healthy".to_string(),
            message: "OK".to_string(),
        };
        let json = serde_json::to_string(&response).expect("serialize");
        assert!(json.contains("healthy"));
        assert!(json.contains("OK"));
    }
}
