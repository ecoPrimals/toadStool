// SPDX-License-Identifier: AGPL-3.0-or-later
//! Dispatch telemetry for cross-primal consumption.
//!
//! # Consumer: barraCuda `ml.mlp_train` (36-dim perceptron)
//!
//! barraCuda trains a 36-dim perceptron over dispatch telemetry to predict
//! optimal gate routing. The schema is organized into 6 groups:
//!
//! | Dims  | Group           | Fields                                                           | Encoding            |
//! |-------|-----------------|------------------------------------------------------------------|---------------------|
//! | 0–3   | Identity        | `gate_of_origin`, `trust_level`, `dispatch_mode`, `method`       | string → hash/onehot|
//! | 4–8   | Timing          | `queue_wait_ms`, `dispatch_ms`, `readback_ms`, `total_ms`, `timeout_ms` | numeric (ms) |
//! | 9–14  | Workload shape  | `binary_size_bytes`, `workgroup_{x,y,z}`, `buffer_count`, `total_buffer_bytes` | numeric   |
//! | 15–20 | Hardware        | `gpu_vendor`, `gpu_device_id`, `bdf`, `vram_total_mb`, `vram_used_mb`, `thermal_throttled` | mixed |
//! | 21–25 | Resource envelope | `mem_limit_mb`, `cpu_limit_cores`, `timeout_limit_ms`, `tenant_id`, `priority` | mixed     |
//! | 26–30 | Outcome         | `success`, `error_code`, `retried`, `forwarded`, `remote_gate`   | bool/nullable       |
//! | 31–35 | Mesh context    | `local_gate_id`, `mesh_hop_count`, `yield_strategy`, `guest_load_active`, `timestamp_unix_ms` | mixed |
//!
//! ## How to consume
//!
//! 1. **Schema discovery**: call `dispatch.telemetry.schema` JSON-RPC → returns field list + version.
//! 2. **Feature vector**: call `DispatchTelemetryRecord::to_feature_vector()` to get a `[f64; 36]`
//!    with string fields hashed to `f64` via FNV-1a for numeric stability.
//! 3. **Normalization**: consumers should min-max normalize each dimension across their training set.

/// Wire contract version for dispatch telemetry.
/// Consumers (barraCuda, biomeOS) should validate this on connection.
pub const TELEMETRY_SCHEMA_VERSION: &str = "1.1";

use crate::pure_jsonrpc::handler::method_gate::CallerContext;
use serde::{Deserialize, Serialize};

/// Structured telemetry record for a single dispatch event.
/// Schema aligns with barraCuda ml.mlp_train's 36-dim perceptron feature vector.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "schema type for downstream dispatch telemetry — populated when submit paths emit records"
)]
pub struct DispatchTelemetryRecord {
    // Identity (dims 0-3)
    pub gate_of_origin: Option<String>,
    pub trust_level: String,
    pub dispatch_mode: String,
    pub method: String,

    // Timing (dims 4-8)
    pub queue_wait_ms: u64,
    pub dispatch_ms: u64,
    pub readback_ms: u64,
    pub total_ms: u64,
    pub timeout_ms: u64,

    // Workload shape (dims 9-14)
    pub binary_size_bytes: u64,
    pub workgroup_x: u32,
    pub workgroup_y: u32,
    pub workgroup_z: u32,
    pub buffer_count: u32,
    pub total_buffer_bytes: u64,

    // Hardware (dims 15-20)
    pub gpu_vendor: String,
    pub gpu_device_id: u32,
    pub bdf: String,
    pub vram_total_mb: u64,
    pub vram_used_mb: u64,
    pub thermal_throttled: bool,

    // Resource envelope (dims 21-25)
    pub mem_limit_mb: u64,
    pub cpu_limit_cores: u32,
    pub timeout_limit_ms: u64,
    pub tenant_id: String,
    pub priority: u32,

    // Outcome (dims 26-30)
    pub success: bool,
    pub error_code: Option<i64>,
    pub retried: bool,
    pub forwarded: bool,
    pub remote_gate: Option<String>,

    // Mesh context (dims 31-35)
    pub local_gate_id: String,
    pub mesh_hop_count: u32,
    pub yield_strategy: String,
    pub guest_load_active: u32,
    pub timestamp_unix_ms: u64,
}

