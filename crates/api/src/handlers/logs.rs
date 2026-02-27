//! Log retrieval handlers
//!
//! This module contains handlers for retrieving execution logs

use axum::{
    extract::State,
    extract::{Path, Query},
    response::IntoResponse,
    Json,
};
use std::time::SystemTime;

use tracing::{debug, warn};
use uuid::Uuid;

use crate::constants::EXECUTOR_SOURCE;
use crate::types::{ApiError, ExecutionLogs, LogEntry, LogLevel, TimeRange};
use crate::ApiState;

/// Parse RFC3339 timestamp string (e.g. "2025-12-02T10:00:00Z") to SystemTime
fn parse_rfc3339_to_system_time(s: &str) -> Option<SystemTime> {
    use std::time::Duration;
    let s = s.trim_end_matches('Z');
    let parts: Vec<&str> = s.split('T').collect();
    if parts.len() != 2 {
        return None;
    }
    let date_parts: Vec<&str> = parts[0].split('-').collect();
    let time_parts: Vec<&str> = parts[1].split(':').collect();
    if date_parts.len() != 3 || time_parts.len() < 3 {
        return None;
    }
    let year: i32 = date_parts[0].parse().ok()?;
    let month: u32 = date_parts[1].parse().ok()?;
    let day: u32 = date_parts[2].parse().ok()?;
    let hour: u32 = time_parts[0].parse().ok()?;
    let minute: u32 = time_parts[1].parse().ok()?;
    let second: u32 = time_parts[2].split('.').next()?.parse().ok()?;
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }
    // Simplified: compute days since 1970-01-01
    let days = days_from_ymd(year, month, day);
    let secs =
        days as u64 * 86400 + u64::from(hour) * 3600 + u64::from(minute) * 60 + u64::from(second);
    Some(SystemTime::UNIX_EPOCH + Duration::from_secs(secs))
}

fn days_from_ymd(year: i32, month: u32, day: u32) -> i64 {
    let (y, m, d) = (year as i64, month as i64, day as i64);
    let (y, m) = if m <= 2 { (y - 1, m + 12) } else { (y, m) };
    let jdn = (1461 * (y + 4800 + (m - 14) / 12)) / 4 + (367 * (m - 2 - 12 * ((m - 14) / 12))) / 12
        - (3 * ((y + 4900 + (m - 14) / 12) / 100)) / 4
        + d
        - 32075;
    jdn - 2440588 // 2440588 is Julian day for 1970-01-01
}

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
    let now = SystemTime::now();
    let execution_logs: Vec<LogEntry> = vec![
        LogEntry {
            timestamp: now,
            level: LogLevel::Info,
            message: "Execution started".to_string(),
            source: EXECUTOR_SOURCE.to_string(),
        },
        LogEntry {
            timestamp: now,
            level: LogLevel::Debug,
            message: "Initializing runtime environment".to_string(),
            source: EXECUTOR_SOURCE.to_string(),
        },
        LogEntry {
            timestamp: now,
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
pub fn parse_log_line(line: &str) -> Option<LogEntry> {
    let parts: Vec<&str> = line.splitn(4, ' ').collect();
    if parts.len() < 4 {
        return None;
    }

    let timestamp = parse_rfc3339_to_system_time(parts[0]).unwrap_or_else(SystemTime::now);

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
