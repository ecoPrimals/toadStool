// SPDX-License-Identifier: AGPL-3.0-or-later
//! Logging configuration overrides.

use super::super::ConfigResult;
use super::parse;
use crate::ToadStoolConfig;
use toadstool_common::interned_strings::socket_env;

pub(super) fn apply(config: &mut ToadStoolConfig) -> ConfigResult<()> {
    if let Ok(log_to_file) = std::env::var(socket_env::TOADSTOOL_LOG_TO_FILE) {
        config.logging.log_to_file = parse::parse_bool(&log_to_file);
    }

    if let Ok(log_file) = std::env::var(socket_env::TOADSTOOL_LOG_FILE) {
        config.logging.log_file = log_file;
    }

    if let Ok(log_format) = std::env::var(socket_env::TOADSTOOL_LOG_FORMAT) {
        config.logging.format = log_format;
    }

    if let Ok(enabled) = std::env::var("TOADSTOOL_LOG_COLORS") {
        config.logging.enable_colors = parse::parse_bool(&enabled);
    }

    if let Ok(enabled) = std::env::var(socket_env::TOADSTOOL_LOG_TIMESTAMPS) {
        config.logging.enable_timestamps = parse::parse_bool(&enabled);
    }

    if let Ok(enabled) = std::env::var("TOADSTOOL_LOG_THREAD_IDS") {
        config.logging.enable_thread_ids = parse::parse_bool(&enabled);
    }

    if let Ok(enabled) = std::env::var(socket_env::TOADSTOOL_LOG_MODULE_PATHS) {
        config.logging.enable_module_paths = parse::parse_bool(&enabled);
    }

    if let Ok(enabled) = std::env::var(socket_env::TOADSTOOL_LOG_ROTATION) {
        config.logging.log_rotation = parse::parse_bool(&enabled);
    }

    if let Ok(max_size) = std::env::var(socket_env::TOADSTOOL_LOG_MAX_SIZE) {
        config.logging.max_log_size = parse::parse_u64(&max_size, "log max size")?;
    }

    if let Ok(max_files) = std::env::var(socket_env::TOADSTOOL_LOG_MAX_FILES) {
        config.logging.max_log_files = parse::parse_u32(&max_files, "log max files")?;
    }

    Ok(())
}
