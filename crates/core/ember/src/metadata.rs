// SPDX-License-Identifier: AGPL-3.0-only

//! Opaque per-device metadata persistence.
//!
//! [`MetadataStore`] holds arbitrary key/value pairs for a held device.
//! This is the generalization of coralReef's `ring_meta` — instead of
//! GPU-specific ring/mailbox state, any hardware class can persist
//! whatever opaque blobs it needs across personality swaps.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Opaque per-device metadata store.
///
/// Keys are dot-separated namespaces (e.g. `"fecs.cpuctl"`, `"ring.gpfifo"`,
/// `"usb.descriptor"`, `"npu.firmware_version"`). Values are JSON blobs.
///
/// The store is versioned — each mutation increments a generation counter
/// so consumers can detect stale reads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataStore {
    entries: BTreeMap<String, serde_json::Value>,
    generation: u64,
}

impl MetadataStore {
    /// Create an empty metadata store.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            generation: 0,
        }
    }

    /// Get a metadata value by key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.entries.get(key)
    }

    /// Set a metadata value, returning the previous value if any.
    pub fn set(
        &mut self,
        key: impl Into<String>,
        value: serde_json::Value,
    ) -> Option<serde_json::Value> {
        self.generation += 1;
        self.entries.insert(key.into(), value)
    }

    /// Remove a metadata key, returning its value if present.
    pub fn remove(&mut self, key: &str) -> Option<serde_json::Value> {
        let removed = self.entries.remove(key);
        if removed.is_some() {
            self.generation += 1;
        }
        removed
    }

    /// Current generation (mutation counter).
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// Number of stored entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the store is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate over all entries.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &serde_json::Value)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Merge another store into this one (last-writer-wins per key).
    pub fn merge(&mut self, other: &Self) {
        for (k, v) in &other.entries {
            self.entries.insert(k.clone(), v.clone());
        }
        self.generation += 1;
    }

    /// Snapshot the entire store as a JSON object.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails (should not happen for valid JSON values).
    pub fn snapshot(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }

    /// Restore from a snapshot.
    ///
    /// # Errors
    ///
    /// Returns an error if the snapshot is not a valid `MetadataStore`.
    pub fn restore(snapshot: &serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(snapshot.clone())
    }
}

impl Default for MetadataStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_store() {
        let store = MetadataStore::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
        assert_eq!(store.generation(), 0);
    }

    #[test]
    fn set_get_remove() {
        let mut store = MetadataStore::new();
        store.set("ring.gpfifo", serde_json::json!({"size": 4096}));
        assert_eq!(store.len(), 1);
        assert_eq!(store.generation(), 1);
        assert!(store.get("ring.gpfifo").is_some());

        store.remove("ring.gpfifo");
        assert!(store.get("ring.gpfifo").is_none());
        assert_eq!(store.generation(), 2);
    }

    #[test]
    fn snapshot_restore_roundtrip() {
        let mut store = MetadataStore::new();
        store.set("fw.version", serde_json::json!("1.2.3"));
        store.set("bar0.offset", serde_json::json!(0x100));

        let snap = store.snapshot().expect("snapshot");
        let restored = MetadataStore::restore(&snap).expect("restore");
        assert_eq!(restored.len(), 2);
        assert_eq!(restored.get("fw.version"), store.get("fw.version"));
    }

    #[test]
    fn merge_last_writer_wins() {
        let mut a = MetadataStore::new();
        a.set("key", serde_json::json!("old"));

        let mut b = MetadataStore::new();
        b.set("key", serde_json::json!("new"));

        a.merge(&b);
        assert_eq!(a.get("key").unwrap(), &serde_json::json!("new"));
    }
}
