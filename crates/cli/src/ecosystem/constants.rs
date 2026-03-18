// SPDX-License-Identifier: AGPL-3.0-or-later
//! 🎯 Zero-Copy Constants - Ecosystem Service Names & Capabilities
//!
//! Central location for static string constants to avoid repeated allocations.
//!
//! **Zero-Copy Pattern**:
//! - Define strings as `&'static str` constants
//! - Only allocate (`to_string()`) when ownership is required
//! - Use references everywhere else
//!
//! **Impact**: ~50-100 fewer allocations across ecosystem discovery

/// Service names — capability-based constants from `toadstool_common::interned_strings::capabilities`.
/// Prefer `interned_strings::capabilities::*` directly in new code.
pub mod service_names {
    use toadstool_common::constants::PRIMAL_NAME;
    use toadstool_common::interned_strings::capabilities;

    /// Coordination/discovery capability (Songbird)
    pub const SONGBIRD: &str = capabilities::COORDINATION;
    /// Crypto/security capability (BearDog)
    pub const BEARDOG: &str = capabilities::CRYPTO;
    /// Storage capability (NestGate)
    pub const NESTGATE: &str = capabilities::STORAGE;
    /// Intelligence/AI capability (Squirrel)
    pub const SQUIRREL: &str = capabilities::INTELLIGENCE;
    /// Compute capability (ToadStool)
    pub const TOADSTOOL: &str = PRIMAL_NAME;

    /// All known capability identifiers as slice
    pub const ALL: &[&str] = &[SONGBIRD, BEARDOG, NESTGATE, SQUIRREL, TOADSTOOL];
}

/// Capability categories - Use these for capability matching
pub mod capability_categories {
    /// Network and coordination capabilities
    pub const NETWORK: &str = "network";

    /// Cryptographic capabilities
    pub const CRYPTO: &str = "crypto";

    /// Storage and data management capabilities
    pub const STORAGE: &str = "storage";

    /// Orchestration capabilities
    pub const ORCHESTRATION: &str = "orchestration";

    /// Compute capabilities
    pub const COMPUTE: &str = "compute";

    /// All capability categories
    pub const ALL: &[&str] = &[NETWORK, CRYPTO, STORAGE, ORCHESTRATION, COMPUTE];
}

/// System-level config paths (last-resort fallback after XDG/home paths)
pub mod paths {
    /// System-wide services config (fallback when user config not found)
    pub const SYSTEM_SERVICES_CONFIG: &str = "/etc/toadstool/services.toml";
}

/// Common string constants
pub mod common {
    /// Unknown version string
    pub const UNKNOWN_VERSION: &str = "unknown";

    /// Latest version tag
    pub const LATEST: &str = "latest";

    /// Text output format
    pub const FORMAT_TEXT: &str = "text";

    /// JSON output format
    pub const FORMAT_JSON: &str = "json";

    /// YAML output format
    pub const FORMAT_YAML: &str = "yaml";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_names_defined() {
        assert_eq!(service_names::SONGBIRD, "coordination");
        assert_eq!(service_names::BEARDOG, "crypto");
        assert_eq!(service_names::NESTGATE, "storage");
        assert_eq!(service_names::SQUIRREL, "intelligence");
        assert_eq!(service_names::TOADSTOOL, "toadstool");
    }

    #[test]
    fn test_service_names_all() {
        assert_eq!(service_names::ALL.len(), 5);
        assert!(service_names::ALL.contains(&"coordination"));
        assert!(service_names::ALL.contains(&"crypto"));
    }

    #[test]
    fn test_capability_categories() {
        assert_eq!(capability_categories::NETWORK, "network");
        assert_eq!(capability_categories::CRYPTO, "crypto");
        assert_eq!(capability_categories::STORAGE, "storage");
    }

    #[test]
    fn test_common_constants() {
        assert_eq!(common::UNKNOWN_VERSION, "unknown");
        assert_eq!(common::LATEST, "latest");
    }
}
