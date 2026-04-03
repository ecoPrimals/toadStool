// SPDX-License-Identifier: AGPL-3.0-only

use tracing::info;

use super::super::types::{Terminal3270, Terminal3270Attributes, Terminal5250};

use crate::{AuthenticationSettings, ConnectionSettings, ToadStoolResult};

impl Default for Terminal3270 {
    fn default() -> Self {
        Self {
            connection: ConnectionSettings {
                host: std::env::var("TOADSTOOL_MAINFRAME_3270_HOST").unwrap_or_else(|_| {
                    std::env::var("TOADSTOOL_BIND_ADDRESS").unwrap_or_else(|_| {
                        toadstool_common::constants::network::LOCALHOST_IPV4.to_string()
                    })
                }),
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
    /// Currently always returns `Ok`.
    pub async fn connect(&mut self, settings: &ConnectionSettings) -> ToadStoolResult<()> {
        self.connection = settings.clone();
        // In a real implementation, this would establish a 3270 connection
        info!(
            "Connected to 3270 terminal at {}:{}",
            settings.host, settings.port
        );
        Ok(())
    }

    /// Disconnects from the 3270 terminal session.
    ///
    /// # Errors
    ///
    /// Currently always returns `Ok`.
    pub async fn disconnect(&mut self) -> ToadStoolResult<()> {
        self.session = None;
        info!("Disconnected from 3270 terminal");
        Ok(())
    }
}

impl Default for Terminal5250 {
    fn default() -> Self {
        Self {
            connection: ConnectionSettings {
                host: std::env::var("TOADSTOOL_MAINFRAME_5250_HOST").unwrap_or_else(|_| {
                    std::env::var("TOADSTOOL_BIND_ADDRESS").unwrap_or_else(|_| {
                        toadstool_common::constants::network::LOCALHOST_IPV4.to_string()
                    })
                }),
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
