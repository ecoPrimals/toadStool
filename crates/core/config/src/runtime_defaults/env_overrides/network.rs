// SPDX-License-Identifier: AGPL-3.0-only
//! Network configuration overrides.
//!
//! # Network Environment Variables (primary; current values are fallback defaults)
//!
//! | Variable | Fallback | Description |
//! |----------|----------|-------------|
//! | `TOADSTOOL_BIND_ADDRESS` | `127.0.0.1` (from config) | Full socket address (host:port) |
//! | `TOADSTOOL_PORT` | port from config | Override port only |
//! | `TOADSTOOL_SONGBIRD_ENDPOINT` | (deprecated) | Legacy coordination endpoint |
//! | `TOADSTOOL_BEARDOG_ENDPOINT` | (deprecated) | Legacy PKI endpoint |
//! | `TOADSTOOL_NESTGATE_ENDPOINT` | (deprecated) | Legacy storage endpoint |
//! | `TOADSTOOL_SQUIRREL_ENDPOINT` | (deprecated) | Legacy AI endpoint |

use super::super::{ConfigError, ConfigResult};
use super::parse;
use crate::ToadStoolConfig;
use std::time::Duration;

pub(super) fn apply(config: &mut ToadStoolConfig) -> ConfigResult<()> {
    // TOADSTOOL_BIND_ADDRESS: full "host:port" (e.g. 0.0.0.0:9000, 127.0.0.1:3000)
    if let Ok(bind_address) = std::env::var("TOADSTOOL_BIND_ADDRESS") {
        config.network.bind_address = bind_address
            .parse()
            .map_err(|e| ConfigError::Invalid(format!("Invalid bind address: {e}")))?;
    }

    // TOADSTOOL_PORT: override port only
    if let Ok(port) = std::env::var("TOADSTOOL_PORT") {
        let port = parse::parse_u16(&port, "port")?;
        config.network.bind_address.set_port(port);
    }

    // Legacy endpoint overrides (deprecated - use capability-based discovery)
    #[allow(deprecated)]
    {
        if let Ok(songbird_endpoint) = std::env::var("TOADSTOOL_SONGBIRD_ENDPOINT") {
            config.network.endpoints.songbird = songbird_endpoint;
        }

        if let Ok(beardog_endpoint) = std::env::var("TOADSTOOL_BEARDOG_ENDPOINT") {
            config.network.endpoints.beardog = beardog_endpoint;
        }

        if let Ok(nestgate_endpoint) = std::env::var("TOADSTOOL_NESTGATE_ENDPOINT") {
            config.network.endpoints.nestgate = nestgate_endpoint;
        }

        if let Ok(squirrel_endpoint) = std::env::var("TOADSTOOL_SQUIRREL_ENDPOINT") {
            config.network.endpoints.squirrel = squirrel_endpoint;
        }
    }

    if let Ok(request_timeout) = std::env::var("TOADSTOOL_REQUEST_TIMEOUT") {
        let timeout_secs = parse::parse_u64(&request_timeout, "request timeout")?;
        config.network.connection.request_timeout = Duration::from_secs(timeout_secs);
    }

    Ok(())
}
