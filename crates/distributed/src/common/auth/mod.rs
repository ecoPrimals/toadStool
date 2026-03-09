// SPDX-License-Identifier: AGPL-3.0-only
//! Common Authentication Abstractions
//!
//! Capability-based authentication for inter-primal trust. Primals discover
//! each other at runtime and establish trust via capability tokens.
//!
//! ## Evolution
//!
//! - Phase 1-3: No auth (localhost only, single-user)
//! - Phase 4: Token-based capability auth (this module)
//! - Phase 5: Mutual TLS + signed capability tokens

/// Trust level between primals, discovered at runtime.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum TrustLevel {
    /// No trust — unauthenticated connection.
    #[default]
    Untrusted,
    /// Localhost peer — same machine, basic trust.
    LocalPeer,
    /// Authenticated via capability token.
    Authenticated,
    /// Mutually authenticated with verified identity.
    MutuallyVerified,
}

/// A capability token exchanged between primals for authorization.
#[derive(Debug, Clone)]
pub struct CapabilityToken {
    /// Primal that issued this token.
    pub issuer: String,
    /// Capabilities granted by this token.
    pub capabilities: Vec<String>,
    /// Expiry timestamp (seconds since epoch). 0 = no expiry.
    pub expires_at: u64,
}

impl CapabilityToken {
    /// Whether this token has expired.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        if self.expires_at == 0 {
            return false;
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        now >= self.expires_at
    }

    /// Whether this token grants a specific capability.
    #[must_use]
    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities.iter().any(|c| c == capability)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trust_level_ordering() {
        assert!(TrustLevel::Untrusted < TrustLevel::LocalPeer);
        assert!(TrustLevel::LocalPeer < TrustLevel::Authenticated);
        assert!(TrustLevel::Authenticated < TrustLevel::MutuallyVerified);
    }

    #[test]
    fn test_trust_level_default() {
        assert_eq!(TrustLevel::default(), TrustLevel::Untrusted);
    }

    #[test]
    fn test_capability_token_no_expiry() {
        let token = CapabilityToken {
            issuer: "toadStool".to_string(),
            capabilities: vec!["compute.execute".to_string()],
            expires_at: 0,
        };
        assert!(!token.is_expired());
        assert!(token.has_capability("compute.execute"));
        assert!(!token.has_capability("storage.write"));
    }

    #[test]
    fn test_capability_token_expired() {
        let token = CapabilityToken {
            issuer: "toadStool".to_string(),
            capabilities: vec![],
            expires_at: 1,
        };
        assert!(token.is_expired());
    }
}
