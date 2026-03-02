//! Application and general configuration overrides.

use super::super::ConfigResult;
use super::parse;
use crate::ToadStoolConfig;

pub(super) fn apply(config: &mut ToadStoolConfig) -> ConfigResult<()> {
    if let Ok(env_name) = std::env::var("TOADSTOOL_ENV") {
        config.app.environment = env_name;
    }

    if let Ok(debug) = std::env::var("TOADSTOOL_DEBUG") {
        config.features.enable_debug = parse::parse_bool(&debug);
    }

    if let Ok(verbose) = std::env::var("TOADSTOOL_VERBOSE") {
        config.logging.level = if parse::parse_bool(&verbose) {
            "debug".to_string()
        } else {
            "info".to_string()
        };
    }

    if let Ok(log_level) = std::env::var("TOADSTOOL_LOG_LEVEL") {
        config.logging.level = log_level;
    }

    if let Ok(data_dir) = std::env::var("TOADSTOOL_DATA_DIR") {
        config.app.data_dir = data_dir;
    }

    if let Ok(cache_dir) = std::env::var("TOADSTOOL_CACHE_DIR") {
        config.app.cache_dir = cache_dir;
    }

    if let Ok(worker_threads) = std::env::var("TOADSTOOL_WORKER_THREADS") {
        config.app.worker_threads = parse::parse_usize(&worker_threads, "worker threads")?;
    }

    Ok(())
}
