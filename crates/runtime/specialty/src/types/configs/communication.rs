// SPDX-License-Identifier: AGPL-3.0-only
//! Communication and connection configuration types for legacy systems

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use toadstool_common::config_bases::{RetryConfig, TimeoutConfig};

/// Communication settings for legacy systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationSettings {
    /// Connection type
    pub connection_type: ConnectionType,

    /// Timeout configuration (connection, request, read, write)
    #[serde(flatten)]
    pub timeouts: TimeoutConfig,

    /// Retry configuration (max attempts, backoff, jitter)
    #[serde(flatten)]
    pub retries: RetryConfig,

    /// Authentication settings
    pub authentication: Option<AuthenticationSettings>,
}

impl Default for CommunicationSettings {
    fn default() -> Self {
        Self {
            connection_type: ConnectionType::LocalEmulation,
            timeouts: TimeoutConfig::default(),
            retries: RetryConfig::default(),
            authentication: None,
        }
    }
}

/// Connection types for legacy systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionType {
    /// Direct serial connection
    DirectSerial { port: String, baud_rate: u32 },
    /// Telnet connection
    Telnet { host: String, port: u16 },
    /// SSH connection
    SSH { host: String, port: u16 },
    /// IBM 3270 terminal emulation
    IBM3270 { host: String, port: u16 },
    /// Local emulation
    LocalEmulation,
    /// Custom connection
    Custom {
        name: String,
        parameters: HashMap<String, String>,
    },
}

/// Authentication settings for legacy systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationSettings {
    /// Authentication type
    pub auth_type: AuthenticationType,
    /// Username
    pub username: Option<String>,
    /// Password
    pub password: Option<String>,
    /// Key file
    pub key_file: Option<PathBuf>,
    /// Certificate
    pub certificate: Option<PathBuf>,
}

/// Authentication types for legacy systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthenticationType {
    /// No authentication
    None,
    /// Username/password
    UsernamePassword,
    /// Public key
    PublicKey,
    /// Certificate
    Certificate,
    /// Custom authentication
    Custom { name: String },
}

/// Connection settings for mainframes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionSettings {
    /// Host address
    pub host: String,
    /// Port number
    pub port: u16,
    /// Connection type
    pub connection_type: MainframeConnectionType,
    /// Authentication
    pub authentication: AuthenticationSettings,
}

/// Mainframe connection types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MainframeConnectionType {
    /// IBM 3270 terminal
    IBM3270,
    /// IBM 5250 terminal
    IBM5250,
    /// FTP
    FTP,
    /// SFTP
    SFTP,
    /// HTTP/HTTPS
    HTTP,
    /// Custom connection
    Custom { name: String },
}

/// Programming interface for embedded systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgrammingInterface {
    /// Interface type
    pub interface_type: ProgrammingInterfaceType,
    /// Connection parameters
    pub connection_params: HashMap<String, String>,
}

/// Programming interface types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProgrammingInterfaceType {
    /// In-System Programming (ISP)
    ISP,
    /// In-Circuit Serial Programming (ICSP)
    ICSP,
    /// JTAG
    JTAG,
    /// SWD (Serial Wire Debug)
    SWD,
    /// Parallel programmer
    Parallel,
    /// Serial programmer
    Serial,
    /// Custom interface
    Custom { name: String },
}
