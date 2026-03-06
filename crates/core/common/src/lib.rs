// SPDX-License-Identifier: AGPL-3.0-or-later
#![deny(unsafe_code)]

//! # `ToadStool` Common Utilities
//!
//! This crate provides common utilities, types, and functionality shared across all `ToadStool` components.
//!
//! ## Features
//!
//! - ID generation utilities
//! - Time and timestamp handling
//! - Format utilities for bytes and duration
//! - Validation traits for type safety
//! - Infant discovery system

use std::time::SystemTime;
use uuid::Uuid;

// Public modules
pub mod auth;
pub mod capability_discovery; // NEW: Pure infant discovery API (modern interface)
pub mod capability_provider; // NEW: Deep Debt - Capability-based service provider abstraction
pub mod config_bases;
pub mod constants; // Ecosystem constants: JSON-RPC codes, timeouts, network defaults
pub mod discovery_defaults; // NEW: Fallback defaults for service discovery (infant pattern)
pub mod error;
pub mod error_codes;
pub mod infant_discovery;
pub mod interned_strings; // NEW: Zero-allocation string constants
pub mod modern_utils;
pub mod platform_paths; // NEW: Platform-agnostic path resolution (ecoBin v2.0)
pub mod primal_capabilities; // Capability-to-primal reference mapping
pub mod primal_discovery; // NEW: Runtime capability-based primal discovery
pub mod primal_discovery_complete; // NEW: Complete capability-based discovery with mDNS
pub mod primal_discovery_mdns; // NEW: mDNS integration adapter
pub mod primal_identity;
pub mod primal_integration; // NEW: Self-knowledge only architecture
pub mod primal_sockets;
pub mod runtime_discovery; // UPDATED: Zero-hardcoding capability-based discovery
pub mod runtime_ports; // NEW: Deep Debt compliant dynamic port discovery
pub mod self_identity; // Self-aware primal identity and capability discovery
pub mod service_discovery; // NEW: Capability-based service discovery (infant pattern)
pub mod system_time_serde; // Serde for std::time::SystemTime (Unix timestamp)
pub mod uid_detector; // NEW: Pure Rust unix socket path discovery (100% pure Rust!)
pub mod universal_adapter;
pub mod unix_jsonrpc_client; // NEW: JSON-RPC 2.0 over unix sockets (pure Rust!) // NEW: Phase 1B - Capability-based primal discovery (infant discovery)

// Re-export commonly used types
pub use auth::{AuthCredentials, AuthType, ServiceAuthConfig};

pub use error::{
    ConfigError, ConfigResult, ExecutionError, ExecutionResult, IntegrationError,
    IntegrationResult, NetworkError, NetworkResult, ResourceError, ResourceResult, SecurityError,
    SecurityResult, SystemError, SystemResult, ToadStoolError, ToadStoolErrorExt,
    ToadStoolErrorWithCode, ToadStoolResult, ToadStoolResultWithCode,
};

pub use error_codes::{codes, ErrorCategory, ErrorCode};

/// A unique identifier for `ToadStool` resources
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ToadStoolId(Uuid);

impl ToadStoolId {
    /// Generate a new random ID
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Get the inner UUID
    #[must_use]
    pub const fn inner(&self) -> Uuid {
        self.0
    }
}

impl Default for ToadStoolId {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate a unique ID for `ToadStool` resources
#[must_use]
pub fn generate_id() -> Uuid {
    Uuid::new_v4()
}

/// A timestamp representing a point in time in `ToadStool` systems
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Timestamp(SystemTime);

impl Timestamp {
    /// Create a new timestamp with the current time
    #[must_use]
    pub fn now() -> Self {
        Self(SystemTime::now())
    }

    /// Create a timestamp from the current system time
    #[must_use]
    pub fn current() -> Self {
        Self(SystemTime::now())
    }

    /// Get the inner `SystemTime`
    #[must_use]
    pub const fn inner(&self) -> SystemTime {
        self.0
    }
}

/// Format bytes in a human-readable way
///
/// # Examples
///
/// ```
/// # use toadstool_common::format_bytes;
/// assert_eq!(format_bytes(1024), "1.0 KB");
/// assert_eq!(format_bytes(1048576), "1.0 MB");
/// ```
#[must_use]
pub fn format_bytes(bytes: u64) -> String {
    #[allow(clippy::cast_precision_loss)]
    let mut size = bytes as f64;
    let units = ["B", "KB", "MB", "GB", "TB", "PB"];
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < units.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, units[unit_index])
    } else {
        format!("{:.1} {}", size, units[unit_index])
    }
}

