//! Capability-based crypto service discovery
//!
//! **Design Philosophy**:
//! - No hardcoding: Discover crypto providers by capability, not name
//! - Runtime discovery: Find services dynamically
//! - Self-knowledge: Toadstool knows what it needs, not where to get it

use serde::{Deserialize, Serialize};

use super::SecurityLevel;

/// Crypto capability description
///
/// **Design**: Describes what crypto capabilities are needed,
/// not which specific service provides them
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoCapability {
    /// Supported encryption algorithms
    pub algorithms: Vec<String>,

    /// Required security level
    pub security_level: SecurityLevel,

    /// Whether hardware-backed crypto is required
    pub hardware_backed: bool,
}

impl CryptoCapability {
    /// Check if this capability matches required features
    pub fn matches(&self, required: &CryptoCapability) -> bool {
        // Check security level
        if self.security_level < required.security_level {
            return false;
        }

        // Check hardware requirement
        if required.hardware_backed && !self.hardware_backed {
            return false;
        }

        // Check if we support at least one required algorithm
        required
            .algorithms
            .iter()
            .any(|req_alg| self.algorithms.iter().any(|our_alg| our_alg == req_alg))
    }

    /// Score this capability match (higher = better match)
    ///
    /// **Design**: Allows prioritizing better matches
    pub fn match_score(&self, required: &CryptoCapability) -> u32 {
        let mut score = 0u32;

        // Security level match (exact match = higher score)
        if self.security_level == required.security_level {
            score += 100;
        } else if self.security_level > required.security_level {
            score += 50; // Better than required, but not exact
        }

        // Hardware backing
        if self.hardware_backed && required.hardware_backed {
            score += 50;
        }

        // Algorithm matches (more = better)
        let matching_algorithms = required
            .algorithms
            .iter()
            .filter(|req_alg| self.algorithms.contains(req_alg))
            .count();
        score += matching_algorithms as u32 * 10;

        score
    }
}

/// Crypto service discovery query
///
/// **Design**: Describes what we're looking for without hardcoding
#[derive(Debug, Clone)]
pub struct CryptoServiceQuery {
    /// Required capability
    pub capability: CryptoCapability,

    /// Preferred service location (None = any)
    pub preferred_location: Option<ServiceLocation>,

    /// Maximum acceptable latency
    pub max_latency_ms: Option<u64>,
}

impl CryptoServiceQuery {
    /// Create query for specific capability
    pub fn for_capability(capability: CryptoCapability) -> Self {
        Self {
            capability,
            preferred_location: None,
            max_latency_ms: None,
        }
    }

    /// Prefer local services
    pub fn prefer_local(mut self) -> Self {
        self.preferred_location = Some(ServiceLocation::Local);
        self
    }

    /// Prefer network services (distributed)
    pub fn prefer_network(mut self) -> Self {
        self.preferred_location = Some(ServiceLocation::Network);
        self
    }

    /// Set maximum acceptable latency
    pub fn max_latency(mut self, ms: u64) -> Self {
        self.max_latency_ms = Some(ms);
        self
    }
}

/// Service location preference
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceLocation {
    /// Local process (same machine)
    Local,
    /// Network service (remote, but same private network)
    Network,
    /// Internet service (public network)
    Internet,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_matching() {
        let provided = CryptoCapability {
            algorithms: vec!["chacha20poly1305".to_string(), "aes-256-gcm".to_string()],
            security_level: SecurityLevel::Enhanced,
            hardware_backed: false,
        };

        let required = CryptoCapability {
            algorithms: vec!["chacha20poly1305".to_string()],
            security_level: SecurityLevel::Standard,
            hardware_backed: false,
        };

        assert!(provided.matches(&required));
    }

    #[test]
    fn test_capability_no_match_security_level() {
        let provided = CryptoCapability {
            algorithms: vec!["chacha20poly1305".to_string()],
            security_level: SecurityLevel::Standard,
            hardware_backed: false,
        };

        let required = CryptoCapability {
            algorithms: vec!["chacha20poly1305".to_string()],
            security_level: SecurityLevel::Enhanced,
            hardware_backed: false,
        };

        assert!(!provided.matches(&required));
    }

    #[test]
    fn test_capability_scoring() {
        let provided = CryptoCapability {
            algorithms: vec!["chacha20poly1305".to_string(), "aes-256-gcm".to_string()],
            security_level: SecurityLevel::Enhanced,
            hardware_backed: true,
        };

        let required = CryptoCapability {
            algorithms: vec!["chacha20poly1305".to_string()],
            security_level: SecurityLevel::Enhanced,
            hardware_backed: true,
        };

        let score = provided.match_score(&required);
        assert!(score > 100); // Should have high score
    }
}
