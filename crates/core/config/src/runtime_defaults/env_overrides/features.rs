//! Feature flags and metrics/cache toggle overrides.

use super::super::ConfigResult;
use super::parse;
use crate::{BackendCacheConfig, MetricsConfig, ToadStoolConfig};

pub(super) fn apply(config: &mut ToadStoolConfig) -> ConfigResult<()> {
    if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_METRICS") {
        config.metrics = if parse::parse_bool(&enabled) {
            Some(MetricsConfig::default())
        } else {
            None
        };
    }

    if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_CACHE") {
        config.cache = if parse::parse_bool(&enabled) {
            Some(BackendCacheConfig::default())
        } else {
            None
        };
    }

    if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_AUTH") {
        config.security.auth.enabled = parse::parse_bool(&enabled);
    }

    if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_SANDBOX") {
        config.security.sandbox.enabled = parse::parse_bool(&enabled);
    }

    if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_FEDERATION") {
        config.features.enable_federation = parse::parse_bool(&enabled);
    }

    if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_DISTRIBUTED") {
        config.features.enable_distributed = parse::parse_bool(&enabled);
    }

    if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_AUTO_CONFIG") {
        config.features.enable_auto_config = parse::parse_bool(&enabled);
    }

    if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_HOT_RELOAD") {
        config.features.enable_hot_reload = parse::parse_bool(&enabled);
    }

    if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_EXPERIMENTAL") {
        config.features.enable_experimental = parse::parse_bool(&enabled);
    }

    if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_BETA") {
        config.features.enable_beta = parse::parse_bool(&enabled);
    }

    if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_PROFILING") {
        config.features.enable_profiling = parse::parse_bool(&enabled);
    }

    if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_OPENAPI") {
        config.features.enable_openapi = parse::parse_bool(&enabled);
    }

    if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_GRPC") {
        #[allow(deprecated)]
        {
            config.features.enable_grpc = parse::parse_bool(&enabled);
        }
    }

    if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_GRAPHQL") {
        config.features.enable_graphql = parse::parse_bool(&enabled);
    }

    Ok(())
}
