// SPDX-License-Identifier: AGPL-3.0-only
//! 🎯 Zero-Copy Constants - Template Strings
//!
//! Central location for template string constants to avoid repeated allocations.
//!
//! **Zero-Copy Pattern**:
//! - Define strings as `&'static str` constants
//! - Only allocate (`to_string()`) when ownership is required
//! - Use references everywhere else
//!
//! **Impact**: ~50-70 fewer allocations across template generation

/// Template names
pub mod template_names {
    pub const SCIENCE: &str = "science-biome";
    pub const AI_RESEARCH: &str = "ai-research-biome";
    pub const QUANTUM: &str = "quantum-biome";
    pub const GENOMICS: &str = "genomics-biome";
    pub const VISION: &str = "vision-biome";
    pub const DISTRIBUTED: &str = "distributed-biome";
    pub const SOVEREIGN: &str = "sovereign-biome";
    pub const BASIC: &str = "basic-biome";
    pub const DEVELOPMENT: &str = "dev-biome";
}

/// Service names used in templates
///
/// ⚠️ **DEPRECATION NOTICE**: Primal-specific names (nestgate, beardog, songbird) are deprecated.
/// Use capability-based discovery instead:
/// - Instead of "nestgate" → use capability "storage"
/// - Instead of "beardog" → use capability "security"
/// - Instead of "songbird" → use capability "coordination"
///
/// Generic service names (jupyter, postgres, redis) remain valid as these are
/// third-party services, not ecoPrimals.
pub mod service_names {
    /// Jupyter notebook service (third-party, not a primal)
    pub const JUPYTER: &str = "jupyter";

    /// PostgreSQL database (third-party, not a primal)
    pub const POSTGRES: &str = "postgres";

    /// Redis cache (third-party, not a primal)
    pub const REDIS: &str = "redis";

    /// ⚠️ DEPRECATED: Use capability "storage" instead
    #[deprecated(
        since = "0.4.0",
        note = "Use capability-based discovery: discover_capability(\"storage\") instead of hardcoding \"nestgate\""
    )]
    #[allow(deprecated)]
    pub const NESTGATE: &str = toadstool_common::interned_strings::primals::NESTGATE;

    /// ⚠️ DEPRECATED: Use capability "security" instead
    #[deprecated(
        since = "0.4.0",
        note = "Use capability-based discovery: discover_capability(\"security\") instead of hardcoding \"beardog\""
    )]
    #[allow(deprecated)]
    pub const BEARDOG: &str = toadstool_common::interned_strings::primals::BEARDOG;

    /// ⚠️ DEPRECATED: Use capability "coordination" instead
    #[deprecated(
        since = "0.4.0",
        note = "Use capability-based discovery: discover_capability(\"coordination\") instead of hardcoding \"songbird\""
    )]
    #[allow(deprecated)]
    pub const SONGBIRD: &str = toadstool_common::interned_strings::primals::SONGBIRD;
}

/// Docker registries
pub mod registries {
    pub const DOCKER_HUB: &str = "docker.io";
    pub const SOVEREIGN_SCIENCE: &str = "registry.ecosystem.sovereignscience.org";
    pub const GITHUB: &str = "ghcr.io";
}

/// Common version tags
pub mod versions {
    pub const LATEST: &str = "latest";
    pub const STABLE: &str = "stable";
    pub const UNKNOWN: &str = "unknown";
}

/// Common environment variable names
pub mod env_vars {
    pub const JUPYTER_ENABLE_LAB: &str = "JUPYTER_ENABLE_LAB";
    pub const POSTGRES_PASSWORD: &str = "POSTGRES_PASSWORD";
    pub const REDIS_PASSWORD: &str = "REDIS_PASSWORD";
}

/// Common commands
pub mod commands {
    pub const HEALTH: &str = "health";
    pub const STATUS: &str = "status";
    pub const CURL: &str = "curl";
}

/// Resource sizes
pub mod resource_sizes {
    pub const GB_16: &str = "16GB";
    pub const GB_32: &str = "32GB";
    pub const GB_64: &str = "64GB";
    pub const GB_100: &str = "100GB";
    pub const GB_500: &str = "500GB";
    pub const TB_1: &str = "1TB";
}

/// Docker images
pub mod images {
    pub const JUPYTER_SCIPY: &str = "jupyter/scipy-notebook";
    pub const JUPYTER_DATASCIENCE: &str = "jupyter/datascience-notebook";
    pub const POSTGRES: &str = "postgres";
    pub const REDIS: &str = "redis";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_names() {
        assert_eq!(template_names::SCIENCE, "science-biome");
        assert_eq!(template_names::AI_RESEARCH, "ai-research-biome");
        assert_eq!(template_names::QUANTUM, "quantum-biome");
    }

    #[test]
    fn test_service_names() {
        assert_eq!(service_names::JUPYTER, "jupyter");
        assert_eq!(service_names::POSTGRES, "postgres");
        assert_eq!(service_names::REDIS, "redis");
    }

    #[test]
    fn test_registries() {
        assert_eq!(registries::DOCKER_HUB, "docker.io");
        assert_eq!(
            registries::SOVEREIGN_SCIENCE,
            "registry.ecosystem.sovereignscience.org"
        );
    }

    #[test]
    fn test_versions() {
        assert_eq!(versions::LATEST, "latest");
        assert_eq!(versions::STABLE, "stable");
    }

    #[test]
    fn test_resource_sizes() {
        assert_eq!(resource_sizes::GB_16, "16GB");
        assert_eq!(resource_sizes::GB_100, "100GB");
        assert_eq!(resource_sizes::TB_1, "1TB");
    }
}
