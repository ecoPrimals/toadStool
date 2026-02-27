//! Modern Rust Patterns Example for Execution Handlers
//!
//! This file demonstrates how to apply modern idiomatic Rust patterns
//! to replace manual loops and nested conditionals with iterator chains
//! and combinators.

use std::time::SystemTime;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use tracing::{debug, info, warn};
use uuid::Uuid;
use validator::Validate;

use crate::types::{
    ApiError, ExecutionFilter, ExecutionInfo, ExecutionRequest, ExecutionResponse,
    ExecutionStatus, PaginatedResponse, PaginationInfo,
};
use crate::ApiState;

/// Modern Pattern: Iterator chains with combinators for filtering
///
/// BEFORE (nested if statements):
/// ```ignore
/// .filter(|exec| {
///     if let Some(status) = &filter.status {
///         if exec.status != *status {
///             return false;
///         }
///     }
///     if let Some(runtime_type) = &filter.runtime_type {
///         if exec.runtime_type != *runtime_type {
///             return false;
///         }
///     }
///     true
/// })
/// ```
///
/// AFTER (functional combinators):
pub async fn list_executions_modern(
    State(state): State<ApiState>,
    Query(filter): Query<ExecutionFilter>,
) -> Result<impl IntoResponse, ApiError> {
    // Validate using Result combinators
    filter.validate().map_err(ApiError::validation_error)?;

    let page = filter.page.unwrap_or(1);
    let per_page = filter.per_page.unwrap_or(10);

    debug!("Listing executions with filter: {:?}", filter);

    let executions = state.executions.read().await;

    // ✅ MODERN: Functional filtering with combinators
    let mut filtered_executions: Vec<_> = executions
        .values()
        .filter(|exec| {
            // Use combinators to chain conditions
            filter.status.as_ref().map_or(true, |s| &exec.status == s)
                && filter
                    .runtime_type
                    .as_ref()
                    .map_or(true, |rt| &exec.runtime_type == rt)
                && filter
                    .submitted_after
                    .as_ref()
                    .map_or(true, |after| exec.submitted_at >= *after)
                && filter
                    .submitted_before
                    .as_ref()
                    .map_or(true, |before| exec.submitted_at <= *before)
        })
        .collect();

    // ✅ MODERN: Functional sorting
    filtered_executions.sort_by_key(|exec| std::cmp::Reverse(exec.submitted_at));

    let total_items = filtered_executions.len() as u64;
    let total_pages = total_items.div_ceil(u64::from(per_page));

    // ✅ MODERN: Iterator adapters for pagination
    let page_data: Vec<_> = filtered_executions
        .into_iter()
        .skip(((page - 1) * per_page) as usize)
        .take(per_page as usize)
        .cloned()
        .collect();

    let pagination = PaginationInfo {
        page,
        per_page,
        total_pages: total_pages as u32,
        total_items,
        has_next: page < total_pages as u32,
        has_prev: page > 1,
    };

    Ok(Json(PaginatedResponse {
        data: page_data,
        pagination,
    }))
}

/// Modern Pattern: Option/Result combinators instead of nested matches
///
/// BEFORE:
/// ```ignore
/// let value = if let Some(x) = maybe_value {
///     if x > 10 {
///         Some(x * 2)
///     } else {
///         None
///     }
/// } else {
///     None
/// };
/// ```
///
/// AFTER:
/// ```ignore
/// let value = maybe_value
///     .filter(|&x| x > 10)
///     .map(|x| x * 2);
/// ```
pub fn extract_resource_value_modern(
    request: &ExecutionRequest,
) -> (f64, u64, u64, u32) {
    // ✅ MODERN: Option combinators instead of nested if-let
    let cpu_cores = request
        .resources
        .as_ref()
        .and_then(|r| r.cpu_cores)
        .unwrap_or(1.0);

    let memory_mb = request
        .resources
        .as_ref()
        .and_then(|r| r.memory_mb)
        .unwrap_or(512);

    let storage_mb = request
        .resources
        .as_ref()
        .and_then(|r| r.storage_mb)
        .unwrap_or(1024);

    let gpu_count = request
        .resources
        .as_ref()
        .and_then(|r| r.gpu_count)
        .unwrap_or(0);

    (cpu_cores, memory_mb, storage_mb, gpu_count)
}

/// Modern Pattern: Iterator chains for collection transformation
///
/// BEFORE:
/// ```ignore
/// let mut results = Vec::new();
/// for item in items {
///     if item.is_valid() {
///         let processed = item.process();
///         if processed.is_ok() {
///             results.push(processed.unwrap());
///         }
///     }
/// }
/// ```
///
/// AFTER:
pub fn process_executions_modern(
    executions: Vec<ExecutionInfo>,
) -> Vec<(Uuid, ExecutionStatus)> {
    // ✅ MODERN: Functional transformation pipeline
    executions
        .into_iter()
        .filter(|exec| exec.duration_ms.is_some())
        .map(|exec| (exec.execution_id, exec.status))
        .collect()
}

/// Modern Pattern: Error handling with Result combinators
///
/// BEFORE:
/// ```ignore
/// let result = do_something();
/// if result.is_err() {
///     return Err(ApiError::new("ERROR", &result.unwrap_err().to_string()));
/// }
/// let value = result.unwrap();
/// ```
///
/// AFTER:
pub async fn validate_and_submit_modern(
    request: ExecutionRequest,
) -> Result<Uuid, ApiError> {
    // ✅ MODERN: Chained Result operations
    request
        .validate()
        .map_err(ApiError::validation_error)
        .and_then(|_| {
            // Validation passed, generate ID
            Ok(Uuid::new_v4())
        })
}

