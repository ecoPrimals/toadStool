// SPDX-License-Identifier: AGPL-3.0-or-later
//! In-memory policy cache entries and TTL checks.

use std::time::{Duration, SystemTime};

use crate::types::{PolicyManagerConfig, SecurityPolicy};

/// Cached policy with LRU metadata.
#[derive(Debug, Clone)]
pub(crate) struct CachedPolicy {
    pub(crate) policy: SecurityPolicy,
    pub(crate) cached_at: SystemTime,
    pub(crate) access_count: u64,
    pub(crate) last_accessed: SystemTime,
}

impl CachedPolicy {
    pub(crate) fn touch(&mut self) {
        self.access_count += 1;
        self.last_accessed = SystemTime::now();
    }
}

pub(crate) fn is_cache_valid(cached_policy: &CachedPolicy, config: &PolicyManagerConfig) -> bool {
    if !config.cache_enabled {
        return false;
    }

    let cache_duration = Duration::from_secs(config.cache_ttl_hours * 3600);
    cached_policy.cached_at.elapsed().unwrap_or(Duration::MAX) < cache_duration
}
