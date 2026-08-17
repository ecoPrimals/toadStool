// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Child;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Instant;
use tracing::warn;
use uuid::Uuid;

#[derive(Debug)]
pub struct ProcessHandle {
    pub(crate) child: Option<Child>,
    pub(crate) _start_time: Instant,
    pub(crate) _workload_id: String,
    pub(crate) _executable_path: PathBuf,
}

pub async fn cleanup_process(
    processes: &Arc<RwLock<HashMap<Uuid, ProcessHandle>>>,
    execution_id: &Uuid,
) {
    let mut processes_guard = processes
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(mut process_handle) = processes_guard.remove(execution_id)
        && let Some(mut child) = process_handle.child.take()
        && let Err(e) = child.kill()
    {
        warn!("Failed to kill process {}: {}", execution_id, e);
    }
}
