// SPDX-License-Identifier: AGPL-3.0-only
//! # Legacy Networking Support
//!
//! Support for legacy network protocols:
//! - NetBIOS
//! - IPX/SPX
//! - DECnet
//! - Token Ring
//! - Legacy serial protocols

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Legacy network protocol support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LegacyNetworkProtocol {
    NetBIOS,
    IPXSPX,
    DECnet,
    TokenRing,
    SerialProtocol { baud_rate: u32 },
}

/// Legacy networking manager
#[derive(Debug)]
pub struct LegacyNetworkManager {
    protocols: HashMap<String, LegacyNetworkProtocol>,
}

impl LegacyNetworkManager {
    pub fn new() -> Self {
        Self {
            protocols: HashMap::new(),
        }
    }
    
    pub fn add_protocol(&mut self, name: impl Into<String>, protocol: LegacyNetworkProtocol) {
        self.protocols.insert(name.into(), protocol);
    }
    
    pub fn get_protocols(&self) -> &HashMap<String, LegacyNetworkProtocol> {
        &self.protocols
    }
} 