impl DispatchTelemetryRecord {
    /// Create a minimal record with defaults for unset fields.
    pub fn new(method: &str, local_gate_id: &str) -> Self {
        Self {
            method: method.to_owned(),
            local_gate_id: local_gate_id.to_owned(),
            timestamp_unix_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            ..Self::default_empty()
        }
    }

    /// Convert to a 36-dimensional feature vector for ml.mlp_train consumption.
    ///
    /// String fields are hashed via FNV-1a to produce stable `f64` values.
    /// Boolean fields map to `0.0` / `1.0`. Nullable fields use `0.0` for `None`.
    #[cfg_attr(not(test), allow(dead_code, reason = "barraCuda ml.mlp_train consumer API"))]
    #[must_use]
    pub fn to_feature_vector(&self) -> [f64; 36] {
        [
            // dims 0-3: identity
            fnv1a_hash_f64(self.gate_of_origin.as_deref().unwrap_or("")),
            fnv1a_hash_f64(&self.trust_level),
            fnv1a_hash_f64(&self.dispatch_mode),
            fnv1a_hash_f64(&self.method),
            // dims 4-8: timing
            self.queue_wait_ms as f64,
            self.dispatch_ms as f64,
            self.readback_ms as f64,
            self.total_ms as f64,
            self.timeout_ms as f64,
            // dims 9-14: workload shape
            self.binary_size_bytes as f64,
            f64::from(self.workgroup_x),
            f64::from(self.workgroup_y),
            f64::from(self.workgroup_z),
            f64::from(self.buffer_count),
            self.total_buffer_bytes as f64,
            // dims 15-20: hardware
            fnv1a_hash_f64(&self.gpu_vendor),
            f64::from(self.gpu_device_id),
            fnv1a_hash_f64(&self.bdf),
            self.vram_total_mb as f64,
            self.vram_used_mb as f64,
            if self.thermal_throttled { 1.0 } else { 0.0 },
            // dims 21-25: resource envelope
            self.mem_limit_mb as f64,
            f64::from(self.cpu_limit_cores),
            self.timeout_limit_ms as f64,
            fnv1a_hash_f64(&self.tenant_id),
            f64::from(self.priority),
            // dims 26-30: outcome
            if self.success { 1.0 } else { 0.0 },
            self.error_code.map_or(0.0, |c| c as f64),
            if self.retried { 1.0 } else { 0.0 },
            if self.forwarded { 1.0 } else { 0.0 },
            fnv1a_hash_f64(self.remote_gate.as_deref().unwrap_or("")),
            // dims 31-35: mesh context
            fnv1a_hash_f64(&self.local_gate_id),
            f64::from(self.mesh_hop_count),
            fnv1a_hash_f64(&self.yield_strategy),
            f64::from(self.guest_load_active),
            self.timestamp_unix_ms as f64,
        ]
    }

    /// Serialize caller trust level for telemetry (snake_case, matches JSON-RPC).
    pub fn trust_level_from_caller(ctx: &CallerContext) -> String {
        serde_json::to_value(ctx.trust_level)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_else(|| String::from("anonymous"))
    }

    fn default_empty() -> Self {
        Self {
            gate_of_origin: None,
            trust_level: String::from("anonymous"),
            dispatch_mode: String::new(),
            method: String::new(),
            queue_wait_ms: 0,
            dispatch_ms: 0,
            readback_ms: 0,
            total_ms: 0,
            timeout_ms: 0,
            binary_size_bytes: 0,
            workgroup_x: 0,
            workgroup_y: 0,
            workgroup_z: 0,
            buffer_count: 0,
            total_buffer_bytes: 0,
            gpu_vendor: String::new(),
            gpu_device_id: 0,
            bdf: String::new(),
            vram_total_mb: 0,
            vram_used_mb: 0,
            thermal_throttled: false,
            mem_limit_mb: 0,
            cpu_limit_cores: 0,
            timeout_limit_ms: 0,
            tenant_id: String::from("anonymous"),
            priority: 3,
            success: false,
            error_code: None,
            retried: false,
            forwarded: false,
            remote_gate: None,
            local_gate_id: String::new(),
            mesh_hop_count: 0,
            yield_strategy: String::from("queue"),
            guest_load_active: 0,
            timestamp_unix_ms: 0,
        }
    }
}

