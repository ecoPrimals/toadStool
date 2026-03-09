// SPDX-License-Identifier: AGPL-3.0-only
//! Security test fixtures for integration testing
//!
//! Test fixtures may use `expect()` for setup operations, as test setup
//! failures should fail fast.

#![allow(clippy::expect_used)] // Test fixtures may expect on setup

use std::path::PathBuf;
use tempfile::TempDir;

/// Test environment that provides temporary directories and cleanup
pub struct TestEnvironment {
    pub temp_dir: TempDir,
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
}

impl TestEnvironment {
    /// Create a new test environment with temporary directories
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let base = temp_dir.path();

        let config_dir = base.join("config");
        let data_dir = base.join("data");
        let cache_dir = base.join("cache");

        std::fs::create_dir_all(&config_dir).expect("Failed to create config dir");
        std::fs::create_dir_all(&data_dir).expect("Failed to create data dir");
        std::fs::create_dir_all(&cache_dir).expect("Failed to create cache dir");

        Self {
            temp_dir,
            config_dir,
            data_dir,
            cache_dir,
        }
    }

    /// Get the base path of the test environment
    pub fn base_path(&self) -> &std::path::Path {
        self.temp_dir.path()
    }
}

impl Default for TestEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a test security policy for integration testing
pub fn create_test_security_policy() -> serde_json::Value {
    serde_json::json!({
        "id": "test_policy",
        "name": "Test Security Policy",
        "version": "1.0.0",
        "rules": [
            {
                "id": "allow_wasm",
                "effect": "allow",
                "resource": "runtime:wasm",
                "actions": ["execute"]
            },
            {
                "id": "deny_network",
                "effect": "deny",
                "resource": "network:external",
                "actions": ["connect"]
            }
        ]
    })
}

/// Create a permissive test policy that allows most actions
pub fn create_permissive_test_policy() -> serde_json::Value {
    serde_json::json!({
        "id": "permissive_policy",
        "name": "Permissive Test Policy",
        "version": "1.0.0",
        "rules": [
            {
                "id": "allow_all",
                "effect": "allow",
                "resource": "*",
                "actions": ["*"]
            }
        ]
    })
}

/// Create a restrictive test policy that denies most actions
pub fn create_restrictive_test_policy() -> serde_json::Value {
    serde_json::json!({
        "id": "restrictive_policy",
        "name": "Restrictive Test Policy",
        "version": "1.0.0",
        "rules": [
            {
                "id": "deny_all",
                "effect": "deny",
                "resource": "*",
                "actions": ["*"]
            }
        ]
    })
}

/// Test security context builder
pub struct TestSecurityContextBuilder {
    user_id: String,
    permissions: Vec<String>,
    isolation_level: String,
}

impl TestSecurityContextBuilder {
    pub fn new() -> Self {
        Self {
            user_id: "test_user".to_string(),
            permissions: vec!["execute".to_string()],
            isolation_level: "standard".to_string(),
        }
    }

    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = user_id.into();
        self
    }

    pub fn with_permissions(mut self, permissions: Vec<String>) -> Self {
        self.permissions = permissions;
        self
    }

    pub fn with_isolation_level(mut self, level: impl Into<String>) -> Self {
        self.isolation_level = level.into();
        self
    }

    pub fn build(self) -> serde_json::Value {
        serde_json::json!({
            "user_id": self.user_id,
            "permissions": self.permissions,
            "isolation_level": self.isolation_level,
        })
    }
}

impl Default for TestSecurityContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Write test policy to a file
pub fn write_test_policy(policy: &serde_json::Value, path: &PathBuf) -> std::io::Result<()> {
    std::fs::write(path, serde_json::to_string_pretty(policy)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_security_policy() {
        let policy = create_test_security_policy();
        assert_eq!(policy["id"], "test_policy");
        assert!(policy["rules"].is_array());
    }

    #[test]
    fn test_security_context_builder() {
        let context = TestSecurityContextBuilder::new()
            .with_user_id("custom_user")
            .with_permissions(vec!["read".to_string(), "write".to_string()])
            .build();

        assert_eq!(context["user_id"], "custom_user");
        assert_eq!(context["permissions"].as_array().unwrap().len(), 2);
    }
}
