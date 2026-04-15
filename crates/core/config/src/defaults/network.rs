// SPDX-License-Identifier: AGPL-3.0-or-later
//! # ⚠️ PARTIALLY DEPRECATED: Network-related default values
//!
//! **Legacy primal-name ports (`SONGBIRD_PORT`, `BEARDOG_PORT`, etc.) are deprecated.**
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
//! // ❌ BAD: Don't hardcode other services' ports
//! // let coordination_port = network::COORDINATION_PORT;
//!
//! // ✅ GOOD: Discover peer capabilities at runtime
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

/// Coordination capability. Port discovered via capability resolution at runtime.
pub const COORDINATION_FALLBACK_PORT: u16 = 0;

/// Security capability. Port discovered via capability resolution at runtime.
pub const SECURITY_FALLBACK_PORT: u16 = 0;

/// Storage capability. Port discovered via capability resolution at runtime.
pub const STORAGE_FALLBACK_PORT: u16 = 0;

/// AI / intelligence capability. Port discovered via capability resolution at runtime.
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

// ═══════════════════════════════════════════════════════════════════════════
// BYOB / ECOSYSTEM DISCOVERY SHARED LITERALS
// ═══════════════════════════════════════════════════════════════════════════

/// Default CIDR for BYOB executor default network subnet.
pub const DEFAULT_NETWORK_SUBNET: &str = "10.0.0.0/24";

/// Gateway IP when the subnet base cannot be parsed (last octet forced to `.1`).
pub const GATEWAY_FALLBACK_IP: &str = "10.0.0.1";

/// First three octets for allocated internal service IPs in BYOB (`{base}.{offset+n}`).
pub const INTERNAL_IP_BASE: &str = "10.0.0";

/// Host octet for the first service in BYOB internal IP allocation.
pub const INTERNAL_IP_OFFSET: usize = 10;

/// Typical private-network `/24` ranges scanned for ecosystem discovery (RFC 1918-style defaults).
pub const RFC1918_SCAN_RANGES: &[&str] = &[
    "192.168.1.0/24",
    "192.168.0.0/24",
    "10.0.0.0/24",
    "172.16.0.0/24",
];

/// Default TCP port when a URL has no port (HTTP probe).
pub const PROBE_DEFAULT_PORT: u16 = 80;

/// Host octets to probe when scanning a `/24` range for ecosystem services.
pub const COMMON_SCAN_SUFFIXES: &[u8] = &[1, 2, 10, 20, 50, 100, 200, 254];

/// RFC 5737 TEST-NET-3 documentation prefix (non-globally routable; safe for examples).
pub const TEST_NET_3_PREFIX: &str = "203.0.113";

/// Alias for documentation / example IP allocation (same as [`TEST_NET_3_PREFIX`]).
pub const DOCUMENTATION_PREFIX: &str = TEST_NET_3_PREFIX;

// ═══════════════════════════════════════════════════════════════════════════
// TCP CONNECTION LIFECYCLE
// ═══════════════════════════════════════════════════════════════════════════

/// Idle timeout for TCP connections (seconds).
///
/// Connections with no activity for this duration are closed to prevent stale
/// half-open connections accumulating in multi-gate WAN deployments.
/// Override via `TOADSTOOL_TCP_IDLE_TIMEOUT_SECS`.
pub const TCP_IDLE_TIMEOUT_SECS: u64 = 300;
