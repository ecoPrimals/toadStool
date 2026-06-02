// SPDX-License-Identifier: AGPL-3.0-or-later
//! Distributed storage mount configuration types.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Distributed storage mount configuration (capability-based)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageMount {
    /// Dataset name in the storage service
    pub dataset_name: String,
    /// Local mount path
    pub mount_point: PathBuf,
    /// storage endpoint URL
    pub endpoint: String,
    /// ZFS dataset name (if applicable)
    pub zfs_dataset: Option<String>,
    /// Access mode (read, write, admin)
    pub access_mode: String,
    /// Encryption key (if encrypted)
    pub encryption_key: Option<String>,
}
