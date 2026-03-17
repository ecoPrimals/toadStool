// SPDX-License-Identifier: AGPL-3.0-only
//! Input validation for security hardening
//!
//! Extracted from security_hardening.rs for modularity (Feb 14, 2026).

use crate::{ToadStoolError, ToadStoolResult};

use super::config::ValidationRules;

/// Input validator
pub struct InputValidator {
    /// Validation rules
    rules: ValidationRules,
}

impl InputValidator {
    /// Create new input validator
    #[must_use]
    pub const fn new(rules: ValidationRules) -> Self {
        Self { rules }
    }

    /// Validate input string
    pub fn validate_input(&self, input: &str) -> ToadStoolResult<()> {
        // Check length
        if input.len() > self.rules.max_input_length {
            return Err(ToadStoolError::validation(format!(
                "Input length {} exceeds maximum {}",
                input.len(),
                self.rules.max_input_length
            )));
        }

        // Check for blocked patterns
        for pattern in &self.rules.blocked_patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                if regex.is_match(input) {
                    return Err(ToadStoolError::security(format!(
                        "Input contains blocked pattern: {pattern}"
                    )));
                }
            }
        }

        // Check for SQL injection
        for pattern in &self.rules.sql_injection_patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                if regex.is_match(input) {
                    return Err(ToadStoolError::security(
                        "Input contains SQL injection pattern".to_string(),
                    ));
                }
            }
        }

        // Check for XSS
        for pattern in &self.rules.xss_patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                if regex.is_match(input) {
                    return Err(ToadStoolError::security(
                        "Input contains XSS pattern".to_string(),
                    ));
                }
            }
        }

        // Check for command injection
        for pattern in &self.rules.command_injection_patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                if regex.is_match(input) {
                    return Err(ToadStoolError::security(
                        "Input contains command injection pattern".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }

    /// Sanitize input string
    #[must_use]
    pub fn sanitize_input(&self, input: &str) -> String {
        let mut sanitized = input.to_string();

        // HTML entity encoding for common dangerous characters
        sanitized = sanitized.replace('&', "&amp;");
        sanitized = sanitized.replace('<', "&lt;");
        sanitized = sanitized.replace('>', "&gt;");
        sanitized = sanitized.replace('"', "&quot;");
        sanitized = sanitized.replace('\'', "&#x27;");

        // Remove null bytes
        sanitized = sanitized.replace('\0', "");

        // Truncate if too long
        if sanitized.len() > self.rules.max_input_length {
            sanitized.truncate(self.rules.max_input_length);
        }

        sanitized
    }
}
