// SPDX-License-Identifier: AGPL-3.0-or-later

#[derive(Debug, Clone)]
pub(super) struct DispatchJob {
    #[expect(
        dead_code,
        reason = "stored for logging/diagnostics in dispatch pipeline"
    )]
    pub(super) id: String,
    pub(super) bdf: String,
    pub(super) status: DispatchStatus,
    pub(super) submitted_at: std::time::Instant,
    pub(super) binary_size: usize,
    pub(super) result: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum DispatchStatus {
    Submitted,
    Running,
    Completed,
    Failed(String),
}

impl DispatchStatus {
    /// Wire-stable status tag without embedded error details.
    pub(super) fn as_str(&self) -> &str {
        match self {
            Self::Submitted => "submitted",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed(_) => "failed",
        }
    }
}

impl std::fmt::Display for DispatchStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed(msg) => write!(f, "failed: {msg}"),
            other => f.write_str(other.as_str()),
        }
    }
}

// ═══════════════════════════════════════════════════════════
// Pipeline dispatch types — ordered multi-stage compute (neuralSpring PG-05)
// ═══════════════════════════════════════════════════════════

/// Preferred execution substrate for a pipeline stage.
///
/// Recorded per-stage for scheduling hints and future substrate-aware routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum PipelineSubstrate {
    CpuOnly,
    GpuOnly,
    GpuPreferred,
    Any,
}

impl std::fmt::Display for PipelineSubstrate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CpuOnly => write!(f, "cpu_only"),
            Self::GpuOnly => write!(f, "gpu_only"),
            Self::GpuPreferred => write!(f, "gpu_preferred"),
            Self::Any => write!(f, "any"),
        }
    }
}

/// A single stage in a pipeline dispatch request.
#[derive(Debug, Clone, serde::Deserialize)]
pub(super) struct PipelineStageRequest {
    pub(super) id: String,
    pub(super) method: String,
    #[serde(default)]
    pub(super) params: serde_json::Value,
    #[serde(default = "default_substrate")]
    pub(super) substrate: PipelineSubstrate,
}

fn default_substrate() -> PipelineSubstrate {
    PipelineSubstrate::Any
}

/// Tracked state of a pipeline dispatch.
#[derive(Debug, Clone)]
pub(super) struct PipelineJob {
    #[expect(
        dead_code,
        reason = "stored for logging/diagnostics in pipeline tracking"
    )]
    pub(super) id: String,
    pub(super) name: String,
    pub(super) status: PipelineStatus,
    pub(super) submitted_at: std::time::Instant,
    pub(super) stage_count: usize,
    pub(super) stages_completed: usize,
    pub(super) stage_results: Vec<PipelineStageResult>,
}

/// Result from a single pipeline stage execution.
#[derive(Debug, Clone, serde::Serialize)]
pub(super) struct PipelineStageResult {
    pub(super) stage_id: String,
    pub(super) method: String,
    pub(super) substrate: PipelineSubstrate,
    pub(super) status: String,
    pub(super) elapsed_ms: u64,
    pub(super) result: Option<serde_json::Value>,
    pub(super) error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum PipelineStatus {
    Submitted,
    Running {
        current_stage: String,
    },
    Completed,
    PartialFailure {
        completed: usize,
        failed_stage: String,
        error: String,
    },
    Failed(String),
}

impl PipelineStatus {
    /// Wire-stable status tag without embedded details.
    pub(super) fn as_str(&self) -> &str {
        match self {
            Self::Submitted => "submitted",
            Self::Running { .. } => "running",
            Self::Completed => "completed",
            Self::PartialFailure { .. } => "partial_failure",
            Self::Failed(_) => "failed",
        }
    }
}

impl std::fmt::Display for PipelineStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Running { current_stage } => write!(f, "running:{current_stage}"),
            Self::PartialFailure {
                failed_stage,
                error,
                ..
            } => write!(f, "partial_failure:{failed_stage}:{error}"),
            Self::Failed(msg) => write!(f, "failed:{msg}"),
            other => f.write_str(other.as_str()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_status_display() {
        assert_eq!(DispatchStatus::Submitted.to_string(), "submitted");
        assert_eq!(DispatchStatus::Running.to_string(), "running");
        assert_eq!(DispatchStatus::Completed.to_string(), "completed");
        assert_eq!(
            DispatchStatus::Failed("timeout".into()).to_string(),
            "failed: timeout"
        );
    }

    #[test]
    fn pipeline_substrate_display() {
        assert_eq!(PipelineSubstrate::CpuOnly.to_string(), "cpu_only");
        assert_eq!(PipelineSubstrate::GpuOnly.to_string(), "gpu_only");
        assert_eq!(PipelineSubstrate::GpuPreferred.to_string(), "gpu_preferred");
        assert_eq!(PipelineSubstrate::Any.to_string(), "any");
    }

    #[test]
    fn pipeline_substrate_serde_roundtrip() {
        for variant in [
            PipelineSubstrate::CpuOnly,
            PipelineSubstrate::GpuOnly,
            PipelineSubstrate::GpuPreferred,
            PipelineSubstrate::Any,
        ] {
            let json = serde_json::to_value(variant).unwrap();
            let back: PipelineSubstrate = serde_json::from_value(json).unwrap();
            assert_eq!(variant, back);
        }
    }

    #[test]
    fn pipeline_substrate_deserializes_snake_case() {
        let val: PipelineSubstrate = serde_json::from_str("\"gpu_preferred\"").unwrap();
        assert_eq!(val, PipelineSubstrate::GpuPreferred);
    }

    #[test]
    fn pipeline_status_display_all_variants() {
        assert_eq!(PipelineStatus::Submitted.to_string(), "submitted");
        assert_eq!(PipelineStatus::Completed.to_string(), "completed");
        assert_eq!(
            PipelineStatus::Running {
                current_stage: "attention".into()
            }
            .to_string(),
            "running:attention"
        );
        assert_eq!(
            PipelineStatus::PartialFailure {
                completed: 1,
                failed_stage: "ffn".into(),
                error: "oom".into()
            }
            .to_string(),
            "partial_failure:ffn:oom"
        );
        assert_eq!(
            PipelineStatus::Failed("bad graph".into()).to_string(),
            "failed:bad graph"
        );
    }

    #[test]
    fn default_substrate_is_any() {
        assert_eq!(default_substrate(), PipelineSubstrate::Any);
    }

    #[test]
    fn pipeline_stage_request_deserializes_without_substrate() {
        let json = serde_json::json!({
            "id": "stage1",
            "method": "compute.dispatch.submit",
            "params": {}
        });
        let req: PipelineStageRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.id, "stage1");
        assert_eq!(req.substrate, PipelineSubstrate::Any);
    }

