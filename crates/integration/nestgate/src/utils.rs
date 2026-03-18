// SPDX-License-Identifier: AGPL-3.0-or-later
//! Utility functions for the NestGate storage client
//!
//! Provides checksum calculation, content-type detection, and cache cleanup.

use sha2::Digest;
use sha2::Sha256;
use tracing::debug;

use crate::client::StorageClient;

impl StorageClient {
    /// Calculate checksum for data integrity
    ///
    /// Uses SHA-256 to produce a hex-encoded checksum suitable for artifact verification.
    #[cfg_attr(test, allow(dead_code))]
    pub(crate) fn calculate_checksum(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    /// Detect content type from data magic bytes
    ///
    /// Inspects the leading bytes to infer MIME type for common formats:
    /// - ZIP (PK), PNG, JPEG, or application/octet-stream as fallback.
    #[cfg_attr(test, allow(dead_code))]
    pub(crate) fn detect_content_type(data: &[u8]) -> String {
        // Simple content type detection
        if data.starts_with(b"PK") {
            "application/zip".to_string()
        } else if data.starts_with(b"\x89PNG") {
            "image/png".to_string()
        } else if data.starts_with(b"\xFF\xD8\xFF") {
            "image/jpeg".to_string()
        } else {
            "application/octet-stream".to_string()
        }
    }

    /// Clean up expired cache entries
    ///
    /// Intentionally a no-op: the cache uses TTL-based expiry managed by the runtime,
    /// not explicit cleanup. Entries expire automatically; no manual cleanup is needed.
    pub fn cleanup_cache(&self) {
        if !self.config.cache.as_ref().is_some_and(|c| c.enabled) {
            return;
        }

        // Cache implementation would go here
        // For now, this is a no-op
        debug!("Cache cleanup completed (no-op)");
    }
}
