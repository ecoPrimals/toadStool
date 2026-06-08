// SPDX-License-Identifier: AGPL-3.0-or-later
//! sourDough-compatible transport endpoint abstraction.
//!
//! Implements the canonical `TransportEndpoint` wire format defined by the
//! sourDough scaffold standard.  When `sourdough-core` becomes available as a
//! crate dependency this module can re-export its type instead.
//!
//! # Wire format
//!
//! ```json
//! { "transport": "uds",        "path": "/run/user/1000/biomeos/beardog.sock" }
//! { "transport": "tcp",        "host": "127.0.0.1", "port": 9100 }
//! { "transport": "mesh_relay", "peer_id": "strandgate", "capability": "security" }
//! ```
//!
//! Primals accept the `TRANSPORT_ENDPOINT` environment variable as a JSON
//! string.  The launcher / Tower Atomic decides the transport — primals never
//! self-bind in production.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Canonical transport endpoint (sourDough-compatible wire format).
///
/// Tagged by `"transport"` field per the ecosystem standard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "transport", rename_all = "snake_case")]
pub enum TransportEndpoint {
    /// Unix domain socket (filesystem path).
    Uds {
        /// Absolute path to the socket file.
        path: PathBuf,
    },

    /// TCP socket (host + port).
    Tcp {
        /// Host or IP address.
        host: String,
        /// Port number.
        port: u16,
    },

    /// Mesh relay (peer-routed via songBird mesh layer).
    MeshRelay {
        /// Mesh peer identifier.
        peer_id: String,
        /// Capability name to route to.
        capability: String,
    },
}

impl TransportEndpoint {
    /// Load from the `TRANSPORT_ENDPOINT` environment variable.
    ///
    /// Returns `None` if the variable is unset or empty.
    ///
    /// # Errors
    ///
    /// Returns an error if the variable is set but contains invalid JSON.
    pub fn from_env() -> Result<Option<Self>, String> {
        let val = std::env::var(crate::interned_strings::socket_env::TRANSPORT_ENDPOINT);
        match val {
            Ok(s) if s.is_empty() => Ok(None),
            Ok(s) => serde_json::from_str(&s)
                .map(Some)
                .map_err(|e| format!("invalid TRANSPORT_ENDPOINT JSON: {e}")),
            Err(_) => Ok(None),
        }
    }

    /// True when this endpoint represents a local Unix domain socket.
    #[must_use]
    pub const fn is_uds(&self) -> bool {
        matches!(self, Self::Uds { .. })
    }

    /// True when this endpoint represents a TCP connection.
    #[must_use]
    pub const fn is_tcp(&self) -> bool {
        matches!(self, Self::Tcp { .. })
    }

    /// True when this endpoint is mesh-relayed.
    #[must_use]
    pub const fn is_mesh_relay(&self) -> bool {
        matches!(self, Self::MeshRelay { .. })
    }
}

impl std::fmt::Display for TransportEndpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Uds { path } => write!(f, "uds:{}", path.display()),
            Self::Tcp { host, port } => write!(f, "tcp://{host}:{port}"),
            Self::MeshRelay {
                peer_id,
                capability,
            } => write!(f, "mesh://{peer_id}/{capability}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_uds() {
        let ep = TransportEndpoint::Uds {
            path: PathBuf::from("/run/user/1000/biomeos/compute.sock"),
        };
        let json = serde_json::to_string(&ep).unwrap();
        assert!(json.contains(r#""transport":"uds""#));
        let parsed: TransportEndpoint = serde_json::from_str(&json).unwrap();
        assert_eq!(ep, parsed);
    }

    #[test]
    fn roundtrip_tcp() {
        let ep = TransportEndpoint::Tcp {
            host: "127.0.0.1".into(),
            port: 9100,
        };
        let json = serde_json::to_string(&ep).unwrap();
        assert!(json.contains(r#""transport":"tcp""#));
        let parsed: TransportEndpoint = serde_json::from_str(&json).unwrap();
        assert_eq!(ep, parsed);
    }

    #[test]
    fn roundtrip_mesh_relay() {
        let ep = TransportEndpoint::MeshRelay {
            peer_id: "strandgate".into(),
            capability: "security".into(),
        };
        let json = serde_json::to_string(&ep).unwrap();
        assert!(json.contains(r#""transport":"mesh_relay""#));
        let parsed: TransportEndpoint = serde_json::from_str(&json).unwrap();
        assert_eq!(ep, parsed);
    }

    #[test]
    fn wire_format_matches_sourdough() {
        let json = r#"{"transport":"uds","path":"/run/user/1000/biomeos/beardog.sock"}"#;
        let parsed: TransportEndpoint = serde_json::from_str(json).unwrap();
        assert!(parsed.is_uds());

        let json = r#"{"transport":"tcp","host":"127.0.0.1","port":9100}"#;
        let parsed: TransportEndpoint = serde_json::from_str(json).unwrap();
        assert!(parsed.is_tcp());

        let json = r#"{"transport":"mesh_relay","peer_id":"strandgate","capability":"security"}"#;
        let parsed: TransportEndpoint = serde_json::from_str(json).unwrap();
        assert!(parsed.is_mesh_relay());
    }

    #[test]
    fn display_format() {
        let uds = TransportEndpoint::Uds {
            path: "/tmp/test.sock".into(),
        };
        assert_eq!(uds.to_string(), "uds:/tmp/test.sock");

        let tcp = TransportEndpoint::Tcp {
            host: "10.0.0.1".into(),
            port: 8080,
        };
        assert_eq!(tcp.to_string(), "tcp://10.0.0.1:8080");

        let mesh = TransportEndpoint::MeshRelay {
            peer_id: "p1".into(),
            capability: "compute".into(),
        };
        assert_eq!(mesh.to_string(), "mesh://p1/compute");
    }

    #[test]
    fn from_env_when_unset() {
        temp_env::with_var_unset("TRANSPORT_ENDPOINT", || {
            let result = TransportEndpoint::from_env();
            assert!(result.unwrap().is_none());
        });
    }

    #[test]
    fn from_env_valid_json() {
        let json = r#"{"transport":"uds","path":"/run/user/1000/biomeos/compute.sock"}"#;
        temp_env::with_var("TRANSPORT_ENDPOINT", Some(json), || {
            let result = TransportEndpoint::from_env().unwrap();
            assert!(result.is_some());
            assert!(result.unwrap().is_uds());
        });
    }

    #[test]
    fn reject_invalid_json() {
        temp_env::with_var("TRANSPORT_ENDPOINT", Some("not json"), || {
            let result = TransportEndpoint::from_env();
            assert!(result.is_err());
        });
    }
}
