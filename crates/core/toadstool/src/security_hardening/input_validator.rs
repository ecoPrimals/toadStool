// SPDX-License-Identifier: AGPL-3.0-only
//! Input validation for security hardening
//!
//! Extracted from `security_hardening.rs` for modularity (Feb 14, 2026).

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
    ///
    /// # Errors
    ///
    /// Returns error if length or content violates configured rules.
    pub fn validate_input(&self, input: &str) -> ToadStoolResult<()> {
        // Check length
        if input.len() > self.rules.max_input_length {
            return Err(ToadStoolError::validation(format!(
                "Input length {} exceeds maximum {}",
                input.len(),
                self.rules.max_input_length
            )));
        }

        self.check_patterns(input, &self.rules.blocked_patterns, "blocked pattern")?;
        self.check_patterns(
            input,
            &self.rules.sql_injection_patterns,
            "SQL injection pattern",
        )?;
        self.check_patterns(input, &self.rules.xss_patterns, "XSS pattern")?;
        self.check_patterns(
            input,
            &self.rules.command_injection_patterns,
            "command injection pattern",
        )?;

        Ok(())
    }

    fn check_patterns(&self, input: &str, patterns: &[String], label: &str) -> ToadStoolResult<()> {
        let lowered = input.to_lowercase();
        for pattern in patterns {
            if lowered.contains(&pattern.to_lowercase()) {
                return Err(ToadStoolError::security(format!(
                    "Input contains {label}: {pattern}"
                )));
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
