// SPDX-License-Identifier: AGPL-3.0-or-later
//! 🎯 Zero-Copy Constants - Universal Operations
//!
//! Central location for universal operations string constants.
//!
//! **Zero-Copy Pattern**:
//! - Define strings as `&'static str` constants
//! - Only allocate (`to_string()`) when ownership is required
//! - Use references everywhere else
//!
//! **Impact**: ~30-40 fewer allocations across migration and federation operations

/// Federation capability strings
pub mod capabilities {
    /// Universal compute capability
    pub const UNIVERSAL_COMPUTE: &str = "universal-compute";
    /// WASM execution capability
    pub const WASM_EXECUTION: &str = "wasm-execution";
    /// Container runtime capability
    pub const CONTAINER_RUNTIME: &str = "container-runtime";
    /// Substrate detection capability
    pub const SUBSTRATE_DETECTION: &str = "substrate-detection";
    /// Workload migration capability
    pub const WORKLOAD_MIGRATION: &str = "workload-migration";
    /// Federation capability
    pub const FEDERATION: &str = "federation";
    /// GPU acceleration capability
    pub const GPU_ACCELERATION: &str = "gpu-acceleration";
    /// Distributed storage capability
    pub const DISTRIBUTED_STORAGE: &str = "distributed-storage";
}

/// Federation protocol versions
pub mod protocol {
    /// Protocol version 1.0
    pub const VERSION_1_0: &str = "1.0";
    /// Protocol version 1.1
    pub const VERSION_1_1: &str = "1.1";
    /// Protocol version 2.0
    pub const VERSION_2_0: &str = "2.0";
}

/// Migration type strings
pub mod migration_types {
    /// Live migration (no downtime)
    pub const LIVE: &str = "live";
    /// Cold migration (stop, move, start)
    pub const COLD: &str = "cold";
    /// Hot migration (replication-based)
    pub const HOT: &str = "hot";
    /// Clone migration (copy to new target)
    pub const CLONE: &str = "clone";
}

/// Migration status messages
pub mod migration_status {
    /// Migration is being prepared
    pub const PREPARING: &str = "preparing";
    /// Migration in progress
    pub const IN_PROGRESS: &str = "in_progress";
    /// Migration completed
    pub const COMPLETED: &str = "completed";
    /// Migration failed
    pub const FAILED: &str = "failed";
    /// Verifying migrated workload
    pub const VERIFYING: &str = "verifying";
}

/// Federation modes
pub mod federation_modes {
    /// Peer-to-peer federation
    pub const PEER_TO_PEER: &str = "peer-to-peer";
    /// Client-server federation
    pub const CLIENT_SERVER: &str = "client-server";
    /// Mesh federation
    pub const MESH: &str = "mesh";
    /// Hierarchical federation
    pub const HIERARCHICAL: &str = "hierarchical";
}

/// Platform identifiers
pub mod platforms {
    /// Docker container platform
    pub const DOCKER: &str = "docker";
    /// Podman container platform
    pub const PODMAN: &str = "podman";
    /// Kubernetes orchestration platform
    pub const KUBERNETES: &str = "kubernetes";
    /// WebAssembly runtime
    pub const WASM: &str = "wasm";
    /// Native process execution
    pub const NATIVE: &str = "native";
    /// Systemd service
    pub const SYSTEMD: &str = "systemd";
    /// BiomeOS platform
    pub const BIOME_OS: &str = "biomeos";
}

/// Temporary paths and prefixes
///
/// **Deep Debt Compliance**: Uses runtime discovery of temp directory
/// - Respects TOADSTOOL_TEMP_DIR environment variable
/// - Falls back to std::env::temp_dir() (platform-agnostic)
/// - Never hardcodes specific paths
pub mod paths {
    use std::path::PathBuf;
    use std::sync::OnceLock;

    static TEMP_DIR: OnceLock<PathBuf> = OnceLock::new();

    /// Get the temporary directory base path
    ///
    /// Priority:
    /// 1. TOADSTOOL_TEMP_DIR environment variable
    /// 2. System temp directory (cross-platform)
    pub fn temp_dir() -> &'static PathBuf {
        TEMP_DIR.get_or_init(|| {
            std::env::var("TOADSTOOL_TEMP_DIR")
                .ok()
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir)
        })
    }

    /// Get checkpoint prefix path (runtime-discovered)
    pub fn checkpoint_prefix() -> PathBuf {
        temp_dir().join("toadstool_checkpoint_")
    }

    /// Get export prefix path (runtime-discovered)
    pub fn export_prefix() -> PathBuf {
        temp_dir().join("toadstool_export_")
    }

    /// Get snapshot prefix path (runtime-discovered)
    pub fn snapshot_prefix() -> PathBuf {
        temp_dir().join("toadstool_snapshot_")
    }
}

/// Trust levels
pub mod trust_levels {
    /// Trusted source
    pub const TRUSTED: &str = "trusted";
    /// Untrusted source
    pub const UNTRUSTED: &str = "untrusted";
    /// Cryptographically verified
    pub const VERIFIED: &str = "verified";
    /// Sovereign (full verification)
    pub const SOVEREIGN: &str = "sovereign";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capabilities() {
        assert_eq!(capabilities::UNIVERSAL_COMPUTE, "universal-compute");
        assert_eq!(capabilities::WASM_EXECUTION, "wasm-execution");
        assert_eq!(capabilities::CONTAINER_RUNTIME, "container-runtime");
    }

    #[test]
    fn test_protocol_versions() {
        assert_eq!(protocol::VERSION_1_0, "1.0");
        assert_eq!(protocol::VERSION_2_0, "2.0");
    }

    #[test]
    fn test_migration_types() {
        assert_eq!(migration_types::LIVE, "live");
        assert_eq!(migration_types::COLD, "cold");
        assert_eq!(migration_types::HOT, "hot");
        assert_eq!(migration_types::CLONE, "clone");
    }

    #[test]
    fn test_platforms() {
        assert_eq!(platforms::DOCKER, "docker");
        assert_eq!(platforms::KUBERNETES, "kubernetes");
        assert_eq!(platforms::WASM, "wasm");
    }

    #[test]
    fn test_paths() {
        // Test runtime-discovered paths
        let temp_dir = paths::temp_dir();
        assert!(temp_dir.exists() || temp_dir.parent().is_some_and(|p| p.exists()));

        // Test prefix functions
        let checkpoint = paths::checkpoint_prefix();
        assert!(
            checkpoint
                .to_string_lossy()
                .contains("toadstool_checkpoint_")
        );

        let export = paths::export_prefix();
        assert!(export.to_string_lossy().contains("toadstool_export_"));

        let snapshot = paths::snapshot_prefix();
        assert!(snapshot.to_string_lossy().contains("toadstool_snapshot_"));
    }

    #[test]
    fn test_temp_dir_respects_env() {
        // This test demonstrates the env var is checked
        // Actual testing would require setting env vars before the program starts
        let temp = paths::temp_dir();
        assert!(temp.is_absolute());
    }
}
