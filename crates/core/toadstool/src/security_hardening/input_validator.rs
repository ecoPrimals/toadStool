// SPDX-License-Identifier: AGPL-3.0-or-later
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

#[cfg(test)]
mod tests {
    use super::*;

    fn default_validator() -> InputValidator {
        InputValidator::new(ValidationRules::default())
    }

    #[test]
    fn valid_input_passes() {
        assert!(default_validator().validate_input("hello world").is_ok());
    }

    #[test]
    fn rejects_script_tag() {
        assert!(default_validator().validate_input("<script>").is_err());
    }

    #[test]
    fn rejects_javascript_protocol() {
        assert!(
            default_validator()
                .validate_input("javascript:void(0)")
                .is_err()
        );
    }

    #[test]
    fn rejects_sql_injection_union() {
        assert!(
            default_validator()
                .validate_input("1 UNION SELECT *")
                .is_err()
        );
    }

    #[test]
    fn rejects_command_injection_semicolon() {
        assert!(default_validator().validate_input("foo; rm -rf /").is_err());
    }

    #[test]
    fn rejects_command_injection_pipe() {
        assert!(
            default_validator()
                .validate_input("foo | cat /etc/passwd")
                .is_err()
        );
    }

    #[test]
    fn rejects_input_exceeding_max_length() {
        let rules = ValidationRules {
            max_input_length: 5,
            ..ValidationRules::default()
        };
        let validator = InputValidator::new(rules);
        assert!(validator.validate_input("123456").is_err());
        assert!(validator.validate_input("12345").is_ok());
    }

    #[test]
    fn case_insensitive_pattern_matching() {
        assert!(default_validator().validate_input("<SCRIPT>").is_err());
        assert!(default_validator().validate_input("JAVASCRIPT:").is_err());
    }

    #[test]
    fn sanitize_html_entities() {
        let s = default_validator().sanitize_input("<div class=\"x\">&'test");
        assert!(!s.contains('<'));
        assert!(!s.contains('>'));
        assert!(!s.contains('"'));
        assert!(s.contains("&lt;"));
        assert!(s.contains("&gt;"));
        assert!(s.contains("&quot;"));
        assert!(s.contains("&#x27;"));
    }

    #[test]
    fn sanitize_removes_null_bytes() {
        let s = default_validator().sanitize_input("hello\0world");
        assert!(!s.contains('\0'));
        assert!(s.contains("helloworld"));
    }

    #[test]
    fn sanitize_truncates_long_input() {
        let rules = ValidationRules {
            max_input_length: 10,
            ..ValidationRules::default()
        };
        let validator = InputValidator::new(rules);
        let s = validator.sanitize_input("a".repeat(100).as_str());
        assert!(s.len() <= 10);
    }

    #[test]
    fn empty_input_is_valid() {
        assert!(default_validator().validate_input("").is_ok());
    }

    #[test]
    fn empty_blocked_patterns_allows_everything() {
        let rules = ValidationRules {
            max_input_length: 1024,
            allowed_characters: None,
            blocked_patterns: vec![],
            sql_injection_patterns: vec![],
            xss_patterns: vec![],
            command_injection_patterns: vec![],
        };
        let validator = InputValidator::new(rules);
        assert!(validator.validate_input("<script>; DROP TABLE").is_ok());
    }
}
