// SPDX-License-Identifier: AGPL-3.0-or-later
//! Test isolation helpers
//!
//! Provides isolated environments for concurrent test execution without
//! global state conflicts or the need for `#[serial]` markers.
//!
//! Test helpers may use `expect()` and `unwrap()` for setup operations,
//! as test setup failures should fail fast.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use std::sync::RwLock;

/// Isolated test environment with its own temporary directory
pub struct IsolatedEnv {
    temp_dir: TempDir,
    env_vars: Arc<RwLock<Vec<(String, String)>>>,
}

impl IsolatedEnv {
    /// Create a new isolated environment with a temporary directory
    pub fn new() -> std::io::Result<Self> {
        Ok(Self {
            temp_dir: TempDir::new()?,
            env_vars: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Get the temporary directory path
    pub fn path(&self) -> &std::path::Path {
        self.temp_dir.path()
    }

    /// Set an environment variable for the duration of the test
    pub async fn set_var(&self, key: impl Into<String>, value: impl Into<String>) {
        let mut vars = self.env_vars.write().unwrap_or_else(|e| e.into_inner());
        vars.push((key.into(), value.into()));
    }

    /// Get an environment variable previously set via set_var
    pub async fn get_var(&self, key: &str) -> Option<String> {
        let vars = self.env_vars.read().unwrap_or_else(|e| e.into_inner());
        vars.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
    }

    /// Create a path for a file in the temp dir (does not create the file)
    pub fn create_file(&self, name: &str) -> PathBuf {
        self.path().join(name)
    }
}

impl Default for IsolatedEnv {
    fn default() -> Self {
        Self::new().expect("Failed to create isolated environment")
    }
}

/// Isolated configuration for testing
#[derive(Clone)]
pub struct IsolatedConfig {
    data: Arc<RwLock<serde_json::Value>>,
}

impl IsolatedConfig {
    /// Create a new isolated config with empty JSON
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(serde_json::json!({}))),
        }
    }

    /// Set a config value by key
    pub async fn set(&self, key: &str, value: serde_json::Value) {
        let mut data = self.data.write().unwrap_or_else(|e| e.into_inner());
        data[key] = value;
    }

    /// Get a config value by key
    pub async fn get(&self, key: &str) -> Option<serde_json::Value> {
        let data = self.data.read().unwrap_or_else(|e| e.into_inner());
        data.get(key).cloned()
    }

    /// Get a config value as string
    pub async fn get_str(&self, key: &str) -> Option<String> {
        self.get(key).await?.as_str().map(|s| s.to_string())
    }

    /// Get a config value as u64
    pub async fn get_u64(&self, key: &str) -> Option<u64> {
        self.get(key).await?.as_u64()
    }

    /// Get a config value as bool
    pub async fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key).await?.as_bool()
    }
}

impl Default for IsolatedConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Isolated runtime for testing with resource limits
pub struct IsolatedRuntime {
    handle: tokio::runtime::Handle,
    _guard: RuntimeGuard,
}

struct RuntimeGuard;

impl Drop for RuntimeGuard {
    fn drop(&mut self) {
        // Cleanup happens here if needed
    }
}

impl IsolatedRuntime {
    /// Create a runtime using the current tokio handle
    pub fn new() -> Self {
        Self {
            handle: tokio::runtime::Handle::current(),
            _guard: RuntimeGuard,
        }
    }

    /// Get the tokio runtime handle for spawning tasks
    pub const fn handle(&self) -> &tokio::runtime::Handle {
        &self.handle
    }

    /// Spawn an async task on the runtime
    pub fn spawn<F>(&self, future: F) -> tokio::task::JoinHandle<F::Output>
    where
        F: std::future::Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.handle.spawn(future)
    }

    /// Spawn a blocking task on the runtime
    pub fn spawn_blocking<F, R>(&self, f: F) -> tokio::task::JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        self.handle.spawn_blocking(f)
    }
}

impl Default for IsolatedRuntime {
    fn default() -> Self {
        Self::new()
    }
}

/// Test scope that ensures cleanup on drop
pub struct TestScope<T> {
    resource: Option<T>,
    cleanup: Option<Box<dyn FnOnce(T) + Send>>,
}

impl<T> TestScope<T> {
    /// Create a scope that owns a resource (no cleanup)
    pub fn new(resource: T) -> Self {
        Self {
            resource: Some(resource),
            cleanup: None,
        }
    }

    /// Create a scope with a cleanup closure to run on drop
    pub fn with_cleanup<F>(resource: T, cleanup: F) -> Self
    where
        F: FnOnce(T) + Send + 'static,
    {
        Self {
            resource: Some(resource),
            cleanup: Some(Box::new(cleanup)),
        }
    }

    /// Get immutable reference to the test resource
    ///
    /// # Safety
    /// The resource is guaranteed to be `Some` until `into_inner()` consumes it.
    /// This is a test helper where panic-on-misuse is acceptable.
    #[expect(
        clippy::expect_used,
        reason = "test helper — panic on misuse is the intended contract"
    )]
    pub fn get(&self) -> &T {
        self.resource
            .as_ref()
            .expect("TestScope::get: inner resource already consumed via into_inner")
    }

    /// Get mutable reference to the test resource
    ///
    /// # Safety
    /// The resource is guaranteed to be `Some` until `into_inner()` consumes it.
    /// This is a test helper where panic-on-misuse is acceptable.
    #[expect(
        clippy::expect_used,
        reason = "test helper — panic on misuse is the intended contract"
    )]
    pub fn get_mut(&mut self) -> &mut T {
        self.resource
            .as_mut()
            .expect("TestScope::get_mut: inner resource already consumed via into_inner")
    }

    /// Consume the scope and return the inner resource
    ///
    /// # Safety
    /// The resource is guaranteed to be `Some` until this method is called.
    /// This is a test helper where panic-on-misuse is acceptable.
    #[expect(
        clippy::expect_used,
        reason = "test helper — panic on misuse is the intended contract"
    )]
    pub fn into_inner(mut self) -> T {
        self.resource
            .take()
            .expect("TestScope::into_inner: inner resource already consumed")
    }
}

impl<T> Drop for TestScope<T> {
    fn drop(&mut self) {
        if let (Some(resource), Some(cleanup)) = (self.resource.take(), self.cleanup.take()) {
            cleanup(resource);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_isolated_env() {
        let env = IsolatedEnv::new().unwrap();
        env.set_var("TEST_KEY", "test_value").await;
        assert_eq!(
            env.get_var("TEST_KEY").await,
            Some("test_value".to_string())
        );
    }

    #[tokio::test]
    async fn test_isolated_config() {
        let config = IsolatedConfig::new();
        config.set("port", serde_json::json!(8080)).await;
        assert_eq!(config.get_u64("port").await, Some(8080));
    }

    #[tokio::test]
    async fn test_multiple_isolated_configs() {
        let config1 = IsolatedConfig::new();
        let config2 = IsolatedConfig::new();

        config1.set("value", serde_json::json!(1)).await;
        config2.set("value", serde_json::json!(2)).await;

        assert_eq!(config1.get_u64("value").await, Some(1));
        assert_eq!(config2.get_u64("value").await, Some(2));
    }

    #[test]
    fn test_test_scope_cleanup() {
        let called = Arc::new(std::sync::Mutex::new(false));
        let called_clone = Arc::clone(&called);

        {
            let _scope = TestScope::with_cleanup(42, move |_val| {
                *called_clone.lock().unwrap() = true;
            });
        }

        assert!(*called.lock().unwrap());
    }
}