/// Format a duration in a human-readable way
///
/// # Examples
///
/// ```
/// # use std::time::Duration;
/// # use toadstool_common::format_duration;
/// assert_eq!(format_duration(Duration::from_secs(90)), "1m 30s");
/// assert_eq!(format_duration(Duration::from_secs(3661)), "1h 1m 1s");
/// ```
#[must_use]
pub fn format_duration(duration: std::time::Duration) -> String {
    let secs = duration.as_secs();
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;

    if hours > 0 {
        format!("{hours}h {minutes}m {seconds}s")
    } else if minutes > 0 {
        format!("{minutes}m {seconds}s")
    } else {
        format!("{seconds}s")
    }
}

/// Trait for validating `ToadStool` types
pub trait Validate {
    /// The error type returned when validation fails
    type Error;

    /// Validate the type
    ///
    /// # Errors
    ///
    /// Returns an error if the type is not valid according to its constraints
    fn validate(&self) -> Result<(), Self::Error>;
}

/// String extensions
pub trait StringExt {
    /// Check if string is empty or contains only whitespace
    fn is_blank(&self) -> bool;

    /// Truncate string to maximum length
    fn truncate_to(&self, max_len: usize) -> String;
}

impl StringExt for str {
    fn is_blank(&self) -> bool {
        self.trim().is_empty()
    }

    fn truncate_to(&self, max_len: usize) -> String {
        if self.len() <= max_len {
            self.to_string()
        } else {
            format!("{}...", &self[..max_len.saturating_sub(3)])
        }
    }
}

impl StringExt for String {
    fn is_blank(&self) -> bool {
        self.as_str().is_blank()
    }

    fn truncate_to(&self, max_len: usize) -> String {
        self.as_str().truncate_to(max_len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1_048_576), "1.0 MB");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(std::time::Duration::from_secs(30)), "30s");
        assert_eq!(
            format_duration(std::time::Duration::from_secs(90)),
            "1m 30s"
        );
        assert_eq!(
            format_duration(std::time::Duration::from_secs(3661)),
            "1h 1m 1s"
        );
    }

    #[test]
    fn test_string_extensions() {
        assert!("".is_blank());
        assert!("   ".is_blank());
        assert!(!"hello".is_blank());

        assert_eq!("hello".truncate_to(10), "hello");
        assert_eq!("hello world".truncate_to(8), "hello...");
    }

    #[test]
    fn test_generate_id() {
        let id1 = generate_id();
        let id2 = generate_id();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_toadstool_id_new() {
        let id = ToadStoolId::new();
        assert_ne!(id.inner(), uuid::Uuid::nil());
    }

    #[test]
    fn test_toadstool_id_default() {
        let id = ToadStoolId::default();
        assert_ne!(id.inner(), uuid::Uuid::nil());
    }

    #[test]
    fn test_toadstool_id_equality() {
        let id = ToadStoolId::new();
        assert_eq!(id, id);
    }

    #[test]
    fn test_toadstool_id_debug() {
        let id = ToadStoolId::new();
        let debug_str = format!("{id:?}");
        assert!(!debug_str.is_empty());
    }

    #[test]
    fn test_timestamp_now() {
        let ts = Timestamp::now();
        assert!(ts.inner().elapsed().is_ok());
    }

    #[test]
    fn test_timestamp_current() {
        let ts = Timestamp::current();
        assert!(ts.inner().elapsed().is_ok());
    }

    #[test]
    fn test_timestamp_inner() {
        let ts = Timestamp::now();
        let inner = ts.inner();
        assert_eq!(ts.inner(), inner);
    }

    #[test]
    fn test_format_bytes_large() {
        assert_eq!(format_bytes(1_073_741_824), "1.0 GB");
        assert_eq!(format_bytes(1_099_511_627_776), "1.0 TB");
    }

    #[test]
    fn test_string_extensions_truncate_boundary() {
        assert_eq!("hi".truncate_to(2), "hi");
        assert_eq!("hello".truncate_to(5), "hello");
    }

    #[test]
    fn test_string_extensions_truncate_saturating() {
        assert_eq!("x".truncate_to(0), "...");
    }
}
