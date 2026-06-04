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

use serde::{Deserialize, Serialize};

/// Structured telemetry record for a single dispatch event.
/// Schema aligns with barraCuda ml.mlp_train's 36-dim perceptron feature vector.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(
    dead_code,
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

#[allow(dead_code, reason = "constructor used by tests and future dispatch emission")]
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

/// JSON-RPC handler: `dispatch.telemetry.schema`
/// Returns the 36-dim feature schema for ml.mlp_train consumption.
#[must_use]
pub fn telemetry_schema() -> serde_json::Value {
    serde_json::json!({
        "version": "1.0",
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
    })
}

/// FNV-1a hash of a string, scaled to `f64` in `[0, 1)` for perceptron input stability.
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
        assert_eq!(schema["version"], "1.0");
        assert_eq!(schema["dimensions"], 36);
        let fields = schema["fields"].as_array().expect("fields array");
        assert_eq!(fields.len(), 36);
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
