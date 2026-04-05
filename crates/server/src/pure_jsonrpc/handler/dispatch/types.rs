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
