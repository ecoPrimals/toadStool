// SPDX-License-Identifier: AGPL-3.0-or-later
//! Test Environment Fixture - Enables Parallel Testing
//!
//! This module provides isolated environment variable management for tests,
//! eliminating the need for #[serial] annotations.
//!
//! Instead of mutating the actual environment (which requires serial execution),
//! tests use an isolated HashMap that simulates environment variables.

use std::collections::HashMap;

/// Isolated environment for testing
///
/// Provides an isolated environment that doesn't affect the actual process environment,
/// allowing tests to run in parallel without conflicts.
#[derive(Debug, Clone, Default)]
pub struct TestEnv {
    vars: HashMap<String, String>,
}

impl TestEnv {
    /// Create a new isolated test environment
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
        }
    }

    /// Set a variable in the isolated environment
    pub fn set<K: Into<String>, V: Into<String>>(&mut self, key: K, value: V) {
        self.vars.insert(key.into(), value.into());
    }

    /// Get a variable from the isolated environment
    pub fn get(&self, key: &str) -> Option<&String> {
        self.vars.get(key)
    }

    /// Remove a variable from the isolated environment
    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.vars.remove(key)
    }

    /// Check if a variable exists in the isolated environment
    #[allow(dead_code)]
    pub fn contains(&self, key: &str) -> bool {
        self.vars.contains_key(key)
    }

    /// Get all variables as a HashMap
    #[allow(dead_code)]
    pub fn as_map(&self) -> &HashMap<String, String> {
        &self.vars
    }

    /// Create a TestEnv with common TOADSTOOL variables
    pub fn with_toadstool_defaults() -> Self {
        let mut env = Self::new();
        env.set("TOADSTOOL_ENVIRONMENT", "test");
        env.set("TOADSTOOL_LOG_LEVEL", "debug");
        env
    }

    /// Merge with another TestEnv, taking values from other
    pub fn merge(&mut self, other: &TestEnv) {
        for (k, v) in &other.vars {
            self.vars.insert(k.clone(), v.clone());
        }
    }
}

/// Helper to get value with fallback to actual environment
///
/// This allows tests to gradually migrate from std::env to TestEnv
#[allow(dead_code)]
pub fn get_or_env(test_env: &TestEnv, key: &str) -> Option<String> {
    test_env
        .get(key)
        .cloned()
        .or_else(|| std::env::var(key).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_basic_operations() {
        let mut env = TestEnv::new();

        env.set("TEST_KEY", "test_value");
        assert_eq!(env.get("TEST_KEY"), Some(&"test_value".to_string()));

        env.remove("TEST_KEY");
        assert_eq!(env.get("TEST_KEY"), None);
    }

    #[test]
    fn test_env_isolation() {
        // These tests can run in parallel - they don't affect each other
        let mut env1 = TestEnv::new();
        let mut env2 = TestEnv::new();

        env1.set("KEY", "value1");
        env2.set("KEY", "value2");

        assert_eq!(env1.get("KEY"), Some(&"value1".to_string()));
        assert_eq!(env2.get("KEY"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_env_with_defaults() {
        let env = TestEnv::with_toadstool_defaults();
        assert_eq!(env.get("TOADSTOOL_ENVIRONMENT"), Some(&"test".to_string()));
        assert_eq!(env.get("TOADSTOOL_LOG_LEVEL"), Some(&"debug".to_string()));
    }

    #[test]
    fn test_env_merge() {
        let mut env1 = TestEnv::new();
        let mut env2 = TestEnv::new();

        env1.set("KEY1", "value1");
        env2.set("KEY2", "value2");

        env1.merge(&env2);

        assert_eq!(env1.get("KEY1"), Some(&"value1".to_string()));
        assert_eq!(env1.get("KEY2"), Some(&"value2".to_string()));
    }
}
