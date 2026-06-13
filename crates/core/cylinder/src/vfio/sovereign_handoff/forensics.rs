// SPDX-License-Identifier: AGPL-3.0-or-later

//! Forensic breadcrumbs for handoff lockup diagnosis.
//!
//! Writes timestamped markers to a persistent filesystem location so that
//! after a hard lockup + power cycle, we can identify exactly which step
//! froze. Uses sync_all() to force flush to disk.

use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

static INITIALIZED: AtomicBool = AtomicBool::new(false);

const DEFAULT_FORENSICS_PATH: &str = "/var/log/handoff-forensics.log";

fn forensics_path() -> PathBuf {
    std::env::var("TOADSTOOL_FORENSICS_LOG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_FORENSICS_PATH))
}

/// Write a timestamped forensic breadcrumb to persistent storage.
pub fn breadcrumb(msg: &str) {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let path = forensics_path();
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
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
    let path = forensics_path();
    breadcrumb("DAEMON STARTUP — forensics smoke test");
    tracing::info!(path = %path.display(), "forensics: startup smoke test written");
}
