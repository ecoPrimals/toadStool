// SPDX-License-Identifier: AGPL-3.0-only
//! # Primal Unix Socket Discovery
//!
//! Pure Rust unix socket path resolution for primal-to-primal communication.
//!
//! ## Fallback filenames (cold start)
//!
//! When discovery is unavailable, Unix socket paths fall back to capability-named
//! files under the biomeOS runtime directory (e.g. `crypto.sock`, `coordination.sock`).
//! Legacy environment variables (`BEARDOG_SOCKET`, …) remain supported as aliases.
//!
//! ## TRUE PRIMAL Architecture
//!
//! - **No HTTP**: All primal communication via unix sockets
//! - **Songbird Handles External**: Only Songbird uses HTTP/TLS for external
//! - **Local IPC**: Fast, secure, pure Rust
//! - **Discovery-Based**: Socket paths from environment/runtime

mod api;
mod discovery;
mod env;
mod paths;
#[cfg(test)]
mod tests;

#[allow(deprecated)]
pub use api::{
    ensure_biomeos_dir, get_beardog_socket_path, get_biomeos_dir, get_family_id,
    get_nestgate_socket_path, get_nucleus_socket_path, get_runtime_dir,
    get_socket_path_for_capability, get_socket_path_for_service, get_songbird_socket_path,
    get_squirrel_socket_path, get_toadstool_socket_path,
};
pub use discovery::{
    SocketDiscoveryError, discover_coordination_socket, discover_crypto_socket,
    discover_socket_for_capability, discover_storage_socket,
};
pub use env::SocketPathEnv;
#[allow(deprecated)]
pub use paths::{
    resolve_beardog_socket_fallback, resolve_biomeos_dir, resolve_capability_socket_fallback,
    resolve_family_id, resolve_nestgate_socket_fallback, resolve_nucleus_socket,
    resolve_routing_socket, resolve_runtime_dir, resolve_socket_path_for_service,
    resolve_songbird_socket_fallback, resolve_squirrel_socket, resolve_toadstool_socket,
    service_label_to_capability_id,
};
