// SPDX-License-Identifier: AGPL-3.0-or-later

use super::super::types::{Terminal3270, Terminal3270Attributes, Terminal5250};

use crate::{AuthenticationSettings, ConnectionSettings, ToadStoolError, ToadStoolResult};
use toadstool_common::interned_strings::socket_env;

impl Default for Terminal3270 {
    fn default() -> Self {
        Self {
            connection: ConnectionSettings {
                host: std::env::var(socket_env::TOADSTOOL_MAINFRAME_3270_HOST).unwrap_or_else(
                    |_| {
                        std::env::var(socket_env::TOADSTOOL_BIND_ADDRESS).unwrap_or_else(|_| {
                            toadstool_common::constants::network::LOCALHOST_IPV4.to_string()
                        })
                    },
                ),
                port: 3270,
                connection_type: crate::MainframeConnectionType::IBM3270,
                authentication: AuthenticationSettings {
                    auth_type: crate::AuthenticationType::None,
                    username: None,
                    password: None,
                    key_file: None,
                    certificate: None,
                },
            },
            session: None,
            screen_buffer: vec![vec![' '; 80]; 24],
            cursor_position: (0, 0),
            attributes: Terminal3270Attributes {
                width: 80,
                height: 24,
                color_support: false,
                extended_attributes: false,
            },
        }
    }
}

impl Terminal3270 {
    /// Creates a new 3270 terminal emulator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Connects to the mainframe using the given connection settings.
    ///
    /// # Errors
    ///
    /// Returns an error until TN3270 transport is implemented.
    pub async fn connect(&mut self, settings: &ConnectionSettings) -> ToadStoolResult<()> {
        let _ = settings;
        Err(ToadStoolError::not_supported(
            "3270 terminal connection not implemented — requires TN3270 transport",
        ))
    }

    /// Disconnects from the 3270 terminal session.
    ///
    /// # Errors
    ///
    /// Returns an error until TN3270 transport is implemented.
    pub async fn disconnect(&mut self) -> ToadStoolResult<()> {
        Err(ToadStoolError::not_supported(
            "3270 terminal disconnect not implemented — requires TN3270 transport",
        ))
    }
}

impl Default for Terminal5250 {
    fn default() -> Self {
        Self {
            connection: ConnectionSettings {
                host: std::env::var(socket_env::TOADSTOOL_MAINFRAME_5250_HOST).unwrap_or_else(
                    |_| {
                        std::env::var(socket_env::TOADSTOOL_BIND_ADDRESS).unwrap_or_else(|_| {
                            toadstool_common::constants::network::LOCALHOST_IPV4.to_string()
                        })
                    },
                ),
                port: 5250,
                connection_type: crate::MainframeConnectionType::IBM5250,
                authentication: AuthenticationSettings {
                    auth_type: crate::AuthenticationType::None,
                    username: None,
                    password: None,
                    key_file: None,
                    certificate: None,
                },
            },
            session: None,
            screen_buffer: vec![vec![' '; 80]; 24],
            field_definitions: vec![],
        }
    }
}

impl Terminal5250 {
    /// Creates a new 5250 terminal emulator for AS/400.
    pub fn new() -> Self {
        Self::default()
    }
}
