// SPDX-License-Identifier: AGPL-3.0-or-later
//! Cold-start discovery-related fallback ports (shared with `toadstool_config::defaults::ports`).

/// Default HTTP port for K8s/Compose discovery probes when `TOADSTOOL_DISCOVERY_HTTP_PORT` is unset.
pub const DISCOVERY_HTTP_FALLBACK: u16 = 8080;

/// Default coordination/orchestration fallback when `TOADSTOOL_COORDINATION_PORT` is unset.
pub const DEFAULT_COORDINATION_PORT: u16 = DISCOVERY_HTTP_FALLBACK;

/// Default security/auth fallback when `TOADSTOOL_SECURITY_PORT` is unset.
pub const DEFAULT_SECURITY_PORT: u16 = 8081;

/// Default storage fallback when `TOADSTOOL_STORAGE_PORT` is unset.
pub const DEFAULT_STORAGE_PORT: u16 = 8082;

/// Default base port for `TOADSTOOL_DISCOVERY_FALLBACK_PORT` (localhost fallback endpoints).
pub const DISCOVERY_LOCALHOST_FALLBACK_BASE: u16 = 9080;

/// Cold-start fallback for display IPC TCP when `TOADSTOOL_DISPLAY_IPC_PORT` is unset.
pub const DISPLAY_IPC_FALLBACK: u16 = 8091;
