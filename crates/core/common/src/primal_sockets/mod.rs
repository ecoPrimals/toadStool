// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Primal Unix Socket Discovery
//!
//! Pure Rust unix socket path resolution for primal-to-primal communication.
//!
//! ## Fallback Constants (Transition Period)
//!
//! The `resolve_*_socket_fallback` and `resolve_socket_path_for_service` functions
//! use biomeOS standard socket filenames (e.g. `beardog.sock`, `songbird.sock`).
//! These are **fallback constants** for the transition period until full
//! capability-based discovery is deployed.
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
pub use paths::{
    resolve_beardog_socket_fallback, resolve_biomeos_dir, resolve_family_id,
    resolve_nestgate_socket_fallback, resolve_nucleus_socket, resolve_runtime_dir,
    resolve_socket_path_for_service, resolve_songbird_socket_fallback, resolve_squirrel_socket,
    resolve_toadstool_socket,
};
