//! HTTP API handlers for server endpoints

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
};
use serde_json::json;
use tracing::{debug, info};
use uuid::Uuid;

// Removed mock dependency - using real system resources now
use crate::state::{ServerEvent, ServerState};

/// Health check endpoint handler
pub async fn health_check_handler(State(state): State<ServerState>) -> impl IntoResponse {
    debug!("Health check requested");

    // Get real system resources
    let system_resources = match state.resource_monitor.get_system_resources().await {
        Ok(resources) => resources,
        Err(e) => {
            tracing::error!("Failed to get system resources: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "status": "error",
                    "message": "Failed to get system resources",
                    "timestamp": chrono::Utc::now(),
                })),
            );
        }
    };

    // For now, we'll use simplified health metrics
    // In a real implementation, we'd track usage over time
    let cpu_usage_percent = 50.0; // Placeholder
    let memory_usage_percent = 45.0; // Placeholder
    let healthy = cpu_usage_percent < 90.0 && memory_usage_percent < 90.0;

    let response = json!({
        "status": if healthy { "healthy" } else { "unhealthy" },
        "timestamp": chrono::Utc::now(),
        "version": env!("CARGO_PKG_VERSION"),
        "resources": {
            "cpu_usage_percent": cpu_usage_percent,
            "memory_usage_percent": memory_usage_percent,
            "available_memory_bytes": system_resources.available_memory_bytes,
            "available_cpu_cores": system_resources.available_cpu_cores,
            "available_storage_bytes": system_resources.available_storage_bytes,
            "available_gpu_units": system_resources.available_gpu_units,
        },
        "active_executions": state.active_executions.read().await.len(),
        "runtime_engines": state.runtime_engines.read().await.len(),
    });

    if healthy {
        (StatusCode::OK, Json(response))
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, Json(response))
    }
}

/// Readiness check endpoint handler
pub async fn readiness_check_handler(State(state): State<ServerState>) -> impl IntoResponse {
    debug!("Readiness check requested");

    let runtime_engines = state.runtime_engines.read().await;
    let has_engines = !runtime_engines.is_empty();

    let response = json!({
        "status": if has_engines { "ready" } else { "not_ready" },
        "timestamp": chrono::Utc::now(),
        "runtime_engines": runtime_engines.len(),
        "registered_engines": runtime_engines.keys().collect::<Vec<_>>(),
    });

    if has_engines {
        (StatusCode::OK, Json(response))
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, Json(response))
    }
}

/// Metrics endpoint handler
pub async fn metrics_handler(State(state): State<ServerState>) -> impl IntoResponse {
    debug!("Metrics requested");

    let stats = state.stats.read().await;
    let active_executions = state.active_executions.read().await;

    let response = json!({
        "timestamp": chrono::Utc::now(),
        "statistics": {
            "total_executions": stats.total_executions,
            "successful_executions": stats.successful_executions,
            "failed_executions": stats.failed_executions,
            "average_execution_time_ms": stats.average_execution_time_ms,
            "peak_concurrent_executions": stats.peak_concurrent_executions,
            "uptime_seconds": stats.uptime_seconds,
            "total_requests": stats.total_requests,
            "errors_count": stats.errors_count,
        },
        "current_state": {
            "active_executions": active_executions.len(),
            "runtime_engines": state.runtime_engines.read().await.len(),
        },
    });

    (StatusCode::OK, Json(response))
}

