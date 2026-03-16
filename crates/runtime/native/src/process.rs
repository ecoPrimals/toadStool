// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::process::Child;
use tokio::sync::RwLock;
use tracing::warn;
use uuid::Uuid;

#[derive(Debug)]
pub(crate) struct ProcessHandle {
    pub(crate) child: Option<Child>,
    pub(crate) _start_time: Instant,
    pub(crate) _workload_id: String,
    pub(crate) _executable_path: PathBuf,
}

pub(crate) async fn cleanup_process(
    processes: &Arc<RwLock<HashMap<Uuid, ProcessHandle>>>,
    execution_id: &Uuid,
) {
    let mut processes_guard = processes.write().await;
    if let Some(mut process_handle) = processes_guard.remove(execution_id) {
        if let Some(mut child) = process_handle.child.take() {
            if let Err(e) = child.kill().await {
                warn!("Failed to kill process {}: {}", execution_id, e);
            }
        }
    }
}