/// Modern Pattern: Iterator find_map instead of manual loop
///
/// BEFORE:
/// ```ignore
/// let mut result = None;
/// for exec in executions {
///     if exec.status == ExecutionStatus::Running {
///         result = Some(exec.execution_id);
///         break;
///     }
/// }
/// ```
///
/// AFTER:
pub fn find_running_execution_modern(
    executions: &[ExecutionInfo],
) -> Option<Uuid> {
    // ✅ MODERN: find_map combinator
    executions
        .iter()
        .find(|exec| matches!(exec.status, ExecutionStatus::Running))
        .map(|exec| exec.execution_id)
}

/// Modern Pattern: partition instead of manual filtering
///
/// BEFORE:
/// ```ignore
/// let mut completed = Vec::new();
/// let mut failed = Vec::new();
/// for exec in executions {
///     if exec.status == ExecutionStatus::Completed {
///         completed.push(exec);
///     } else if exec.status == ExecutionStatus::Failed {
///         failed.push(exec);
///     }
/// }
/// ```
///
/// AFTER:
pub fn partition_executions_modern(
    executions: Vec<ExecutionInfo>,
) -> (Vec<ExecutionInfo>, Vec<ExecutionInfo>) {
    // ✅ MODERN: partition with predicate
    executions.into_iter().partition(|exec| {
        matches!(
            exec.status,
            ExecutionStatus::Completed | ExecutionStatus::Cancelled
        )
    })
}

/// Modern Pattern: Iterator statistics with fold
///
/// BEFORE:
/// ```ignore
/// let mut total = 0.0;
/// let mut count = 0;
/// for exec in executions {
///     if let Some(duration) = exec.duration_ms {
///         total += duration as f64;
///         count += 1;
///     }
/// }
/// let average = if count > 0 { total / count as f64 } else { 0.0 };
/// ```
///
/// AFTER:
pub fn calculate_average_duration_modern(
    executions: &[ExecutionInfo],
) -> f64 {
    // ✅ MODERN: fold with functional pipeline
    let (total, count) = executions
        .iter()
        .filter_map(|exec| exec.duration_ms)
        .fold((0u64, 0usize), |(sum, count), duration| {
            (sum + duration, count + 1)
        });

    if count > 0 {
        total as f64 / count as f64
    } else {
        0.0
    }
}

/// Modern Pattern: collect into HashMap
///
/// BEFORE:
/// ```ignore
/// let mut map = HashMap::new();
/// for exec in executions {
///     map.insert(exec.execution_id, exec.status);
/// }
/// ```
///
/// AFTER:
pub fn create_status_map_modern(
    executions: Vec<ExecutionInfo>,
) -> std::collections::HashMap<Uuid, ExecutionStatus> {
    // ✅ MODERN: collect directly into HashMap
    executions
        .into_iter()
        .map(|exec| (exec.execution_id, exec.status))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use toadstool::RuntimeType;

    #[test]
    fn test_modern_filtering() {
        let executions = vec![
            ExecutionInfo {
                execution_id: Uuid::new_v4(),
                status: ExecutionStatus::Running,
                runtime_type: RuntimeType::Native,
                submitted_at: SystemTime::now(),
                started_at: None,
                completed_at: None,
                duration_ms: None,
                progress: None,
                error_message: None,
                resource_usage: None,
                metadata: std::collections::HashMap::new(),
            },
            ExecutionInfo {
                execution_id: Uuid::new_v4(),
                status: ExecutionStatus::Completed,
                runtime_type: RuntimeType::Native,
                submitted_at: SystemTime::now(),
                started_at: None,
                completed_at: None,
                duration_ms: Some(1000),
                progress: None,
                error_message: None,
                resource_usage: None,
                metadata: std::collections::HashMap::new(),
            },
        ];

        let processed = process_executions_modern(executions);
        assert_eq!(processed.len(), 1); // Only one has duration_ms
    }

    #[test]
    fn test_modern_find() {
        let executions = vec![
            ExecutionInfo {
                execution_id: Uuid::new_v4(),
                status: ExecutionStatus::Submitted,
                runtime_type: RuntimeType::Native,
                submitted_at: SystemTime::now(),
                started_at: None,
                completed_at: None,
                duration_ms: None,
                progress: None,
                error_message: None,
                resource_usage: None,
                metadata: std::collections::HashMap::new(),
            },
            ExecutionInfo {
                execution_id: Uuid::new_v4(),
                status: ExecutionStatus::Running,
                runtime_type: RuntimeType::Native,
                submitted_at: SystemTime::now(),
                started_at: None,
                completed_at: None,
                duration_ms: None,
                progress: None,
                error_message: None,
                resource_usage: None,
                metadata: std::collections::HashMap::new(),
            },
        ];

        let running_id = find_running_execution_modern(&executions);
        assert!(running_id.is_some());
    }

    #[test]
    fn test_modern_partition() {
        let executions = vec![
            ExecutionInfo {
                execution_id: Uuid::new_v4(),
                status: ExecutionStatus::Completed,
                runtime_type: RuntimeType::Native,
                submitted_at: SystemTime::now(),
                started_at: None,
                completed_at: None,
                duration_ms: Some(1000),
                progress: None,
                error_message: None,
                resource_usage: None,
                metadata: std::collections::HashMap::new(),
            },
            ExecutionInfo {
                execution_id: Uuid::new_v4(),
                status: ExecutionStatus::Failed,
                runtime_type: RuntimeType::Native,
                submitted_at: SystemTime::now(),
                started_at: None,
                completed_at: None,
                duration_ms: Some(500),
                progress: None,
                error_message: Some("error".to_string()),
                resource_usage: None,
                metadata: std::collections::HashMap::new(),
            },
        ];

        let (completed, failed) = partition_executions_modern(executions);
        assert_eq!(completed.len(), 1);
        assert_eq!(failed.len(), 1);
    }
}

