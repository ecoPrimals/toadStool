// SPDX-License-Identifier: AGPL-3.0-or-later
//! # BYOB HTTP routes for container runtime
//!
//! HTTP API endpoints for handling BYOB deployment requests from Songbird.
//! Extracted from toadstool-api (S96) to avoid REST/axum/utoipa dependency chain.
//! The API crate has been fossilized; this is the canonical BYOB route implementation.

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

use toadstool::{
    ToadStoolError,
    byob::{ByobDeploymentRequest, ByobDeploymentResponse, ByobExecutor, ResourceUsage},
};

/// HTTP API for BYOB operations (router factory)
pub struct ByobApi {
    executor: Arc<dyn ByobExecutor>,
}

impl ByobApi {
    /// Create a new BYOB API
    pub fn new(executor: Arc<dyn ByobExecutor>) -> Self {
        Self { executor }
    }

    /// Create router for BYOB API with state already applied.
    pub fn router(self) -> Router {
        Self::routes().with_state(self.executor)
    }

    /// Build BYOB route definitions without applying state.
    /// The caller is responsible for providing `Arc<dyn ByobExecutor>` via `with_state`.
    pub fn routes() -> Router<Arc<dyn ByobExecutor>> {
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
    }
}

/// Deploy a team biome
async fn deploy_biome(
    State(executor): State<Arc<dyn ByobExecutor>>,
    Json(request): Json<ByobDeploymentRequest>,
) -> Result<Json<ByobDeploymentResponse>, ByobApiError> {
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
            Err(ByobApiError::from(e))
        }
    }
}

/// List all active deployments
async fn list_deployments(
    State(executor): State<Arc<dyn ByobExecutor>>,
) -> Result<Json<Vec<ByobDeploymentResponse>>, ByobApiError> {
    match executor.list_deployments().await {
        Ok(deployments) => Ok(Json(deployments)),
        Err(e) => {
            error!("Failed to list deployments: {:?}", e);
            Err(ByobApiError::from(e))
        }
    }
}

/// Get deployment status
async fn get_deployment_status(
    State(executor): State<Arc<dyn ByobExecutor>>,
    Path(deployment_id): Path<Uuid>,
) -> Result<Json<ByobDeploymentResponse>, ByobApiError> {
    match executor.get_deployment_status(deployment_id).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            error!("Failed to get deployment status: {:?}", e);
            Err(ByobApiError::from(e))
        }
    }
}

/// Stop a deployment
async fn stop_deployment(
    State(executor): State<Arc<dyn ByobExecutor>>,
    Path(deployment_id): Path<Uuid>,
) -> Result<Json<StopDeploymentResponse>, ByobApiError> {
    match executor.stop_deployment(deployment_id).await {
        Ok(()) => Ok(Json(StopDeploymentResponse {
            deployment_id,
            message: "Deployment stopped successfully".to_string(),
        })),
        Err(e) => {
            error!("Failed to stop deployment: {:?}", e);
            Err(ByobApiError::from(e))
        }
    }
}

/// Get resource usage for a deployment
async fn get_resource_usage(
    State(executor): State<Arc<dyn ByobExecutor>>,
    Path(deployment_id): Path<Uuid>,
) -> Result<Json<ResourceUsage>, ByobApiError> {
    match executor.get_resource_usage(deployment_id).await {
        Ok(usage) => Ok(Json(usage)),
        Err(e) => {
            error!("Failed to get resource usage: {:?}", e);
            Err(ByobApiError::from(e))
        }
    }
}

/// Health check endpoint
async fn health_check() -> Result<Json<HealthResponse>, ByobApiError> {
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

/// API error type for BYOB routes
#[derive(Debug)]
pub struct ByobApiError {
    pub status: StatusCode,
    pub message: String,
}

impl ByobApiError {
    #[allow(dead_code, reason = "preserved for API compatibility")]
    #[must_use]
    pub fn new(status: StatusCode, message: &str) -> Self {
        Self {
            status,
            message: message.to_string(),
        }
    }
}

impl From<ToadStoolError> for ByobApiError {
    fn from(err: ToadStoolError) -> Self {
        use toadstool::error::{ConfigError, ResourceError, SystemError};

        match err {
            ToadStoolError::Resource(ResourceError::NotFound { .. })
            | ToadStoolError::NotFound(_) => Self {
                status: StatusCode::NOT_FOUND,
                message: err.to_string(),
            },
            ToadStoolError::Configuration(ConfigError::ValidationError { .. }) => Self {
                status: StatusCode::BAD_REQUEST,
                message: err.to_string(),
            },
            ToadStoolError::Resource(
                ResourceError::AllocationFailure { .. }
                | ResourceError::LimitExceeded { .. }
                | ResourceError::Insufficient { .. },
            ) => Self {
                status: StatusCode::INSUFFICIENT_STORAGE,
                message: err.to_string(),
            },
            ToadStoolError::System(SystemError::NotSupported { .. }) => Self {
                status: StatusCode::NOT_IMPLEMENTED,
                message: err.to_string(),
            },
            _ => Self {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: err.to_string(),
            },
        }
    }
}

impl axum::response::IntoResponse for ByobApiError {
    fn into_response(self) -> axum::response::Response {
        let body = Json(serde_json::json!({
            "error": self.message
        }));
        (self.status, body).into_response()
    }
}