/// Parameters for emitting dispatch-path telemetry at completion.
pub struct DispatchTelemetryEmit<'a> {
    pub ctx: &'a CallerContext,
    pub method: &'static str,
    pub dispatch_ms: u64,
    pub readback_ms: u64,
    pub dispatch_mode: &'a str,
    pub bdf: &'a str,
    pub binary_bytes: &'a [u8],
    pub workgroup_size: [u32; 3],
    pub timeout_ms: u64,
    pub success: bool,
}

/// Build and emit a dispatch telemetry record from completion parameters.
pub fn emit_dispatch_completion_telemetry(p: &DispatchTelemetryEmit<'_>) {
    let mut telemetry = DispatchTelemetryRecord::new(
        p.method,
        &crate::pure_jsonrpc::handler::resolve_local_gate_id().unwrap_or_default(),
    );
    telemetry.gate_of_origin.clone_from(&p.ctx.gate_id);
    telemetry.trust_level = DispatchTelemetryRecord::trust_level_from_caller(p.ctx);
    p.dispatch_mode.clone_into(&mut telemetry.dispatch_mode);
    telemetry.dispatch_ms = p.dispatch_ms;
    telemetry.readback_ms = p.readback_ms;
    telemetry.total_ms = p.dispatch_ms.saturating_add(p.readback_ms);
    telemetry.timeout_ms = p.timeout_ms;
    telemetry.binary_size_bytes = p.binary_bytes.len() as u64;
    telemetry.workgroup_x = p.workgroup_size[0];
    telemetry.workgroup_y = p.workgroup_size[1];
    telemetry.workgroup_z = p.workgroup_size[2];
    p.bdf.clone_into(&mut telemetry.bdf);
    telemetry.success = p.success;
    emit_telemetry_record(&telemetry);
}

/// Emit a telemetry record as structured tracing output.
/// Consumers (barraCuda, biomeOS) can capture this via tracing subscribers.
pub fn emit_telemetry_record(record: &DispatchTelemetryRecord) {
    tracing::info!(
        target: "dispatch.telemetry",
        method = %record.method,
        gate_of_origin = ?record.gate_of_origin,
        trust_level = %record.trust_level,
        dispatch_mode = %record.dispatch_mode,
        dispatch_ms = record.dispatch_ms,
        total_ms = record.total_ms,
        success = record.success,
        forwarded = record.forwarded,
        local_gate_id = %record.local_gate_id,
        "dispatch telemetry"
    );
}

/// JSON-RPC handler: `dispatch.telemetry.schema`
/// Returns the 36-dim feature schema for ml.mlp_train consumption.
#[must_use]
pub fn telemetry_schema() -> serde_json::Value {
    serde_json::json!({
        "contract": "dispatch.telemetry",
        "version": TELEMETRY_SCHEMA_VERSION,
        "previous_versions": ["1.0"],
        "backward_compatible": true,
        "dimensions": 36,
        "fields": [
            // dims 0-3: identity
            "gate_of_origin", "trust_level", "dispatch_mode", "method",
            // dims 4-8: timing
            "queue_wait_ms", "dispatch_ms", "readback_ms", "total_ms", "timeout_ms",
            // dims 9-14: workload shape
            "binary_size_bytes", "workgroup_x", "workgroup_y", "workgroup_z",
            "buffer_count", "total_buffer_bytes",
            // dims 15-20: hardware
            "gpu_vendor", "gpu_device_id", "bdf", "vram_total_mb", "vram_used_mb",
            "thermal_throttled",
            // dims 21-25: resource envelope
            "mem_limit_mb", "cpu_limit_cores", "timeout_limit_ms", "tenant_id", "priority",
            // dims 26-30: outcome
            "success", "error_code", "retried", "forwarded", "remote_gate",
            // dims 31-35: mesh context
            "local_gate_id", "mesh_hop_count", "yield_strategy",
            "guest_load_active", "timestamp_unix_ms",
        ],
        "encoding": {
            "string_fields": {
                "method": "fnv1a_hash_f64",
                "description": "FNV-1a hash scaled to [0, 1) for numeric stability",
                "affected_dims": [0, 1, 2, 3, 15, 17, 24, 30, 31, 33]
            },
            "boolean_fields": {
                "method": "binary",
                "description": "false → 0.0, true → 1.0",
                "affected_dims": [20, 26, 28, 29]
            },
            "nullable_fields": {
                "method": "zero_default",
                "description": "None → 0.0 (strings hash empty string)",
                "affected_dims": [0, 27, 30]
            },
            "numeric_fields": {
                "method": "raw_cast",
                "description": "Direct u64/u32 → f64 cast",
                "affected_dims": [4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 16, 18, 19, 21, 22, 23, 25, 27, 32, 34, 35]
            }
        },
        "consumers": ["barraCuda:ml.mlp_train", "biomeOS:L5.perceptron"],
        "normalization": "min_max_per_dimension",
        "timestamp_epoch": "unix_ms",
    })
}

