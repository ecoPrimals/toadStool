// SPDX-License-Identifier: AGPL-3.0-or-later
//! Capability-based socket path discovery.
//!
//! Discovers Unix socket paths by capability rather than by primal name.

/// Discover a service by capability rather than by primal name.
///
/// Primals only have self-knowledge and discover other services at runtime.
/// This function returns a socket path or URL when available. For full endpoint
/// discovery with health status, use [`super::discover_service_by_capability`].
///
/// Discovery order:
/// 1. `{CAPABILITY}_SOCKET` environment variable (e.g., `SECURITY_SOCKET`)
/// 2. XDG runtime directory: `$XDG_RUNTIME_DIR/{capability}.sock` (or `/tmp` fallback)
#[must_use]
pub fn discover_service_socket_by_capability(capability: &str) -> Option<String> {
    let env_key = format!("{}_SOCKET", capability.to_uppercase().replace('-', "_"));
    std::env::var(&env_key).ok().or_else(|| {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
        let socket_path = format!("{}/{}.sock", runtime_dir, capability);
        if std::path::Path::new(&socket_path).exists() {
            Some(socket_path)
        } else {
            None
        }
    })
}
