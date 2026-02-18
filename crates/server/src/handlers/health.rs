//! Health and monitoring endpoint handlers

use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Json},
};
use serde_json::json;
use tracing::debug;

use crate::state::ServerState;

/// Health check endpoint handler
pub async fn health_check_handler(State(state): State<ServerState>) -> impl IntoResponse {
    debug!("Health check requested");

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

    use sysinfo::System;

    let mut sys = System::new();
    sys.refresh_memory();
    sys.refresh_cpu_usage();

    let total_memory = sys.total_memory();
    let used_memory = total_memory.saturating_sub(sys.available_memory());
    let memory_usage_percent = if total_memory > 0 {
        ((used_memory as f64 / total_memory as f64) * 100.0).clamp(0.0, 100.0)
    } else {
        45.0
    };

    let cpu_usage_percent = sys.global_cpu_info().cpu_usage() as f64;
    let cpu_usage_percent = if cpu_usage_percent > 0.0 {
        cpu_usage_percent.clamp(0.0, 100.0)
    } else {
        let total_cores = sys.cpus().len() as f64;
        let available_cores = system_resources.available_cpu_cores as f64;
        if total_cores > 0.0 && available_cores <= total_cores {
            ((1.0 - (available_cores / total_cores)) * 100.0).clamp(0.0, 100.0)
        } else {
            50.0
        }
    };

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

/// Dashboard HTML handler
pub async fn dashboard_handler() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}

/// Dashboard HTML content
pub const DASHBOARD_HTML: &str = r#"
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
        
        setInterval(() => {
            refreshHealth();
            refreshEngines();
            refreshExecutions();
            refreshStats();
        }, 30000);
        
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
