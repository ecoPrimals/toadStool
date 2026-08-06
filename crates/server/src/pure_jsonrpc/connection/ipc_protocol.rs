// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ecoPrimals Contributors

//! IPC Protocol Abstraction for G65 Protocol Negotiation.
//!
//! Defines the protocol variants that toadStool can serve on a single socket.
//! Convergent evolution from squirrel (origin) and nestGate (refined).

use serde::{Deserialize, Serialize};
use std::fmt;

/// RPC protocol selector for G65 protocol negotiation.
///
/// Each incoming connection negotiates which protocol to use. JSON-RPC is
/// the default (backward-compatible); tarpc is the high-performance path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum IpcProtocol {
    /// JSON-RPC 2.0 — default, human-readable, language-agnostic.
    #[default]
    JsonRpc,
    /// tarpc — binary, type-safe, high-performance intra-gate RPC.
    Tarpc,
}

impl fmt::Display for IpcProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.negotiation_name())
    }
}

impl IpcProtocol {
    /// Parse a protocol name from a wire string (case-insensitive).
    #[allow(
        clippy::should_implement_trait,
        reason = "custom from_str avoids FromStr trait conflict with std"
    )]
    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "jsonrpc" | "json-rpc" | "json_rpc" => Some(Self::JsonRpc),
            "tarpc" => Some(Self::Tarpc),
            _ => None,
        }
    }

    /// All protocols this server can serve.
    #[must_use]
    pub fn supported() -> Vec<Self> {
        vec![Self::JsonRpc, Self::Tarpc]
    }

    /// Wire name used in `PROTOCOLS:` / `PROTOCOL:` negotiation lines.
    #[must_use]
    pub const fn negotiation_name(self) -> &'static str {
        match self {
            Self::JsonRpc => "jsonrpc",
            Self::Tarpc => "tarpc",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_jsonrpc() {
        assert_eq!(IpcProtocol::default(), IpcProtocol::JsonRpc);
    }

    #[test]
    fn from_str_jsonrpc_variants() {
        assert_eq!(IpcProtocol::from_str("jsonrpc"), Some(IpcProtocol::JsonRpc));
        assert_eq!(
            IpcProtocol::from_str("json-rpc"),
            Some(IpcProtocol::JsonRpc)
        );
        assert_eq!(
            IpcProtocol::from_str("JSON_RPC"),
            Some(IpcProtocol::JsonRpc)
        );
    }

    #[test]
    fn from_str_tarpc() {
        assert_eq!(IpcProtocol::from_str("tarpc"), Some(IpcProtocol::Tarpc));
        assert_eq!(IpcProtocol::from_str("TARPC"), Some(IpcProtocol::Tarpc));
    }

    #[test]
    fn from_str_unknown_returns_none() {
        assert_eq!(IpcProtocol::from_str("grpc"), None);
        assert_eq!(IpcProtocol::from_str(""), None);
    }

    #[test]
    fn supported_includes_both() {
        let s = IpcProtocol::supported();
        assert!(s.contains(&IpcProtocol::JsonRpc));
        assert!(s.contains(&IpcProtocol::Tarpc));
    }

    #[test]
    fn display_matches_negotiation_name() {
        assert_eq!(IpcProtocol::JsonRpc.to_string(), "jsonrpc");
        assert_eq!(IpcProtocol::Tarpc.to_string(), "tarpc");
    }

    #[test]
    fn negotiation_name_is_lowercase() {
        for proto in IpcProtocol::supported() {
            let name = proto.negotiation_name();
            assert_eq!(
                name,
                name.to_lowercase(),
                "{proto} wire name must be lowercase"
            );
        }
    }
}
