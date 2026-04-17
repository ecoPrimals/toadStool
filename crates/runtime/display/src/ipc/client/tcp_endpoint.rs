// SPDX-License-Identifier: AGPL-3.0-or-later
//! TCP fallback addressing for display IPC (env snapshot + discovery files + [`PlatformPaths`]).

use std::net::SocketAddr;

use toadstool_common::interned_strings::socket_env;
use toadstool_config::defaults::ports::DISPLAY_IPC_FALLBACK;

use super::DisplayClient;
use super::LOOPBACK;

/// Snapshot of display IPC TCP settings (typically from env once at startup).
#[derive(Debug, Clone, Default)]
pub struct DisplayIpcTcpSettings {
    /// Full `host:port` from `TOADSTOOL_DISPLAY_IPC_ADDR`.
    pub full_addr: Option<String>,
    /// Port-only override from `TOADSTOOL_DISPLAY_IPC_PORT`.
    pub port_override: Option<u16>,
}

impl DisplayIpcTcpSettings {
    /// Load overrides from the process environment.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            full_addr: std::env::var(socket_env::TOADSTOOL_DISPLAY_IPC_ADDR).ok(),
            port_override: std::env::var(socket_env::TOADSTOOL_DISPLAY_IPC_PORT)
                .ok()
                .and_then(|p| p.parse().ok()),
        }
    }

    /// Resolve a TCP `host:port` string: explicit env → discovery file → port override → cold-start default.
    #[must_use]
    pub fn resolve_conn_str(&self) -> String {
        if let Some(addr) = &self.full_addr {
            return addr.clone();
        }
        if let Some(addr) = try_parse_discovery_tcp_addr() {
            return addr.to_string();
        }
        let port = self.port_override.unwrap_or(DISPLAY_IPC_FALLBACK);
        format!("{LOOPBACK}:{port}")
    }
}

fn try_parse_discovery_tcp_addr() -> Option<SocketAddr> {
    DisplayClient::get_tcp_discovery_file_candidates()
        .into_iter()
        .find_map(|file| {
            let contents = std::fs::read_to_string(&file).ok()?;
            let addr_str = contents.trim().strip_prefix("tcp:")?;
            addr_str.parse().ok()
        })
}

/// Default TCP address for display IPC fallback (see module docs on [`super::default_display_ipc_tcp_addr`]).
#[must_use]
pub fn default_display_ipc_tcp_addr() -> String {
    DisplayIpcTcpSettings::from_env().resolve_conn_str()
}
