// SPDX-License-Identifier: AGPL-3.0-or-later
//! Self IP / bind host discovery for primal execution context.

use toadstool_common::interned_strings::socket_env;
use toadstool_config::defaults::network::BIND_ADDRESS_DEFAULT;

/// Discovers the primal's own IP/host for `PrimalContext.network_location`.
///
/// Resolution order: `TOADSTOOL_BIND_ADDRESS` (host part) → `TOADSTOOL_BIND_HOST` →
/// `BIND_HOST` → `HOST` → `HOSTNAME` → `BIND_ADDRESS_DEFAULT` (`127.0.0.1`).
///
/// This is the self-discovery default when no explicit bind address is configured.
#[must_use]
pub(in crate::universal::scheduler::execution) fn discover_self_ip_address() -> String {
    // 1. TOADSTOOL_BIND_ADDRESS (host:port) — extract host
    if let Ok(addr) = std::env::var(socket_env::TOADSTOOL_BIND_ADDRESS) {
        let host = addr.split(':').next().unwrap_or(&addr).trim();
        if !host.is_empty() {
            return host.to_string();
        }
    }
    // 2. TOADSTOOL_BIND_HOST
    if let Ok(h) = std::env::var(socket_env::TOADSTOOL_BIND_HOST)
        && !h.is_empty()
    {
        return h;
    }
    // 3. BIND_HOST
    if let Ok(h) = std::env::var(socket_env::BIND_HOST)
        && !h.is_empty()
    {
        return h;
    }
    // 4. HOST
    if let Ok(h) = std::env::var(socket_env::HOST)
        && !h.is_empty()
    {
        return h;
    }
    // 5. HOSTNAME
    if let Ok(h) = std::env::var(socket_env::HOSTNAME)
        && !h.is_empty()
    {
        return h;
    }
    // 6. Fallback: loopback default
    BIND_ADDRESS_DEFAULT.to_string()
}

#[cfg(test)]
#[path = "discover_self_ip_tests.rs"]
mod discover_self_ip_tests;
