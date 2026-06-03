// SPDX-License-Identifier: AGPL-3.0-or-later

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
