// SPDX-License-Identifier: AGPL-3.0-or-later

//! DNS service discovery configuration types.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use toadstool_common::config_bases::CacheConfig;

/// DNS service discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsDiscoveryConfig {
    /// Enable DNS discovery
    pub enabled: bool,
    /// DNS servers
    pub dns_servers: Vec<String>,
    /// Search domains
    pub search_domains: Vec<String>,
    /// Service domains
    pub service_domains: ServiceDomainsConfig,
    /// DNS resolution timeout
    pub resolution_timeout: Duration,
    /// DNS cache configuration
    pub cache: DnsCacheConfig,
}

/// Service domains configuration
///
/// **DEPRECATED**: This configuration uses hardcoded primal names.
/// New code should use capability-based discovery instead.
///
/// For backward compatibility, this can be constructed from environment
/// variables or a base domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDomainsConfig {
    /// ToadStool domain
    pub toadstool: String,
    /// Songbird domain (DEPRECATED: use ORCHESTRATION capability)
    pub songbird: String,
    /// BearDog domain (DEPRECATED: use PKI capability)
    pub beardog: String,
    /// NestGate domain (DEPRECATED: use STORAGE capability)
    pub nestgate: String,
    /// Squirrel domain (DEPRECATED: use AI_PROCESSING capability)
    pub squirrel: String,
    /// BiomeOS domain
    pub biomeos: String,
}

impl ServiceDomainsConfig {
    /// Create service domains from environment or defaults
    ///
    /// Reads TOADSTOOL_BASE_DOMAIN (default: "primal.local")
    /// and constructs service-specific domains.
    ///
    /// Individual services can be overridden with:
    /// - TOADSTOOL_DOMAIN
    /// - SONGBIRD_DOMAIN
    /// - BEARDOG_DOMAIN
    /// - NESTGATE_DOMAIN
    /// - SQUIRREL_DOMAIN
    /// - BIOMEOS_DOMAIN
    pub fn from_env() -> Self {
        let base_domain =
            std::env::var("TOADSTOOL_BASE_DOMAIN").unwrap_or_else(|_| "primal.local".to_string());

        Self {
            toadstool: std::env::var("TOADSTOOL_DOMAIN")
                .unwrap_or_else(|_| format!("toadstool.{base_domain}")),
            songbird: std::env::var("SONGBIRD_DOMAIN")
                .unwrap_or_else(|_| format!("songbird.{base_domain}")),
            beardog: std::env::var("BEARDOG_DOMAIN")
                .unwrap_or_else(|_| format!("beardog.{base_domain}")),
            nestgate: std::env::var("NESTGATE_DOMAIN")
                .unwrap_or_else(|_| format!("nestgate.{base_domain}")),
            squirrel: std::env::var("SQUIRREL_DOMAIN")
                .unwrap_or_else(|_| format!("squirrel.{base_domain}")),
            biomeos: std::env::var("BIOMEOS_DOMAIN")
                .unwrap_or_else(|_| format!("biomeos.{base_domain}")),
        }
    }

    /// Create with a custom base domain
    pub fn with_base_domain(base_domain: &str) -> Self {
        Self {
            toadstool: format!("toadstool.{base_domain}"),
            songbird: format!("songbird.{base_domain}"),
            beardog: format!("beardog.{base_domain}"),
            nestgate: format!("nestgate.{base_domain}"),
            squirrel: format!("squirrel.{base_domain}"),
            biomeos: format!("biomeos.{base_domain}"),
        }
    }
}

/// DNS cache configuration
///
/// Uses base `CacheConfig` with DNS-specific semantics.
/// The base configuration provides standard caching parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsCacheConfig {
    /// Base cache configuration (enabled, ttl, max_entries, negative_ttl)
    #[serde(flatten)]
    pub base: CacheConfig,
}