/// FNV-1a hash of a string, scaled to `f64` in `[0, 1)` for perceptron input stability.
#[cfg_attr(not(test), allow(dead_code, reason = "used by to_feature_vector for ml.mlp_train"))]
fn fnv1a_hash_f64(s: &str) -> f64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in s.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    (hash >> 1) as f64 / (u64::MAX >> 1) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_valid_record() {
        let record = DispatchTelemetryRecord::new("compute.dispatch.submit", "gate-local-1");
        assert_eq!(record.method, "compute.dispatch.submit");
        assert_eq!(record.local_gate_id, "gate-local-1");
        assert_eq!(record.trust_level, "anonymous");
        assert_eq!(record.tenant_id, "anonymous");
        assert_eq!(record.priority, 3);
        assert_eq!(record.yield_strategy, "queue");
        assert!(!record.success);
        assert!(record.timestamp_unix_ms > 0);
    }

    #[test]
    fn telemetry_schema_has_36_fields() {
        let schema = telemetry_schema();
        assert_eq!(schema["contract"], "dispatch.telemetry");
        assert_eq!(schema["version"], TELEMETRY_SCHEMA_VERSION);
        assert_eq!(schema["dimensions"], 36);
        let fields = schema["fields"].as_array().expect("fields array");
        assert_eq!(fields.len(), 36);
        let encoding = schema["encoding"].as_object().expect("encoding object");
        assert!(encoding.contains_key("string_fields"));
        assert!(encoding.contains_key("boolean_fields"));
        assert!(encoding.contains_key("nullable_fields"));
        assert!(encoding.contains_key("numeric_fields"));
        let consumers = schema["consumers"].as_array().expect("consumers array");
        assert!(!consumers.is_empty());
    }

    #[test]
    fn feature_vector_has_36_dimensions() {
        let record = DispatchTelemetryRecord::new("compute.dispatch.submit", "gate-local-1");
        let vec = record.to_feature_vector();
        assert_eq!(vec.len(), 36);
        assert!(vec.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn feature_vector_string_fields_in_unit_range() {
        let record = DispatchTelemetryRecord::new("shader.dispatch", "gate-abc");
        let vec = record.to_feature_vector();
        // dims 0-3 are hashed strings — should be in [0, 1)
        for &dim in &vec[0..4] {
            assert!(dim >= 0.0 && dim < 1.0, "hash dim out of range: {dim}");
        }
    }

    #[test]
    fn feature_vector_deterministic() {
        let record = DispatchTelemetryRecord::new("test.method", "gate-x");
        let v1 = record.to_feature_vector();
        let v2 = record.to_feature_vector();
        assert_eq!(v1, v2);
    }

    #[test]
    fn record_serializes_to_json() {
        let record = DispatchTelemetryRecord::new("shader.dispatch", "gate-abc");
        let json = serde_json::to_value(&record).expect("serialize");
        assert_eq!(json["method"], "shader.dispatch");
        assert_eq!(json["local_gate_id"], "gate-abc");
        assert_eq!(json["trust_level"], "anonymous");
        assert!(json.get("gate_of_origin").is_some());
        assert!(json.get("timestamp_unix_ms").is_some());
    }
}
