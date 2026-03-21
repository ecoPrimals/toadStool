// SPDX-License-Identifier: AGPL-3.0-only
//! Job receiver types

use tokio::sync::mpsc;

use super::message_types::SongbirdJobMessage;

// ============================================================================
// Job Receiver
// ============================================================================

pub struct JobReceiver {
    pub receiver: mpsc::Receiver<SongbirdJobMessage>,
}

impl JobReceiver {
    pub async fn receive(&mut self) -> Option<SongbirdJobMessage> {
        self.receiver.recv().await
    }
}

pub struct UniversalJobProcessor {
    pub processor_id: String,
}
