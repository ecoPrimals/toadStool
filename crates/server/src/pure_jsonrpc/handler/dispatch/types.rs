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
    #[expect(
        dead_code,
        reason = "used once VFIO dispatch pipeline tracks in-flight jobs"
    )]
    Running,
    Completed,
    Failed(String),
}

impl std::fmt::Display for DispatchStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Submitted => write!(f, "submitted"),
            Self::Running => write!(f, "running"),
            Self::Completed => write!(f, "completed"),
            Self::Failed(msg) => write!(f, "failed: {msg}"),
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
    #[expect(
        dead_code,
        reason = "recorded for scheduling hints and future substrate-aware routing"
    )]
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
    #[expect(
        dead_code,
        reason = "used once pipeline pre-validation catches structural errors before execution"
    )]
    Failed(String),
}

impl std::fmt::Display for PipelineStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Submitted => write!(f, "submitted"),
            Self::Running { current_stage } => write!(f, "running:{current_stage}"),
            Self::Completed => write!(f, "completed"),
            Self::PartialFailure {
                failed_stage,
                error,
                ..
            } => {
                write!(f, "partial_failure:{failed_stage}:{error}")
            }
            Self::Failed(msg) => write!(f, "failed:{msg}"),
        }
    }
}
