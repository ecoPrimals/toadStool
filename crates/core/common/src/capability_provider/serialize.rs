// SPDX-License-Identifier: AGPL-3.0-or-later
//! Capability serialization helpers.
//!
//! Converts between `Capability` enum and string representation for discovery protocol.
//! These should match the Capability enum in `primal_identity`.

use crate::primal_identity::Capability;

/// Convert capability to discovery protocol string
#[cfg_attr(not(unix), allow(dead_code))]
pub fn capability_to_string(cap: &Capability) -> String {
    match cap {
        Capability::Compute(_) => "compute".to_string(),
        Capability::Storage(_) => "storage".to_string(),
        Capability::Crypto(_) => "crypto".to_string(),
        Capability::Authentication(_) => "authentication".to_string(),
        Capability::Coordination(_) => "coordination".to_string(),
        Capability::Discovery(_) => "discovery".to_string(),
        Capability::Custom { name, .. } => name.clone(),
    }
}

/// Parse discovery protocol string into capability
#[cfg_attr(not(unix), allow(dead_code))]
pub fn string_to_capability(s: &str) -> Capability {
    use crate::primal_identity::{
        AuthCapability, Capability, ComputeCapability, CoordinationCapability, CryptoCapability,
        DiscoveryCapability, StorageCapability,
    };

    match s {
        "compute" => Capability::Compute(ComputeCapability::NativeExecution),
        "storage" => Capability::Storage(StorageCapability::ObjectStorage),
        "crypto" => Capability::Crypto(CryptoCapability::Encryption),
        "authentication" | "security" => {
            Capability::Authentication(AuthCapability::TokenManagement)
        }
        "coordination" => Capability::Coordination(CoordinationCapability::ServiceDiscovery),
        "discovery" => Capability::Discovery(DiscoveryCapability::RegistryDiscovery),
        other => Capability::Custom {
            name: other.to_string(),
            version: "1.0".to_string(),
        },
    }
}
