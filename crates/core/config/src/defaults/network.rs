// SPDX-License-Identifier: AGPL-3.0-or-later
//! # ⚠️ PARTIALLY DEPRECATED: Network-related default values
//!
//! **Primal ports (`SONGBIRD_PORT`, `BEARDOG_PORT`, etc.) are deprecated.**
//! Use `RuntimeDiscovery` with capability-based discovery instead.
//!
//! **Self-configuration (`API_PORT`, `METRICS_PORT`) remains valid** - these are ToadStool's own ports.
//!
//! # Modern Example
//!
//! ```rust,ignore
//! use toadstool_config::defaults::network;
//! use toadstool_common::{RuntimeDiscovery, Capability};
//!
//! // ✅ GOOD: Use for self-configuration
//! let my_api_port = network::API_PORT;
//! let my_metrics_port = network::METRICS_PORT;
//!
//! // ❌ BAD: Don't hardcode other primals
//! // let songbird_port = network::SONGBIRD_PORT;
//!
//! // ✅ GOOD: Discover other primals at runtime
//! let discovery = RuntimeDiscovery::new(client);
//! let coordinators = discovery
//!     .discover_capability(&Capability::Coordination)
//!     .await?;
//! ```
//!
//! **Philosophy**: Know yourself, discover others at runtime.

/// Default bind address (listen on all interfaces).
/// Use for server bind addresses. Override via `TOADSTOOL_BIND_ADDRESS`.
pub const BIND_ADDRESS_DEFAULT: &str = "0.0.0.0";

/// Loopback address for local connections (e.g. client connecting to localhost).
/// Do NOT use for server binding; use `BIND_ADDRESS_DEFAULT` instead.
pub const LOCALHOST: &str = "127.0.0.1";

// ═══════════════════════════════════════════════════════════════════════════
// PRIMAL DISCOVERY PORTS (OS-ASSIGNED)
// ═══════════════════════════════════════════════════════════════════════════
//
// Port 0 means "not configured / discover at runtime". Primals discover each
// other via IPC capability resolution, not hardcoded ports. These exist only
// as sentinel values; actual endpoints come from RuntimeDiscovery.
// ═══════════════════════════════════════════════════════════════════════════

/// Coordination primals (e.g., Songbird). Port discovered via capability resolution at runtime.
pub const COORDINATION_FALLBACK_PORT: u16 = 0;

/// Security primals (e.g., BearDog). Port discovered via capability resolution at runtime.
pub const SECURITY_FALLBACK_PORT: u16 = 0;

/// Storage primals (e.g., NestGate). Port discovered via capability resolution at runtime.
pub const STORAGE_FALLBACK_PORT: u16 = 0;

/// AI primals (e.g., Squirrel). Port discovered via capability resolution at runtime.
pub const AI_FALLBACK_PORT: u16 = 0;

/// Port for JSON-RPC event streaming (replaces deprecated `WebSocket`)
/// Port 0 = OS-assigned at bind time.
pub const EVENTS_PORT: u16 = 0;

/// Default ToadStool API port
/// Port 0 = OS-assigned at bind time.
pub const API_PORT: u16 = 0;

/// Default metrics/telemetry port
/// Port 0 = OS-assigned at bind time.
pub const METRICS_PORT: u16 = 0;

/// Default discovery service port
/// Port 0 = OS-assigned at bind time.
pub const DISCOVERY_PORT: u16 = 0;

/// Default federation port for cross-primal communication
/// Port 0 = OS-assigned at bind time.
pub const FEDERATION_PORT: u16 = 0;

/// Default coordination endpoint URL (fallback when discovery unavailable).
/// Re-exported from `toadstool_common` for config layer access.
pub use toadstool_common::constants::network::DEFAULT_COORDINATION_ENDPOINT;

/// Default server endpoint for client connections (development/testing fallback).
/// Alias for `DEFAULT_COORDINATION_ENDPOINT`. Use `TOADSTOOL_SERVER_URL` env var or discovery in production.
pub use toadstool_common::constants::network::DEFAULT_COORDINATION_ENDPOINT as DEFAULT_SERVER_ENDPOINT;
