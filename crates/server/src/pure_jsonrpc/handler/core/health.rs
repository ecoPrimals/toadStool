// SPDX-License-Identifier: AGPL-3.0-or-later
//! JSON-RPC health probes and full health envelope.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::pure_jsonrpc::types::JsonRpcError;
use crate::rpc_types::HealthStatus;

use super::JsonRpcResult;

/// Wire Standard L1: minimal liveness probe (`health.liveness`).
///
/// Returns `{"status": "starting"}` during initialization (before discovery
/// registration completes) and `{"status": "alive"}` once fully ready.
/// This fast-path lets callers distinguish "socket exists but still
/// initializing" from "fully operational" without timing out (PG-62).
///
/// Recommended caller timeout: >= 3 seconds.
#[allow(
    clippy::unused_async,
    reason = "handler signature requires async for uniform dispatch"
)]
pub(crate) async fn health_liveness(ready: bool) -> JsonRpcResult {
    let status = if ready { "alive" } else { "starting" };
    Ok(serde_json::json!({ "status": status }))
}

/// Wire Standard L2: readiness probe with version (`health.readiness`).
///
/// Returns `"starting"` during initialization, `"ready"` once fully operational.
#[allow(
    clippy::unused_async,
    reason = "handler signature requires async for uniform dispatch"
)]
pub(crate) async fn health_readiness(version: &str, ready: bool) -> JsonRpcResult {
    let status = if ready { "ready" } else { "starting" };
    Ok(serde_json::json!({
        "status": status,
        "version": version,
    }))
}

/// Returns health status with uptime and error count.
///
/// Used by `toadstool.health`, `health.check`, `compute.health`, and semantic `check_health`.
/// Wire Standard L1: includes `"status": "alive"` on the full envelope for orchestrators
/// that expect the legacy shape on deep checks.
#[allow(
    clippy::unused_async,
    reason = "handler signature requires async for uniform dispatch"
)]
pub(crate) async fn health(
    version: &Arc<str>,
    start_time: std::time::Instant,
    error_count: &AtomicU64,
) -> JsonRpcResult {
    let uptime = start_time.elapsed();
    #[expect(
        clippy::cast_possible_truncation,
        reason = "error count u64→usize is lossless on 64-bit"
    )]
    let error_count_val = error_count.load(Ordering::Relaxed) as usize;
    let status = HealthStatus {
        healthy: true,
        version: Arc::clone(version),
        uptime_secs: uptime.as_secs(),
        active_workloads: 0,
        queued_workloads: 0,
        error_count: error_count_val,
        resource_utilization: 0.0,
    };
    let mut value = serde_json::to_value(status)
        .map_err(|e| JsonRpcError::internal_error(format!("Serialization error: {e}")))?;
    // Wire Standard L1: biomeOS probes expect "status": "alive"
    if let Some(obj) = value.as_object_mut() {
        obj.insert("status".into(), serde_json::Value::String("alive".into()));
    }
    Ok(value)
}