    #[test]
    fn pipeline_stage_request_deserializes_with_substrate() {
        let json = serde_json::json!({
            "id": "gpu_stage",
            "method": "shader.dispatch",
            "params": {"binary": [1]},
            "substrate": "gpu_only"
        });
        let req: PipelineStageRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.substrate, PipelineSubstrate::GpuOnly);
    }

    #[test]
    fn pipeline_stage_result_serializes() {
        let result = PipelineStageResult {
            stage_id: "tok".into(),
            method: "compute.dispatch.submit".into(),
            substrate: PipelineSubstrate::GpuPreferred,
            status: "completed".into(),
            elapsed_ms: 42,
            result: Some(serde_json::json!({"data": [1, 2]})),
            error: None,
        };
        let val = serde_json::to_value(&result).unwrap();
        assert_eq!(val["stage_id"], "tok");
        assert_eq!(val["elapsed_ms"], 42);
        assert_eq!(val["substrate"], "gpu_preferred");
        assert!(val["error"].is_null());
        assert_eq!(val["result"]["data"], serde_json::json!([1, 2]));
    }

    #[test]
    fn pipeline_stage_result_serializes_error_case() {
        let result = PipelineStageResult {
            stage_id: "broken".into(),
            method: "shader.dispatch".into(),
            substrate: PipelineSubstrate::Any,
            status: "failed".into(),
            elapsed_ms: 5,
            result: None,
            error: Some("device lost".into()),
        };
        let val = serde_json::to_value(&result).unwrap();
        assert_eq!(val["error"], "device lost");
        assert!(val["result"].is_null());
    }

    #[test]
    fn dispatch_status_equality() {
        assert_eq!(DispatchStatus::Submitted, DispatchStatus::Submitted);
        assert_eq!(DispatchStatus::Completed, DispatchStatus::Completed);
        assert_ne!(DispatchStatus::Submitted, DispatchStatus::Completed);
        assert_eq!(
            DispatchStatus::Failed("x".into()),
            DispatchStatus::Failed("x".into())
        );
        assert_ne!(
            DispatchStatus::Failed("x".into()),
            DispatchStatus::Failed("y".into())
        );
    }

    #[test]
    fn pipeline_status_equality() {
        assert_eq!(PipelineStatus::Submitted, PipelineStatus::Submitted);
        assert_ne!(PipelineStatus::Submitted, PipelineStatus::Completed);
        assert_eq!(
            PipelineStatus::Running {
                current_stage: "a".into()
            },
            PipelineStatus::Running {
                current_stage: "a".into()
            }
        );
        assert_ne!(
            PipelineStatus::Running {
                current_stage: "a".into()
            },
            PipelineStatus::Running {
                current_stage: "b".into()
            }
        );
    }
}
