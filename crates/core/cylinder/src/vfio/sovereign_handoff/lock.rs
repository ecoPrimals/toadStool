// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashSet;
use std::sync::Mutex;

/// Per-BDF handoff concurrency guard. Only one handoff per device at a time.
pub(crate) static HANDOFF_LOCKS: Mutex<Option<HashSet<String>>> = Mutex::new(None);

/// RAII guard that releases the per-BDF handoff lock on drop. This ensures
/// the lock is freed even if the thread panics or the RPC timeout abandons
/// the blocking thread.
pub(crate) struct HandoffGuard {
    bdf: String,
}

impl HandoffGuard {
    pub(crate) fn acquire(bdf: &str) -> Result<Self, String> {
        let mut guard = HANDOFF_LOCKS.lock().map_err(|e| format!("lock poisoned: {e}"))?;
        let set = guard.get_or_insert_with(HashSet::new);
        if !set.insert(bdf.to_string()) {
            return Err(format!("handoff already in progress for {bdf}"));
        }
        Ok(Self { bdf: bdf.to_string() })
    }
}

impl Drop for HandoffGuard {
    fn drop(&mut self) {
        if let Ok(mut guard) = HANDOFF_LOCKS.lock()
            && let Some(set) = guard.as_mut()
        {
            set.remove(&self.bdf);
        }
    }
}
