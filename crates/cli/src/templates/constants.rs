// SPDX-License-Identifier: AGPL-3.0-or-later
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
    /// Science-focused biome template
    pub const SCIENCE: &str = "science-biome";
    /// AI research biome template
    pub const AI_RESEARCH: &str = "ai-research-biome";
    /// Quantum computing biome template
    pub const QUANTUM: &str = "quantum-biome";
    /// Genomics research biome template
    pub const GENOMICS: &str = "genomics-biome";
    /// Computer vision biome template
    pub const VISION: &str = "vision-biome";
    /// Distributed compute biome template
    pub const DISTRIBUTED: &str = "distributed-biome";
    /// Sovereign science biome template
    pub const SOVEREIGN: &str = "sovereign-biome";
    /// Basic minimal biome template
    pub const BASIC: &str = "basic-biome";
    /// Development biome template
    pub const DEVELOPMENT: &str = "dev-biome";
}

/// Service names used in templates
///
/// Prefer capability ids from [`toadstool_common::interned_strings::capabilities`]
/// (`storage`, `crypto`, `coordination`, …) in new manifests — not legacy route labels
/// ([`toadstool_common::interned_strings::primals`]).
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

    /// Storage capability id (container image tag / placeholder — discover by capability at runtime)
    pub const STORAGE: &str = toadstool_common::interned_strings::capabilities::STORAGE;

    /// Crypto / security (PKI) capability id
    pub const CRYPTO: &str = toadstool_common::interned_strings::capabilities::CRYPTO;

    /// Coordination capability id
    pub const COORDINATION: &str = toadstool_common::interned_strings::capabilities::COORDINATION;

    /// Legacy template symbol — evolved to capability id [`STORAGE`] (not the legacy route label
    /// [`toadstool_common::interned_strings::primals::LEGACY_STORAGE_LABEL`]).
    pub const NESTGATE: &str = STORAGE;

    /// Legacy template symbol — evolved to capability id [`CRYPTO`].
    pub const BEARDOG: &str = CRYPTO;

    /// Legacy template symbol — evolved to capability id [`COORDINATION`].
    pub const SONGBIRD: &str = COORDINATION;
}

/// Docker registries
pub mod registries {
    /// Docker Hub registry
    pub const DOCKER_HUB: &str = "docker.io";
    /// Sovereign Science ecosystem registry
    pub const SOVEREIGN_SCIENCE: &str = "registry.ecosystem.sovereignscience.org";
    /// GitHub Container Registry
    pub const GITHUB: &str = "ghcr.io";
}

/// Common version tags
pub mod versions {
    /// Latest version tag
    pub const LATEST: &str = "latest";
    /// Stable version tag
    pub const STABLE: &str = "stable";
    /// Unknown version placeholder
    pub const UNKNOWN: &str = "unknown";
}

/// Common environment variable names
pub mod env_vars {
    /// Enable JupyterLab (vs classic notebook)
    pub const JUPYTER_ENABLE_LAB: &str = "JUPYTER_ENABLE_LAB";
    /// PostgreSQL password
    pub const POSTGRES_PASSWORD: &str = "POSTGRES_PASSWORD";
    /// Redis password
    pub const REDIS_PASSWORD: &str = "REDIS_PASSWORD";
}

/// Common commands used in templates
pub mod commands {
    /// Health check command
    pub const HEALTH: &str = "health";
    /// Status check command
    pub const STATUS: &str = "status";
    /// Curl for HTTP checks
    pub const CURL: &str = "curl";
}

/// Resource sizes for templates
pub mod resource_sizes {
    /// 16 GB
    pub const GB_16: &str = "16GB";
    /// 32 GB
    pub const GB_32: &str = "32GB";
    /// 64 GB
    pub const GB_64: &str = "64GB";
    /// 100 GB
    pub const GB_100: &str = "100GB";
    /// 500 GB
    pub const GB_500: &str = "500GB";
    /// 1 TB
    pub const TB_1: &str = "1TB";
}

/// Docker images used in templates
pub mod images {
    /// Jupyter SciPy notebook image
    pub const JUPYTER_SCIPY: &str = "jupyter/scipy-notebook";
    /// Jupyter data science notebook image
    pub const JUPYTER_DATASCIENCE: &str = "jupyter/datascience-notebook";
    /// PostgreSQL image
    pub const POSTGRES: &str = "postgres";
    /// Redis image
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
