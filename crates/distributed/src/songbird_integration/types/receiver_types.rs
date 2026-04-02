// SPDX-License-Identifier: AGPL-3.0-only
//! Job receiver types

use tokio::sync::mpsc;

use super::message_types::SongbirdJobMessage;

// ============================================================================
// Job Receiver
// ============================================================================

/// Async channel endpoint for inbound Songbird job messages.
pub struct JobReceiver {
    /// Underlying tokio receiver.
    pub receiver: mpsc::Receiver<SongbirdJobMessage>,
}

impl JobReceiver {
    /// Awaits the next job message, or `None` if the channel closed.
    pub async fn receive(&mut self) -> Option<SongbirdJobMessage> {
        self.receiver.recv().await
    }
}

/// Processor identity and metadata for universal job handling.
pub struct UniversalJobProcessor {
    /// Stable id for this processor instance.
    pub processor_id: String,
    /// Human-readable name for logging/monitoring.
    pub display_name: String,
    /// Maximum number of concurrent jobs this processor can handle.
    pub max_concurrent_jobs: usize,
    /// Job types this processor is capable of executing.
    pub supported_job_types: Vec<String>,
}

impl UniversalJobProcessor {
    /// Creates a processor with defaults suitable for local single-node operation.
    #[must_use]
    pub fn new(processor_id: String) -> Self {
        Self {
            display_name: format!("processor-{}", &processor_id[..8.min(processor_id.len())]),
            processor_id,
            max_concurrent_jobs: std::thread::available_parallelism()
                .map(std::num::NonZero::get)
                .unwrap_or(4),
            supported_job_types: Vec::new(),
        }
    }
}
