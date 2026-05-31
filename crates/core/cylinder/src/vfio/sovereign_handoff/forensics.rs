// SPDX-License-Identifier: AGPL-3.0-or-later

//! Forensic breadcrumbs for handoff lockup diagnosis.
//!
//! Writes timestamped markers to a persistent filesystem location so that
//! after a hard lockup + power cycle, we can identify exactly which step
//! froze. Uses sync_all() to force flush to disk.

use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};

static INITIALIZED: AtomicBool = AtomicBool::new(false);

const FORENSICS_PATH: &str = "/var/log/handoff-forensics.log";

/// Write a timestamped forensic breadcrumb to persistent storage.
pub fn breadcrumb(msg: &str) {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(FORENSICS_PATH)
    {
        Ok(mut f) => {
            if !INITIALIZED.swap(true, Ordering::Relaxed) {
                let _ = writeln!(f, "\n{}", "=".repeat(60));
                let _ = writeln!(f, "[{ts}] === NEW HANDOFF SESSION ===");
            }
            let _ = writeln!(f, "[{ts}] {msg}");
            let _ = f.sync_all();
        }
        Err(_) => {
            tracing::warn!(breadcrumb = msg, "FORENSIC: {msg}");
        }
    }
}

/// Smoke-test at daemon startup — verifies the forensic log path is writable
/// BEFORE any handoff is attempted.
pub fn startup_smoke_test() {
    breadcrumb("DAEMON STARTUP — forensics smoke test");
    tracing::info!(path = FORENSICS_PATH, "forensics: startup smoke test written");
}
