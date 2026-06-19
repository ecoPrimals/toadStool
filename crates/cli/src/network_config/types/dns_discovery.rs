// SPDX-License-Identifier: AGPL-3.0-or-later

//! DNS service discovery configuration types.
//!
//! Domain fields use **capability names** (coordination, security, storage, ai\_processing)
//! per `CAPABILITY_BASED_DISCOVERY_STANDARD.md` v1.2.
//!
//! **Serde alias deprecation:** `#[serde(alias = …)]` on [`ServiceDomainsConfig`] accepts
//! legacy primal field names (`songbird`, `beardog`, `nestgate`, `squirrel`, `toadstool`)
//! in on-disk config. These aliases are deprecated and will be removed in a future release;
//! migrate configs to capability field names.

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
    /// Search domains (override defaults from `orchestration_default_network_config` /
    /// `TOADSTOOL_DNS_SEARCH_DOMAINS` when building programmatic defaults)
    pub search_domains: Vec<String>,
    /// Service domains
    pub service_domains: ServiceDomainsConfig,
    /// DNS resolution timeout
    pub resolution_timeout: Duration,
    /// DNS cache configuration
    pub cache: DnsCacheConfig,
}

/// Capability-domain DNS configuration.
///
/// Fields are named by **capability domain**, not primal identity, per
/// `CAPABILITY_BASED_DISCOVERY_STANDARD.md` v1.2. Serde aliases preserve
/// backward compatibility with primal-named config files; aliases are deprecated
/// and will be removed in a future release.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDomainsConfig {
    /// Compute capability domain (self — toadStool)
    #[serde(alias = "toadstool")]
    pub compute: String,
    /// Coordination / orchestration capability domain
    #[serde(alias = "songbird")]
    pub coordination: String,
    /// Security / crypto capability domain
    #[serde(alias = "beardog")]
    pub security: String,
    /// Storage capability domain
    #[serde(alias = "nestgate")]
    pub storage: String,
    /// AI processing capability domain
    #[serde(alias = "squirrel")]
    pub ai_processing: String,
    /// BiomeOS domain
    pub biomeos: String,
}

impl ServiceDomainsConfig {
    /// Create capability-domain config from environment or defaults.
    ///
    /// Primary env vars use capability names (`COORDINATION_DOMAIN`, etc.);
    /// legacy primal-name env vars (`SONGBIRD_DOMAIN`, etc.) are accepted as
    /// fallbacks for backward compatibility.
    pub fn from_env() -> Self {
        use toadstool_common::interned_strings::socket_env;

        let base_domain = std::env::var(socket_env::TOADSTOOL_BASE_DOMAIN)
            .unwrap_or_else(|_| "primal.local".to_string());

        #[expect(deprecated, reason = "legacy domain env-vars kept for migration")]
        Self {
            compute: std::env::var(socket_env::COMPUTE_DOMAIN)
                .or_else(|_| std::env::var(socket_env::TOADSTOOL_DOMAIN))
                .unwrap_or_else(|_| format!("compute.{base_domain}")),
            coordination: std::env::var(socket_env::COORDINATION_DOMAIN)
                .or_else(|_| std::env::var(socket_env::SONGBIRD_DOMAIN))
                .unwrap_or_else(|_| format!("coordination.{base_domain}")),
            security: std::env::var(socket_env::SECURITY_DOMAIN)
                .or_else(|_| std::env::var(socket_env::BEARDOG_DOMAIN))
                .unwrap_or_else(|_| format!("security.{base_domain}")),
            storage: std::env::var(socket_env::STORAGE_DOMAIN)
                .or_else(|_| std::env::var(socket_env::NESTGATE_DOMAIN))
                .unwrap_or_else(|_| format!("storage.{base_domain}")),
            ai_processing: std::env::var(socket_env::AI_PROCESSING_DOMAIN)
                .or_else(|_| std::env::var(socket_env::SQUIRREL_DOMAIN))
                .unwrap_or_else(|_| format!("ai.{base_domain}")),
            biomeos: std::env::var(socket_env::BIOMEOS_DOMAIN)
                .unwrap_or_else(|_| format!("biomeos.{base_domain}")),
        }
    }

    /// Create with a custom base domain using capability-based subdomains.
    pub fn with_base_domain(base_domain: &str) -> Self {
        Self {
            compute: format!("compute.{base_domain}"),
            coordination: format!("coordination.{base_domain}"),
            security: format!("security.{base_domain}"),
            storage: format!("storage.{base_domain}"),
            ai_processing: format!("ai.{base_domain}"),
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
