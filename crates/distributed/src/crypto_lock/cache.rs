//! Performance caching for crypto lock system

use super::access_control::AccessResult;
use super::permissions::ExternalTarget;

/// Permission cache for performance optimization
pub struct PermissionCache;

impl Default for PermissionCache {
    fn default() -> Self {
        Self::new()
    }
}

impl PermissionCache {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    pub async fn get(&self, _target: &ExternalTarget) -> Option<CachedResult> {
        None
    }

    pub async fn cache_result(&self, _target: ExternalTarget, _result: AccessResult) {}

    pub async fn invalidate_for_target(&self, _target: &ExternalTarget) {}
}

/// Cached permission result
pub struct CachedResult {
    pub result: AccessResult,
}

impl CachedResult {
    #[must_use]
    pub const fn is_expired(&self) -> bool {
        false
    }
}
