// SPDX-License-Identifier: AGPL-3.0-or-later
//! Service endpoints, legacy service enum, and trust levels.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::net::SocketAddr;
use std::sync::Arc;

/// Discovered ecosystem service endpoint
#[derive(Debug, Clone)]
#[expect(
    deprecated,
    reason = "ServiceEndpoint uses deprecated EcosystemService for backward compatibility"
)]
pub struct ServiceEndpoint {
    /// Service capability type (discovery, crypto, storage)
    pub service_type: EcosystemService,
    /// Network address (host:port)
    pub address: SocketAddr,
    /// **Zero-Copy**: Uses `Arc<str>` for cheap clones in registry lookups
    pub version: Arc<str>,
    /// Capability names this service provides
    pub capabilities: Vec<String>,
    /// How the endpoint was discovered and verified
    pub trust_level: TrustLevel,
}

// Custom Serialize implementation for ServiceEndpoint
impl Serialize for ServiceEndpoint {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("ServiceEndpoint", 5)?;
        state.serialize_field("service_type", &self.service_type)?;
        state.serialize_field("address", &self.address)?;
        state.serialize_field("version", self.version.as_ref())?;
        state.serialize_field("capabilities", &self.capabilities)?;
        state.serialize_field("trust_level", &self.trust_level)?;
        state.end()
    }
}

// Custom Deserialize implementation for ServiceEndpoint
impl<'de> Deserialize<'de> for ServiceEndpoint {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[expect(
            deprecated,
            reason = "EcosystemService is deprecated but still used for backward compat"
        )]
        struct ServiceEndpointHelper {
            service_type: EcosystemService,
            address: SocketAddr,
            version: String,
            capabilities: Vec<String>,
            trust_level: TrustLevel,
        }

        let helper = ServiceEndpointHelper::deserialize(deserializer)?;
        Ok(Self {
            service_type: helper.service_type,
            address: helper.address,
            version: Arc::from(helper.version.as_str()),
            capabilities: helper.capabilities,
            trust_level: helper.trust_level,
        })
    }
}

/// ⚠️ DEPRECATED: Use capability-based ServiceType instead.
///
/// Evolved to use capability categories for WateringHole sovereignty.
#[deprecated(
    since = "0.1.0",
    note = "Use ServiceType::from_capability() instead. Capability-based discovery."
)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum EcosystemService {
    /// Discovery / coordination capability
    #[serde(alias = "Songbird")] // legacy deserialize
    Discovery,
    /// Crypto / security capability
    #[serde(alias = "BearDog")] // legacy deserialize
    Crypto,
    /// Storage capability
    #[serde(alias = "NestGate")] // legacy deserialize
    Storage,
    /// Unknown capability (discovered at runtime)
    Unknown(String),
}

#[expect(deprecated, reason = "implementation of deprecated EcosystemService")]
impl EcosystemService {
    /// Capability string for this service type
    pub(crate) fn name(&self) -> &str {
        match self {
            Self::Discovery => "discovery",
            Self::Crypto => "crypto",
            Self::Storage => "storage",
            Self::Unknown(name) => name,
        }
    }
}

/// Trust level of a discovered service endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrustLevel {
    /// Trust level unknown
    Unknown,
    /// Found via network scan
    Discovered,
    /// Advertised via mDNS/service mesh
    Advertised,
    /// Explicitly configured (env var/config file)
    Configured,
    /// Cryptographically verified
    Verified,
    /// Full sovereign verification
    Sovereign,
}
