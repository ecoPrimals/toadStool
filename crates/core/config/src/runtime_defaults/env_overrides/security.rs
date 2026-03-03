// SPDX-License-Identifier: AGPL-3.0-or-later
//! Security configuration overrides (auth, encryption, audit, sandbox).

use super::super::ConfigResult;
use super::parse;
use crate::ToadStoolConfig;
use std::time::Duration;

pub(super) fn apply(config: &mut ToadStoolConfig) -> ConfigResult<()> {
    if let Ok(jwt_secret) = std::env::var("TOADSTOOL_JWT_SECRET") {
        config.security.auth.jwt_secret = Some(jwt_secret);
    }

    if let Ok(session_timeout) = std::env::var("TOADSTOOL_SESSION_TIMEOUT") {
        let timeout_secs = parse::parse_u64(&session_timeout, "session timeout")?;
        config.security.auth.session_timeout = Duration::from_secs(timeout_secs);
    }

    if let Ok(max_attempts) = std::env::var("TOADSTOOL_MAX_LOGIN_ATTEMPTS") {
        config.security.auth.max_login_attempts =
            parse::parse_u32(&max_attempts, "max login attempts")?;
    }

    if let Ok(lockout_duration) = std::env::var("TOADSTOOL_LOCKOUT_DURATION") {
        let duration_secs = parse::parse_u64(&lockout_duration, "lockout duration")?;
        config.security.auth.lockout_duration = Duration::from_secs(duration_secs);
    }

    if let Ok(enabled) = std::env::var("TOADSTOOL_ENCRYPTION_ENABLED") {
        config.security.encryption.enabled = parse::parse_bool(&enabled);
    }

    if let Ok(algorithm) = std::env::var("TOADSTOOL_ENCRYPTION_ALGORITHM") {
        config.security.encryption.algorithm = algorithm;
    }

    if let Ok(key_length) = std::env::var("TOADSTOOL_ENCRYPTION_KEY_LENGTH") {
        config.security.encryption.key_length =
            parse::parse_usize(&key_length, "encryption key length")?;
    }

    if let Ok(enabled) = std::env::var("TOADSTOOL_AUDIT_ENABLED") {
        config.security.audit.enabled = parse::parse_bool(&enabled);
    }

    if let Ok(log_file) = std::env::var("TOADSTOOL_AUDIT_LOG_FILE") {
        config.security.audit.log_file = log_file;
    }

    if let Ok(log_level) = std::env::var("TOADSTOOL_AUDIT_LOG_LEVEL") {
        config.security.audit.log_level = log_level;
    }

    if let Ok(sandbox_type) = std::env::var("TOADSTOOL_SANDBOX_TYPE") {
        config.security.sandbox.sandbox_type = sandbox_type;
    }

    if let Ok(allow_network) = std::env::var("TOADSTOOL_SANDBOX_ALLOW_NETWORK") {
        config.security.sandbox.allow_network = parse::parse_bool(&allow_network);
    }

    if let Ok(allow_file_access) = std::env::var("TOADSTOOL_SANDBOX_ALLOW_FILE_ACCESS") {
        config.security.sandbox.allow_file_access = parse::parse_bool(&allow_file_access);
    }

    Ok(())
}
