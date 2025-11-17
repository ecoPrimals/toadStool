//! Log retrieval handlers
//!
//! This module contains handlers for retrieving execution logs

use axum::{
    extract::State,
    extract::{Path, Query},
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::constants::EXECUTOR_SOURCE;
use crate::types::{ApiError, ExecutionLogs, LogEntry, LogLevel, TimeRange};
use crate::ApiState;

/// Get execution logs
#[utoipa::path(
    get,
    path = "/api/v2/executions/{execution_id}/logs",
    params(
        ("execution_id" = Uuid, Path, description = "Execution ID"),
        ("start" = Option<String>, Query, description = "Start time (ISO 8601)"),
        ("end" = Option<String>, Query, description = "End time (ISO 8601)"),
        ("level" = Option<String>, Query, description = "Minimum log level"),
        ("tail" = Option<u32>, Query, description = "Last N lines"),
        ("follow" = Option<bool>, Query, description = "Stream logs")
    ),
    responses(
        (status = 200, description = "Execution logs", body = ExecutionLogs),
        (status = 404, description = "Execution not found", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "executions"
)]
pub async fn get_execution_logs(
    State(state): State<ApiState>,
    Path(execution_id): Path<Uuid>,
    Query(_params): Query<TimeRange>,
) -> Result<impl IntoResponse, ApiError> {
    debug!("Getting logs for execution {}", execution_id);

    // Time range validation can be added if needed in the future

    // Check if execution exists
    let executions = state.executions.read().await;
    if !executions.contains_key(&execution_id) {
        warn!("Execution {} not found", execution_id);
        return Err(ApiError::new(
            "EXECUTION_NOT_FOUND",
            &format!("Execution {execution_id} not found"),
        ));
    }
    drop(executions);

    // Generate sample logs
    let execution_logs: Vec<LogEntry> = vec![
        LogEntry {
            timestamp: Utc::now(),
            level: LogLevel::Info,
            message: "Execution started".to_string(),
            source: EXECUTOR_SOURCE.to_string(),
        },
        LogEntry {
            timestamp: Utc::now(),
            level: LogLevel::Debug,
            message: "Initializing runtime environment".to_string(),
            source: EXECUTOR_SOURCE.to_string(),
        },
        LogEntry {
            timestamp: Utc::now(),
            level: LogLevel::Info,
            message: "Runtime environment ready".to_string(),
            source: EXECUTOR_SOURCE.to_string(),
        },
    ];

    let total_lines = execution_logs.len();
    let has_more = false;
    let next_token = None;

    let logs = ExecutionLogs {
        execution_id,
        logs: execution_logs,
        total_lines: total_lines as u64,
        has_more,
        next_token,
    };

    Ok(Json(logs))
}

/// Parse a log line into a `LogEntry`
///
/// Basic log parsing - assumes format: "timestamp level `[source]` message"
#[allow(dead_code)]
pub fn parse_log_line(line: &str) -> Option<LogEntry> {
    let parts: Vec<&str> = line.splitn(4, ' ').collect();
    if parts.len() < 4 {
        return None;
    }

    let timestamp = chrono::DateTime::parse_from_rfc3339(parts[0])
        .ok()
        .map_or_else(Utc::now, |dt| dt.with_timezone(&Utc));

    let level = match parts[1].to_lowercase().as_str() {
        "error" => LogLevel::Error,
        "warn" | "warning" => LogLevel::Warn,
        "info" => LogLevel::Info,
        "debug" => LogLevel::Debug,
        _ => LogLevel::Info,
    };

    let source = parts[2]
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_string();
    let message = parts[3].to_string();

    Some(LogEntry {
        timestamp,
        level,
        message,
        source,
    })
}