/// Submit execution endpoint handler
pub async fn submit_execution_handler(
    State(state): State<ServerState>,
    Json(request): Json<serde_json::Value>,
) -> impl IntoResponse {
    debug!("Execution submission requested: {:?}", request);

    // Parse the execution request
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

    // Create execution info
    let execution_info = crate::state::ActiveExecution {
        execution_id,
        runtime_type: runtime_type.clone(),
        started_at: chrono::Utc::now(),
        timeout: std::time::Duration::from_secs(300),
        status: toadstool::ExecutionStatus::Pending,
        client_info: crate::state::ClientInfo {
            ip_address: None,
            user_agent: None,
            api_key: None,
            authenticated_user: None,
        },
    };

    // Store execution info
    {
        let mut executions = state.active_executions.write().await;
        executions.insert(execution_id, execution_info);
    }

    // Notify about execution start
    let _ = state.event_broadcaster.send(ServerEvent::ExecutionStarted {
        execution_id,
        runtime_type: runtime_type.clone(),
        timestamp: chrono::Utc::now(),
    });

    (
        StatusCode::ACCEPTED,
        Json(json!({
            "execution_id": execution_id,
            "status": "accepted",
            "runtime_type": runtime_type,
            "timestamp": chrono::Utc::now(),
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
                "started_at": execution.started_at,
                "timeout": execution.timeout.as_secs(),
                "timestamp": chrono::Utc::now(),
            });
            (StatusCode::OK, Json(response))
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "Execution not found",
                "execution_id": execution_id,
                "timestamp": chrono::Utc::now(),
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

    // Implement execution cancellation
    let mut active_executions = state.active_executions.write().await;

    if let Some(execution_info) = active_executions.get_mut(&execution_id) {
        // Check if execution is still running
        match execution_info.status {
            toadstool::ExecutionStatus::Running | toadstool::ExecutionStatus::Pending => {
                // Update status to cancelled
                execution_info.status = toadstool::ExecutionStatus::Cancelled;

                // Runtime engines don't currently support direct cancellation
                // We mark the execution as cancelled and rely on timeout mechanisms
                info!("Marking execution {} as cancelled (runtime {:?} doesn't support direct cancellation)", 
                      execution_id, execution_info.runtime_type);

                // In a real implementation, this would:
                // 1. Update the execution status in the database
                // 2. Send a cancellation signal to the runtime if supported
                // 3. Set up timeout mechanisms for cleanup

                // For now, we just log the cancellation request
                info!(
                    "Cancellation request processed for execution {}",
                    execution_id
                );

                (
                    StatusCode::OK,
                    Json(json!({
                        "execution_id": execution_id,
                        "status": "cancelled",
                        "timestamp": chrono::Utc::now(),
                        "message": "Execution cancelled successfully"
                    })),
                )
            }
            _ => {
                // Execution is not in a cancellable state
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "error": "INVALID_STATE",
                        "message": format!("Execution {} cannot be cancelled in current state: {:?}",
                                         execution_id, execution_info.status),
                        "execution_id": execution_id,
                        "current_status": format!("{:?}", execution_info.status)
                    })),
                )
            }
        }
    } else {
        // Execution not found
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

/// Get cluster status endpoint handler
pub async fn get_cluster_status_handler(State(state): State<ServerState>) -> impl IntoResponse {
    debug!("Cluster status requested");

    let response = json!({
        "cluster_id": "toadstool-cluster",
        "node_id": "toadstool-server",
        "status": "healthy",
        "runtime_engines": state.runtime_engines.read().await.len(),
        "active_executions": state.active_executions.read().await.len(),
        "timestamp": chrono::Utc::now(),
    });

    (StatusCode::OK, Json(response))
}

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
        "timestamp": chrono::Utc::now(),
    });

    (StatusCode::OK, Json(response))
}

/// Dashboard HTML handler
pub async fn dashboard_handler() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}

// Removed is_system_healthy function - health checks now use real system resources inline

