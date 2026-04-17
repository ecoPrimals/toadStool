// SPDX-License-Identifier: AGPL-3.0-or-later
//! Path and environment constants for Linux sandbox integration.

/// Environment variable overriding the directory for per-sandbox log files.
pub const ENV_SANDBOX_LOG_DIR: &str = "TOADSTOOL_SANDBOX_LOG_DIR";

/// Default log directory when `ENV_SANDBOX_LOG_DIR` is unset.
pub const DEFAULT_SANDBOX_LOG_DIR: &str = "/var/log/toadstool/sandbox";

/// cgroup v2 unified hierarchy mount point.
pub const CGROUP2_FS_ROOT: &str = "/sys/fs/cgroup";
