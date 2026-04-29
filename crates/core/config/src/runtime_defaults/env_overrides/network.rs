// SPDX-License-Identifier: AGPL-3.0-or-later
//! Network configuration overrides.
//!
//! # Network Environment Variables (primary; current values are fallback defaults)
//!
//! | Variable | Fallback | Description |
//! |----------|----------|-------------|
//! | `TOADSTOOL_BIND_ADDRESS` | `127.0.0.1` (from config) | Full socket address (host:port) |
//! | `TOADSTOOL_PORT` | port from config | Override port only |
//! | `TOADSTOOL_COORDINATION_ENDPOINT` | (none) | Coordination service endpoint |
//! | `TOADSTOOL_SECURITY_ENDPOINT` | (none) | Security / PKI endpoint |
//! | `TOADSTOOL_STORAGE_ENDPOINT` | (none) | Storage endpoint |
//! | `TOADSTOOL_AI_ENDPOINT` | (none) | AI processing endpoint |
//! | `TOADSTOOL_SONGBIRD_ENDPOINT` | (deprecated) | Legacy alias for coordination endpoint |
//! | `TOADSTOOL_BEARDOG_ENDPOINT` | (deprecated) | Legacy alias for security endpoint |
//! | `TOADSTOOL_NESTGATE_ENDPOINT` | (deprecated) | Legacy alias for storage endpoint |
//! | `TOADSTOOL_SQUIRREL_ENDPOINT` | (deprecated) | Legacy alias for AI endpoint |

use super::super::{ConfigError, ConfigResult};
use super::parse;
use crate::ToadStoolConfig;
use std::time::Duration;
use toadstool_common::interned_strings::socket_env;

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
    #[expect(
        deprecated,
        reason = "legacy endpoint env-vars kept for migration; use capability-based discovery"
    )]
    {
        if let Ok(coordination) = std::env::var("TOADSTOOL_COORDINATION_ENDPOINT")
            .or_else(|_| std::env::var("TOADSTOOL_SONGBIRD_ENDPOINT"))
        {
            config.network.endpoints.coordination = coordination;
        }

        if let Ok(security) = std::env::var("TOADSTOOL_SECURITY_ENDPOINT")
            .or_else(|_| std::env::var("TOADSTOOL_BEARDOG_ENDPOINT"))
        {
            config.network.endpoints.security = security;
        }

        if let Ok(storage) = std::env::var("TOADSTOOL_STORAGE_ENDPOINT")
            .or_else(|_| std::env::var("TOADSTOOL_NESTGATE_ENDPOINT"))
        {
            config.network.endpoints.storage = storage;
        }

        if let Ok(ai) = std::env::var("TOADSTOOL_AI_ENDPOINT")
            .or_else(|_| std::env::var("TOADSTOOL_SQUIRREL_ENDPOINT"))
        {
            config.network.endpoints.ai_processing = ai;
        }
    }

    if let Ok(request_timeout) = std::env::var(socket_env::TOADSTOOL_REQUEST_TIMEOUT) {
        let timeout_secs = parse::parse_u64(&request_timeout, "request timeout")?;
        config.network.connection.request_timeout = Duration::from_secs(timeout_secs);
    }

    Ok(())
}