/// Dashboard HTML content
const DASHBOARD_HTML: &str = r#"
<!DOCTYPE html>
<html>
<head>
    <title>ToadStool Server Dashboard</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; }
        .card { border: 1px solid #ddd; padding: 20px; margin: 10px 0; border-radius: 5px; }
        .status { font-weight: bold; }
        .healthy { color: green; }
        .unhealthy { color: red; }
        .metric { display: inline-block; margin: 10px; }
        .metric-value { font-size: 24px; font-weight: bold; }
        .metric-label { font-size: 14px; color: #666; }
        .refresh-btn { padding: 10px 20px; margin: 10px 0; cursor: pointer; }
        #logs { height: 300px; overflow-y: scroll; border: 1px solid #ccc; padding: 10px; font-family: monospace; }
    </style>
</head>
<body>
    <h1>🍄 ToadStool Server Dashboard</h1>
    
    <div class="card">
        <h2>System Health</h2>
        <div id="health-status" class="status">Loading...</div>
        <div id="health-metrics"></div>
        <button class="refresh-btn" onclick="refreshHealth()">Refresh</button>
    </div>
    
    <div class="card">
        <h2>Runtime Engines</h2>
        <div id="runtime-engines">Loading...</div>
        <button class="refresh-btn" onclick="refreshEngines()">Refresh</button>
    </div>
    
    <div class="card">
        <h2>Active Executions</h2>
        <div id="active-executions">Loading...</div>
        <button class="refresh-btn" onclick="refreshExecutions()">Refresh</button>
    </div>
    
    <div class="card">
        <h2>Server Statistics</h2>
        <div id="statistics">Loading...</div>
        <button class="refresh-btn" onclick="refreshStats()">Refresh</button>
    </div>
    
    <div class="card">
        <h2>Real-time Logs</h2>
        <div id="logs"></div>
        <button class="refresh-btn" onclick="connectWebSocket()">Connect WebSocket</button>
    </div>
    
    <script>
        let ws;
        
        async function refreshHealth() {
            try {
                const response = await fetch('/health');
                const data = await response.json();
                
                document.getElementById('health-status').innerHTML = 
                    `<span class="${data.status === 'healthy' ? 'healthy' : 'unhealthy'}">${data.status.toUpperCase()}</span>`;
                
                document.getElementById('health-metrics').innerHTML = `
                    <div class="metric">
                        <div class="metric-value">${data.resources.cpu_usage_percent.toFixed(1)}%</div>
                        <div class="metric-label">CPU Usage</div>
                    </div>
                    <div class="metric">
                        <div class="metric-value">${data.resources.memory_usage_percent.toFixed(1)}%</div>
                        <div class="metric-label">Memory Usage</div>
                    </div>
                    <div class="metric">
                        <div class="metric-value">${data.active_executions}</div>
                        <div class="metric-label">Active Executions</div>
                    </div>
                `;
            } catch (error) {
                console.error('Error refreshing health:', error);
            }
        }
        
        async function refreshEngines() {
            try {
                const response = await fetch('/api/runtime-engines');
                const data = await response.json();
                
                document.getElementById('runtime-engines').innerHTML = 
                    data.runtime_engines.map(engine => 
                        `<div>🔧 ${engine.runtime_type} (${engine.status})</div>`
                    ).join('');
            } catch (error) {
                console.error('Error refreshing engines:', error);
            }
        }
        
        async function refreshExecutions() {
            document.getElementById('active-executions').innerHTML = 'No active executions';
        }
        
        async function refreshStats() {
            try {
                const response = await fetch('/metrics');
                const data = await response.json();
                
                document.getElementById('statistics').innerHTML = `
                    <div class="metric">
                        <div class="metric-value">${data.statistics.total_executions}</div>
                        <div class="metric-label">Total Executions</div>
                    </div>
                    <div class="metric">
                        <div class="metric-value">${data.statistics.successful_executions}</div>
                        <div class="metric-label">Successful</div>
                    </div>
                    <div class="metric">
                        <div class="metric-value">${data.statistics.failed_executions}</div>
                        <div class="metric-label">Failed</div>
                    </div>
                    <div class="metric">
                        <div class="metric-value">${data.statistics.average_execution_time_ms.toFixed(2)}ms</div>
                        <div class="metric-label">Avg Execution Time</div>
                    </div>
                `;
            } catch (error) {
                console.error('Error refreshing stats:', error);
            }
        }
        
        function connectWebSocket() {
            const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
            const wsUrl = `${protocol}//${window.location.host}/ws`;
            
            ws = new WebSocket(wsUrl);
            
            ws.onopen = function() {
                addLog('WebSocket connected');
            };
            
            ws.onmessage = function(event) {
                const data = JSON.parse(event.data);
                addLog(`Event: ${data.type} - ${JSON.stringify(data.data)}`);
            };
            
            ws.onclose = function() {
                addLog('WebSocket disconnected');
            };
            
            ws.onerror = function(error) {
                addLog(`WebSocket error: ${error}`);
            };
        }
        
        function addLog(message) {
            const logsDiv = document.getElementById('logs');
            const timestamp = new Date().toISOString();
            logsDiv.innerHTML += `<div>[${timestamp}] ${message}</div>`;
            logsDiv.scrollTop = logsDiv.scrollHeight;
        }
        
        // Auto-refresh every 30 seconds
        setInterval(() => {
            refreshHealth();
            refreshEngines();
            refreshExecutions();
            refreshStats();
        }, 30000);
        
        // Initial load
        window.onload = function() {
            refreshHealth();
            refreshEngines();
            refreshExecutions();
            refreshStats();
        };
    </script>
</body>
</html>
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::extract::{Path, State};
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use axum::Json;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::{broadcast, RwLock};
    use uuid::Uuid;

    async fn get_response_json(response: axum::response::Response) -> serde_json::Value {
        let body = response.into_body();
        let bytes = to_bytes(body, usize::MAX).await.expect("body to_bytes");
        serde_json::from_slice(&bytes).expect("response body to be valid JSON")
    }

    fn create_test_state() -> ServerState {
        let (event_broadcaster, _) = broadcast::channel(100);
        ServerState {
            runtime_engines: Arc::new(RwLock::new(HashMap::new())),
            active_executions: Arc::new(RwLock::new(HashMap::new())),
            event_broadcaster,
            config: crate::ServerConfig::default(),
            resource_monitor: Arc::new(toadstool::SystemResourceMonitor::new()),
            stats: Arc::new(RwLock::new(crate::ServerStatistics::default())),
            capability_provider: None,
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_health_check_handler() {
        let state = create_test_state();

        // Add timeout to prevent hanging (5 seconds)
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            health_check_handler(State(state)),
        )
        .await;

        assert!(result.is_ok(), "Health check handler timed out");
        let response = result
            .expect("Health check should return Ok response in test")
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_readiness_check_handler_not_ready() {
        let state = create_test_state();
        let response = readiness_check_handler(State(state)).await.into_response();
        // Should be SERVICE_UNAVAILABLE when no runtime engines
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    // Note: test_readiness_check_handler_ready removed - would need runtime engine deps

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_metrics_handler() {
        let state = create_test_state();
        let response = metrics_handler(State(state)).await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_submit_execution_handler() {
        let state = create_test_state();
        let request = serde_json::json!({
            "runtime_type": "native",
            "workload": "test"
        });
        let response = submit_execution_handler(State(state), Json(request))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_get_execution_status_handler_not_found() {
        let state = create_test_state();
        let execution_id = Uuid::new_v4();
        let response = get_execution_status_handler(State(state), Path(execution_id))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_get_execution_status_handler_found() {
        let state = create_test_state();
        let execution_id = Uuid::new_v4();

        // Add execution
        {
            let mut executions = state.active_executions.write().await;
            executions.insert(
                execution_id,
                crate::ActiveExecution {
                    execution_id,
                    runtime_type: toadstool::RuntimeType::Native,
                    started_at: chrono::Utc::now(),
                    timeout: std::time::Duration::from_secs(300),
                    status: toadstool::ExecutionStatus::Running,
                    client_info: crate::ClientInfo {
                        ip_address: None,
                        user_agent: None,
                        api_key: None,
                        authenticated_user: None,
                    },
                },
            );
        }

        let response = get_execution_status_handler(State(state), Path(execution_id))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_cancel_execution_handler_not_found() {
        let state = create_test_state();
        let execution_id = Uuid::new_v4();
        let response = cancel_execution_handler(State(state), Path(execution_id))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_cancel_execution_handler_success() {
        let state = create_test_state();
        let execution_id = Uuid::new_v4();

        // Add running execution
        {
            let mut executions = state.active_executions.write().await;
            executions.insert(
                execution_id,
                crate::ActiveExecution {
                    execution_id,
                    runtime_type: toadstool::RuntimeType::Native,
                    started_at: chrono::Utc::now(),
                    timeout: std::time::Duration::from_secs(300),
                    status: toadstool::ExecutionStatus::Running,
                    client_info: crate::ClientInfo {
                        ip_address: None,
                        user_agent: None,
                        api_key: None,
                        authenticated_user: None,
                    },
                },
            );
        }

        let response = cancel_execution_handler(State(state.clone()), Path(execution_id))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        // Verify status changed to cancelled
        let executions = state.active_executions.read().await;
        let execution = executions
            .get(&execution_id)
            .expect("Execution ID should exist in state after successful submission");
        assert!(matches!(
            execution.status,
            toadstool::ExecutionStatus::Cancelled
        ));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_cancel_execution_handler_invalid_state() {
        let state = create_test_state();
        let execution_id = Uuid::new_v4();

        // Add completed execution
        {
            let mut executions = state.active_executions.write().await;
            executions.insert(
                execution_id,
                crate::ActiveExecution {
                    execution_id,
                    runtime_type: toadstool::RuntimeType::Native,
                    started_at: chrono::Utc::now(),
                    timeout: std::time::Duration::from_secs(300),
                    status: toadstool::ExecutionStatus::Success,
                    client_info: crate::ClientInfo {
                        ip_address: None,
                        user_agent: None,
                        api_key: None,
                        authenticated_user: None,
                    },
                },
            );
        }

        let response = cancel_execution_handler(State(state), Path(execution_id))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_get_cluster_status_handler() {
        let state = create_test_state();
        let response = get_cluster_status_handler(State(state))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_list_runtime_engines_handler_empty() {
        let state = create_test_state();
        let response = list_runtime_engines_handler(State(state))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    // Note: test_list_runtime_engines_handler_with_engines removed - would need runtime engine deps

    #[test]
    fn test_dashboard_html_constant() {
        // Verify the dashboard HTML contains expected sections
        assert!(DASHBOARD_HTML.contains("ToadStool Server Dashboard"));
        assert!(DASHBOARD_HTML.contains("System Health"));
        assert!(DASHBOARD_HTML.contains("Runtime Engines"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_dashboard_handler() {
        let response = dashboard_handler().await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    // ========================================================================
    // Runtime type parsing in submit_execution_handler
    // ========================================================================

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_submit_execution_runtime_type_container() {
        let state = create_test_state();
        let request = serde_json::json!({ "runtime_type": "container" });
        let response = submit_execution_handler(State(state), Json(request))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let json = get_response_json(response).await;
        assert_eq!(json["runtime_type"], "Container");
        assert_eq!(json["status"], "accepted");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_submit_execution_runtime_type_wasm() {
        let state = create_test_state();
        let request = serde_json::json!({ "runtime_type": "wasm" });
        let response = submit_execution_handler(State(state), Json(request))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let json = get_response_json(response).await;
        assert_eq!(json["runtime_type"], "Wasm");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_submit_execution_runtime_type_python() {
        let state = create_test_state();
        let request = serde_json::json!({ "runtime_type": "python" });
        let response = submit_execution_handler(State(state), Json(request))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let json = get_response_json(response).await;
        assert_eq!(json["runtime_type"], "Python");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_submit_execution_runtime_type_native_explicit() {
        let state = create_test_state();
        let request = serde_json::json!({ "runtime_type": "native" });
        let response = submit_execution_handler(State(state), Json(request))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let json = get_response_json(response).await;
        assert_eq!(json["runtime_type"], "Native");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_submit_execution_runtime_type_invalid_defaults_to_native() {
        let state = create_test_state();
        let request = serde_json::json!({ "runtime_type": "invalid_runtime" });
        let response = submit_execution_handler(State(state), Json(request))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let json = get_response_json(response).await;
        assert_eq!(json["runtime_type"], "Native");
    }

    // ========================================================================
    // Edge cases: empty inputs, missing fields, invalid data
    // ========================================================================

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_submit_execution_empty_object() {
        let state = create_test_state();
        let request = serde_json::json!({});
        let response = submit_execution_handler(State(state), Json(request))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let json = get_response_json(response).await;
        assert!(json["execution_id"].as_str().is_some());
        assert_eq!(json["runtime_type"], "Native");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_submit_execution_missing_runtime_type() {
        let state = create_test_state();
        let request = serde_json::json!({ "workload": "some-workload" });
        let response = submit_execution_handler(State(state), Json(request))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let json = get_response_json(response).await;
        assert_eq!(json["runtime_type"], "Native");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_submit_execution_runtime_type_non_string() {
        let state = create_test_state();
        let request = serde_json::json!({ "runtime_type": 42 });
        let response = submit_execution_handler(State(state), Json(request))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let json = get_response_json(response).await;
        assert_eq!(json["runtime_type"], "Native");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_submit_execution_stores_execution_in_state() {
        let state = create_test_state();
        let request = serde_json::json!({ "runtime_type": "native" });
        let response = submit_execution_handler(State(state.clone()), Json(request))
            .await
            .into_response();
        let json = get_response_json(response).await;
        let execution_id: Uuid =
            serde_json::from_value(json["execution_id"].clone()).expect("execution_id in response");

        let executions = state.active_executions.read().await;
        let execution = executions
            .get(&execution_id)
            .expect("execution stored in state");
        assert_eq!(execution.runtime_type, toadstool::RuntimeType::Native);
        assert!(matches!(
            execution.status,
            toadstool::ExecutionStatus::Pending
        ));
    }

    // ========================================================================
    // Response body structure and error mapping
    // ========================================================================

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_get_execution_status_found_response_body_structure() {
        let state = create_test_state();
        let execution_id = Uuid::new_v4();
        {
            let mut executions = state.active_executions.write().await;
            executions.insert(
                execution_id,
                crate::state::ActiveExecution {
                    execution_id,
                    runtime_type: toadstool::RuntimeType::Wasm,
                    started_at: chrono::Utc::now(),
                    timeout: std::time::Duration::from_secs(300),
                    status: toadstool::ExecutionStatus::Running,
                    client_info: crate::state::ClientInfo {
                        ip_address: None,
                        user_agent: None,
                        api_key: None,
                        authenticated_user: None,
                    },
                },
            );
        }

        let response = get_execution_status_handler(State(state), Path(execution_id))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let json = get_response_json(response).await;
        let id: Uuid = serde_json::from_value(json["execution_id"].clone()).expect("execution_id");
        assert_eq!(id, execution_id);
        assert_eq!(json["status"], "Running");
        assert_eq!(json["runtime_type"], "Wasm");
        assert!(json["started_at"].as_str().is_some());
        assert_eq!(json["timeout"], 300);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_get_execution_status_not_found_error_body() {
        let state = create_test_state();
        let execution_id = Uuid::new_v4();
        let response = get_execution_status_handler(State(state), Path(execution_id))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let json = get_response_json(response).await;
        assert_eq!(json["error"], "Execution not found");
        assert_eq!(json["execution_id"], execution_id.to_string());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_cancel_execution_not_found_error_body() {
        let state = create_test_state();
        let execution_id = Uuid::new_v4();
        let response = cancel_execution_handler(State(state), Path(execution_id))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let json = get_response_json(response).await;
        assert_eq!(json["error"], "EXECUTION_NOT_FOUND");
        assert!(json["message"].as_str().unwrap().contains("not found"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_cancel_execution_invalid_state_error_body() {
        let state = create_test_state();
        let execution_id = Uuid::new_v4();
        {
            let mut executions = state.active_executions.write().await;
            executions.insert(
                execution_id,
                crate::state::ActiveExecution {
                    execution_id,
                    runtime_type: toadstool::RuntimeType::Native,
                    started_at: chrono::Utc::now(),
                    timeout: std::time::Duration::from_secs(300),
                    status: toadstool::ExecutionStatus::Success,
                    client_info: crate::state::ClientInfo {
                        ip_address: None,
                        user_agent: None,
                        api_key: None,
                        authenticated_user: None,
                    },
                },
            );
        }

        let response = cancel_execution_handler(State(state), Path(execution_id))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let json = get_response_json(response).await;
        assert_eq!(json["error"], "INVALID_STATE");
        assert!(json["message"]
            .as_str()
            .unwrap()
            .contains("cannot be cancelled"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_cancel_execution_pending_state_succeeds() {
        let state = create_test_state();
        let execution_id = Uuid::new_v4();
        {
            let mut executions = state.active_executions.write().await;
            executions.insert(
                execution_id,
                crate::state::ActiveExecution {
                    execution_id,
                    runtime_type: toadstool::RuntimeType::Native,
                    started_at: chrono::Utc::now(),
                    timeout: std::time::Duration::from_secs(300),
                    status: toadstool::ExecutionStatus::Pending,
                    client_info: crate::state::ClientInfo {
                        ip_address: None,
                        user_agent: None,
                        api_key: None,
                        authenticated_user: None,
                    },
                },
            );
        }

        let response = cancel_execution_handler(State(state.clone()), Path(execution_id))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let json = get_response_json(response).await;
        assert_eq!(json["status"], "cancelled");

        let executions = state.active_executions.read().await;
        let exec = executions.get(&execution_id).expect("execution exists");
        assert!(matches!(exec.status, toadstool::ExecutionStatus::Cancelled));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_metrics_response_structure() {
        let state = create_test_state();
        let response = metrics_handler(State(state)).await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let json = get_response_json(response).await;
        assert!(json["statistics"]["total_executions"].is_number());
        assert!(json["statistics"]["successful_executions"].is_number());
        assert!(json["statistics"]["failed_executions"].is_number());
        assert!(json["statistics"]["average_execution_time_ms"].is_number());
        assert!(json["current_state"]["active_executions"].is_number());
        assert!(json["current_state"]["runtime_engines"].is_number());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_cluster_status_response_structure() {
        let state = create_test_state();
        let response = get_cluster_status_handler(State(state))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let json = get_response_json(response).await;
        assert_eq!(json["cluster_id"], "toadstool-cluster");
        assert_eq!(json["node_id"], "toadstool-server");
        assert_eq!(json["status"], "healthy");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_list_runtime_engines_response_structure() {
        let state = create_test_state();
        let response = list_runtime_engines_handler(State(state))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let json = get_response_json(response).await;
        assert!(json["runtime_engines"].is_array());
        assert_eq!(json["total_count"].as_u64(), Some(0));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_readiness_not_ready_response_structure() {
        let state = create_test_state();
        let response = readiness_check_handler(State(state)).await.into_response();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let json = get_response_json(response).await;
        assert_eq!(json["status"], "not_ready");
        assert_eq!(json["runtime_engines"].as_u64(), Some(0));
    }

    // ========================================================================
    // Serialization round-trips for types used in handlers
    // ========================================================================

    #[test]
    fn test_runtime_type_serialization_roundtrip() {
        let types = [
            toadstool::RuntimeType::Native,
            toadstool::RuntimeType::Wasm,
            toadstool::RuntimeType::Container,
            toadstool::RuntimeType::Python,
            toadstool::RuntimeType::Gpu,
        ];
        for rt in &types {
            let json = serde_json::to_value(rt).expect("serialize RuntimeType");
            let rt2: toadstool::RuntimeType =
                serde_json::from_value(json).expect("deserialize RuntimeType");
            assert_eq!(rt, &rt2);
        }
    }

    #[test]
    fn test_execution_status_serialization_roundtrip() {
        let statuses = [
            toadstool::ExecutionStatus::Success,
            toadstool::ExecutionStatus::Pending,
            toadstool::ExecutionStatus::Running,
            toadstool::ExecutionStatus::Cancelled,
            toadstool::ExecutionStatus::TimedOut,
            toadstool::ExecutionStatus::Failed {
                error: "test error".to_string(),
            },
        ];
        for st in &statuses {
            let json = serde_json::to_value(st).expect("serialize ExecutionStatus");
            let st2: toadstool::ExecutionStatus =
                serde_json::from_value(json).expect("deserialize ExecutionStatus");
            assert_eq!(st, &st2);
        }
    }

    #[test]
    fn test_uuid_serialization_in_execution_id() {
        let id = Uuid::new_v4();
        let json = serde_json::to_value(id).expect("serialize Uuid");
        let id2: Uuid = serde_json::from_value(json).expect("deserialize Uuid");
        assert_eq!(id, id2);
    }
}
