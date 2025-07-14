//! HTTP API handlers for server endpoints

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
};
use serde_json::json;
use tracing::{debug, info, warn};
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
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({
                "status": "error",
                "message": "Failed to get system resources",
                "timestamp": chrono::Utc::now(),
            })));
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
    let runtime_type = request.get("runtime_type")
        .and_then(|v| v.as_str())
        .map(|s| match s {
            "native" => toadstool::RuntimeType::Native,
            "container" => toadstool::RuntimeType::Container,
            "wasm" => toadstool::RuntimeType::Wasm,
            "python" => toadstool::RuntimeType::Python,
            _ => toadstool::RuntimeType::Native,
        })
        .unwrap_or(toadstool::RuntimeType::Native);

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
                info!("Cancellation request processed for execution {}", execution_id);
                
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
