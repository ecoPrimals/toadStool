// SPDX-License-Identifier: AGPL-3.0-only
//! Performance caching for crypto lock system

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

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
    fn default() -> Self {
        Self::new()
    }
}

impl PermissionCache {
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Look up cached permission result for the target
    pub async fn get(&self, target: &ExternalTarget) -> Option<CachedResult> {
        let guard = self.inner.read().await;
        guard.get(target).cloned()
    }

    /// Store permission result for the target
    pub async fn cache_result(&self, target: ExternalTarget, result: AccessResult) {
        let cached = CachedResult { result };
        let mut guard = self.inner.write().await;
        guard.insert(target, cached);
    }

    /// Remove cached entry for the target (e.g. when permission is installed/updated)
    pub async fn invalidate_for_target(&self, target: &ExternalTarget) {
        let mut guard = self.inner.write().await;
        guard.remove(target);
    }
}

/// Cached permission result
#[derive(Clone)]
pub struct CachedResult {
    pub result: AccessResult,
}

impl CachedResult {
    #[must_use]
    pub const fn is_expired(&self) -> bool {
        false
    }
}
