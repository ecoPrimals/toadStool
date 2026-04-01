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

/// Placeholder processor identity for universal job handling.
pub struct UniversalJobProcessor {
    /// Stable id for this processor instance.
    pub processor_id: String,
}
