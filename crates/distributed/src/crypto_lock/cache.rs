// SPDX-License-Identifier: AGPL-3.0-or-later
//! Performance caching for crypto lock system

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

use super::access_control::AccessResult;
use super::permissions::ExternalTarget;

/// Permission cache for performance optimization
///
/// Thread-safe in-memory cache keyed by external target.
/// Uses RwLock for read-heavy access patterns.
pub struct PermissionCache {
    inner: Arc<RwLock<HashMap<ExternalTarget, CachedResult>>>,
}

impl Default for PermissionCache {
    /// Same as [`PermissionCache::new`].
    fn default() -> Self {
        Self::new()
    }
}

impl PermissionCache {
    /// Creates an empty permission cache.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Look up cached permission result for the target, returning `None` for expired entries.
    pub async fn get(&self, target: &ExternalTarget) -> Option<CachedResult> {
        let guard = self
            .inner
            .read()
            .unwrap_or_else(|e| e.into_inner());
        guard
            .get(target)
            .filter(|cached| !cached.is_expired())
            .cloned()
    }

    /// Store permission result for the target
    pub async fn cache_result(&self, target: ExternalTarget, result: AccessResult) {
        let cached = CachedResult {
            result,
            cached_at: std::time::Instant::now(),
        };
        let mut guard = self
            .inner
            .write()
            .unwrap_or_else(|e| e.into_inner());
        guard.insert(target, cached);
    }

    /// Remove cached entry for the target (e.g. when permission is installed/updated)
    pub async fn invalidate_for_target(&self, target: &ExternalTarget) {
        let mut guard = self
            .inner
            .write()
            .unwrap_or_else(|e| e.into_inner());
        guard.remove(target);
    }
}

/// Cached permission result with TTL-based expiry.
#[derive(Clone)]
pub struct CachedResult {
    /// Last evaluated access result for the target.
    pub result: AccessResult,
    /// When the result was cached.
    cached_at: std::time::Instant,
}

/// Default cache entry TTL (5 minutes).
const CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(300);

impl CachedResult {
    /// Returns `true` if this entry has exceeded its TTL.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.cached_at.elapsed() > CACHE_TTL
    }
}
