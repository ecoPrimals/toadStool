//! # Legacy Compatibility
//!
//! **⚠️  DEPRECATED CODE - DO NOT USE IN NEW CODE**
//!
//! This module contains deprecated methods kept for backward compatibility.
//! All code here is scheduled for removal in future versions.

#![allow(dead_code, unused_variables, deprecated, unused_imports)]

use tracing::{info, warn};

use crate::ToadStoolResult;

use super::types::ServiceInstance;

/// Legacy discovery methods (DEPRECATED)
pub struct LegacyDiscovery;

/// Legacy primal status (DEPRECATED)
#[derive(Debug, Clone)]
pub struct LegacyPrimalStatus {
    /// Primal name
    pub name: String,
    /// Status message  
    pub status: String,
}

/// Legacy primal info (DEPRECATED)
#[derive(Debug, Clone)]
pub struct LegacyPrimalInfo {
    /// Primal name
    pub name: String,
    /// Version
    pub version: String,
}

impl LegacyDiscovery {
    /// Discover services via multicast (LEGACY)
    ///
    /// # ⚠️ Deprecated
    /// This method is deprecated in favor of capability-based discovery.
    /// Use `find_service_by_capability()` instead.
    #[deprecated(since = "0.4.0", note = "Use capability-based discovery instead")]
    pub async fn discover_via_multicast() -> ToadStoolResult<Vec<ServiceInstance>> {
        info!("🔍 Attempting multicast discovery (LEGACY METHOD)");
        warn!("⚠️  Multicast discovery is deprecated and no longer functional");
        Ok(Vec::new())
    }

    /// Discover services via DNS (Legacy - uses hardcoded DNS names)
    ///
    /// # ⚠️ Legacy Pattern
    /// This method is deprecated and uses hardcoded port assumptions.
    /// Prefer capability-based discovery for modern deployments.
    #[deprecated(
        since = "0.3.0",
        note = "Use capability-based discovery methods instead of hardcoded DNS + port scanning"
    )]
    pub async fn discover_via_dns() -> ToadStoolResult<Vec<ServiceInstance>> {
        info!("🔍 Discovering primals via DNS (legacy method)");
        warn!("⚠️  DNS discovery is deprecated and no longer functional");
        Ok(Vec::new())
    }

    /// Discover services via local network scan (Legacy - uses port scanning)
    ///
    /// # ⚠️ Legacy Pattern
    /// This method performs port scanning which is:
    /// - Inefficient (tries many ports sequentially)
    /// - Intrusive (network scanning can trigger security alerts)
    /// - Hardcoded (assumes specific port numbers)
    ///
    /// Modern deployments should use capability-based service discovery instead.
    #[deprecated(
        since = "0.3.0",
        note = "Port scanning is inefficient and intrusive. Use capability-based discovery instead."
    )]
    pub async fn discover_via_local_scan() -> ToadStoolResult<Vec<ServiceInstance>> {
        info!("🔍 Discovering primals via local network scan (legacy method)");
        warn!("⚠️  Port scanning is deprecated and no longer functional");
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_legacy_multicast_deprecated() {
        let result = LegacyDiscovery::discover_via_multicast().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_legacy_dns_deprecated() {
        let result = LegacyDiscovery::discover_via_dns().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_legacy_local_scan_deprecated() {
        let result = LegacyDiscovery::discover_via_local_scan().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }
}
