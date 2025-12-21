//! Version and protocol constants
//!
//! Centralized version strings and protocol identifiers.

// ============================================================================
// ToadStool Versions
// ============================================================================

/// ToadStool version
pub const TOADSTOOL_VERSION: &str = env!("CARGO_PKG_VERSION");

/// ToadStool major version
pub const TOADSTOOL_MAJOR_VERSION: &str = "0";

/// ToadStool minor version
pub const TOADSTOOL_MINOR_VERSION: &str = "1";

/// ToadStool patch version
pub const TOADSTOOL_PATCH_VERSION: &str = "0";

// ============================================================================
// Protocol Versions
// ============================================================================

/// Federation protocol version
pub const FEDERATION_PROTOCOL_VERSION: &str = "1.0";

/// `WebSocket` protocol version
pub const WS_PROTOCOL_VERSION: &str = "13";

/// HTTP API version
pub const API_VERSION: &str = "v1";

/// Biome manifest version
pub const MANIFEST_VERSION: &str = "1.0";

// ============================================================================
// User Agent Strings
// ============================================================================

/// Default user agent for HTTP clients
pub const USER_AGENT: &str = concat!("ToadStool/", env!("CARGO_PKG_VERSION"));

/// Client library user agent
pub const CLIENT_USER_AGENT: &str = "ToadStool-Client/1.0";

/// Server user agent
pub const SERVER_USER_AGENT: &str = "ToadStool-Server/1.0";

// ============================================================================
// Compatibility
// ============================================================================

/// Minimum supported client version
pub const MIN_CLIENT_VERSION: &str = "0.1.0";

/// Minimum supported server version
pub const MIN_SERVER_VERSION: &str = "0.1.0";

/// Breaking change version marker
pub const BREAKING_CHANGE_VERSION: &str = "1.0.0";

// ============================================================================
// Feature Flags (Compile-time)
// ============================================================================

/// Zero telemetry build flag
pub const ZERO_TELEMETRY: bool = true;

/// Zero phone-home build flag
pub const ZERO_PHONE_HOME: bool = true;

/// Full sovereignty mode
pub const FULL_SOVEREIGNTY: bool = true;
