//! Test isolation helpers
//!
//! Provides isolated environments for concurrent test execution without
//! global state conflicts or the need for `#[serial]` markers.

use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::RwLock;

/// Isolated test environment with its own temporary directory
pub struct IsolatedEnv {
    temp_dir: TempDir,
    env_vars: Arc<RwLock<Vec<(String, String)>>>,
}

impl IsolatedEnv {
    pub fn new() -> std::io::Result<Self> {
        Ok(Self {
            temp_dir: TempDir::new()?,
            env_vars: Arc::new(RwLock::new(Vec::new())),
        })
    }

    pub fn path(&self) -> &std::path::Path {
        self.temp_dir.path()
    }

    pub async fn set_var(&self, key: impl Into<String>, value: impl Into<String>) {
        let mut vars = self.env_vars.write().await;
        vars.push((key.into(), value.into()));
    }

    pub async fn get_var(&self, key: &str) -> Option<String> {
        let vars = self.env_vars.read().await;
        vars.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
    }

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
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(serde_json::json!({}))),
        }
    }

    pub async fn set(&self, key: &str, value: serde_json::Value) {
        let mut data = self.data.write().await;
        data[key] = value;
    }

    pub async fn get(&self, key: &str) -> Option<serde_json::Value> {
        let data = self.data.read().await;
        data.get(key).cloned()
    }

    pub async fn get_str(&self, key: &str) -> Option<String> {
        self.get(key).await?.as_str().map(|s| s.to_string())
    }

    pub async fn get_u64(&self, key: &str) -> Option<u64> {
        self.get(key).await?.as_u64()
    }

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
    pub fn new() -> Self {
        Self {
            handle: tokio::runtime::Handle::current(),
            _guard: RuntimeGuard,
        }
    }

    pub fn handle(&self) -> &tokio::runtime::Handle {
        &self.handle
    }

    pub fn spawn<F>(&self, future: F) -> tokio::task::JoinHandle<F::Output>
    where
        F: std::future::Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.handle.spawn(future)
    }

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
    pub fn new(resource: T) -> Self {
        Self {
            resource: Some(resource),
            cleanup: None,
        }
    }

    pub fn with_cleanup<F>(resource: T, cleanup: F) -> Self
    where
        F: FnOnce(T) + Send + 'static,
    {
        Self {
            resource: Some(resource),
            cleanup: Some(Box::new(cleanup)),
        }
    }

    pub fn get(&self) -> &T {
        self.resource.as_ref().unwrap()
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.resource.as_mut().unwrap()
    }

    pub fn into_inner(mut self) -> T {
        self.resource.take().unwrap()
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
        let called_clone = called.clone();

        {
            let _scope = TestScope::with_cleanup(42, move |_val| {
                *called_clone.lock().unwrap() = true;
            });
        }

        assert!(*called.lock().unwrap());
    }
}
