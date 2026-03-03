// SPDX-License-Identifier: AGPL-3.0-or-later
//! Helper functions for sandbox management
//!
//! This module contains platform-agnostic helper functions for sandbox
//! creation, validation, and directory management.

use std::path::{Path, PathBuf};
use tracing::debug;
use uuid::Uuid;

use toadstool::error::{ToadStoolError, ToadStoolResult};

use super::types::*;

/// Generate unique sandbox ID
pub fn generate_sandbox_id() -> String {
    format!("sandbox_{}", Uuid::new_v4().simple())
}

/// Validate sandbox specification
pub async fn validate_sandbox_spec(spec: &SandboxSpec) -> ToadStoolResult<()> {
    // Validate resource limits
    if let Some(memory) = spec.resource_limits.max_memory_bytes {
        if memory == 0 {
            return Err(ToadStoolError::validation(
                "Memory limit cannot be zero".to_string(),
            ));
        }
    }

    if let Some(cpu) = spec.resource_limits.max_cpu_percent {
        if cpu <= 0.0 || cpu > 100.0 {
            return Err(ToadStoolError::validation(format!(
                "CPU limit must be between 1 and 100, got {cpu}"
            )));
        }
    }

    // Validate network configuration
    if !spec.network_config.enabled && !spec.network_config.allowed_hosts.is_empty() {
        debug!("Warning: Network disabled but allowed hosts specified");
    }

    // Validate filesystem mounts
    for mount in &spec.filesystem_mounts {
        if mount.source.to_string_lossy().is_empty() {
            return Err(ToadStoolError::validation(
                "Mount source path cannot be empty".to_string(),
            ));
        }

        if mount.target.to_string_lossy().is_empty() {
            return Err(ToadStoolError::validation(
                "Mount target path cannot be empty".to_string(),
            ));
        }

        // Warn about read-write bind mounts
        if matches!(mount.mount_type, MountType::ReadWriteBind) {
            debug!("Warning: Read-write bind mount: {:?}", mount.source);
        }
    }

    Ok(())
}

/// Create sandbox directory structure
pub async fn create_sandbox_directories(
    sandbox_root: &Path,
    sandbox_id: &str,
) -> ToadStoolResult<PathBuf> {
    let sandbox_dir = sandbox_root.join(sandbox_id);

    tokio::fs::create_dir_all(&sandbox_dir).await.map_err(|e| {
        ToadStoolError::configuration(format!(
            "Failed to create sandbox directory {}: {}",
            sandbox_dir.display(),
            e
        ))
    })?;

    // Create standard directories
    let dirs = ["bin", "etc", "tmp", "var", "proc", "sys", "dev"];
    for dir in &dirs {
        let dir_path = sandbox_dir.join(dir);
        tokio::fs::create_dir_all(&dir_path).await.map_err(|e| {
            ToadStoolError::configuration(format!(
                "Failed to create sandbox subdirectory {}: {}",
                dir_path.display(),
                e
            ))
        })?;
    }

    Ok(sandbox_dir)
}

// Removed validate_mount_target() and calculate_default_limits() - unused helper
// functions. Mount validation is handled inline in setup_filesystem_mounts().
// Resource limits are specified directly in SandboxSpec.
