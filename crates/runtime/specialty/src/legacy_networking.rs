// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Legacy Networking Support
//!
//! Support for legacy network protocols:
//! - NetBIOS
//! - IPX/SPX
//! - DECnet
//! - Token Ring
//! - Legacy serial protocols

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Legacy network protocol support for vintage systems.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LegacyNetworkProtocol {
    /// NetBIOS protocol (IBM LAN Manager, early Windows).
    NetBIOS,
    /// IPX/SPX protocol (Novell NetWare).
    IPXSPX,
    /// DECnet protocol (Digital Equipment Corporation).
    DECnet,
    /// Token Ring (IBM LAN technology).
    TokenRing,
    /// Serial line protocol with configurable baud rate.
    SerialProtocol {
        /// Baud rate for serial communication.
        baud_rate: u32,
    },
}

/// Legacy networking manager
#[derive(Debug, Default)]
pub struct LegacyNetworkManager {
    protocols: HashMap<String, LegacyNetworkProtocol>,
}

impl LegacyNetworkManager {
    /// Creates a new legacy network manager with no protocols registered.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a legacy protocol under the given name.
    pub fn add_protocol(&mut self, name: impl Into<String>, protocol: LegacyNetworkProtocol) {
        self.protocols.insert(name.into(), protocol);
    }

    /// Returns the map of registered protocol names to protocols.
    pub const fn get_protocols(&self) -> &HashMap<String, LegacyNetworkProtocol> {
        &self.protocols
    }
}
