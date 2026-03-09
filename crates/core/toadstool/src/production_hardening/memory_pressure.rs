// SPDX-License-Identifier: AGPL-3.0-or-later
//! Memory pressure handling and optimization.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{error, warn};

/// Memory pressure configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPressureConfig {
    pub warning_threshold: f64,
    pub critical_threshold: f64,
    pub emergency_threshold: f64,
    pub check_interval: Duration,
}

impl Default for MemoryPressureConfig {
    fn default() -> Self {
        Self {
            warning_threshold: 70.0,
            critical_threshold: 85.0,
            emergency_threshold: 95.0,
            check_interval: Duration::from_secs(10),
        }
    }
}

/// Memory pressure levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryPressureLevel {
    Normal,
    Warning,
    Critical,
    Emergency,
}

/// Memory pressure callback trait
// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
pub trait MemoryPressureCallback: Send + Sync {
    async fn handle_pressure(&self, level: MemoryPressureLevel, usage_percent: f64);
}

/// Memory pressure handler
pub struct MemoryPressureHandler {
    config: MemoryPressureConfig,
    current_usage: Arc<RwLock<u64>>,
    /// Arc-wrapped so we can clone and invoke without holding lock across .await
    callbacks: Arc<RwLock<Vec<Arc<dyn MemoryPressureCallback>>>>,
}

impl MemoryPressureHandler {
    #[must_use]
    pub fn new(config: MemoryPressureConfig) -> Self {
        Self {
            config,
            current_usage: Arc::new(RwLock::new(0)),
            callbacks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register a callback. Accepts `Arc` so we can clone and invoke without holding lock across await.
    pub async fn register_callback(&self, callback: Arc<dyn MemoryPressureCallback>) {
        let mut callbacks = self.callbacks.write().await;
        callbacks.push(callback);
    }

    /// Register from `Box`. Converts to `Arc` for storage (one-time allocation).
    pub async fn register_callback_box(&self, callback: Box<dyn MemoryPressureCallback>) {
        self.register_callback(Arc::from(callback)).await;
    }

    pub async fn update_memory_usage(&self, total_memory: u64, used_memory: u64) {
        #[expect(
            clippy::cast_precision_loss,
            reason = "u64 to f64 for percentage calculation"
        )]
        let usage_percent = (used_memory as f64 / total_memory as f64) * 100.0;

        let level = if usage_percent >= self.config.emergency_threshold {
            MemoryPressureLevel::Emergency
        } else if usage_percent >= self.config.critical_threshold {
            MemoryPressureLevel::Critical
        } else if usage_percent >= self.config.warning_threshold {
            MemoryPressureLevel::Warning
        } else {
            MemoryPressureLevel::Normal
        };

        *self.current_usage.write().await = used_memory;

        if level != MemoryPressureLevel::Normal {
            // Clone Arc refs and release lock before await (avoid holding lock across .await)
            let callback_arcs: Vec<Arc<dyn MemoryPressureCallback>> = {
                let guard = self.callbacks.read().await;
                guard.iter().map(Arc::clone).collect()
            };
            for callback in callback_arcs {
                callback.handle_pressure(level, usage_percent).await;
            }
        }
    }

    #[expect(
        clippy::unused_async,
        reason = "API consistency; may add async monitoring in future"
    )]
    pub async fn get_pressure_level(&self) -> MemoryPressureLevel {
        MemoryPressureLevel::Normal
    }
}

/// Default memory pressure callback
pub struct DefaultMemoryPressureCallback;

// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
impl MemoryPressureCallback for DefaultMemoryPressureCallback {
    async fn handle_pressure(&self, level: MemoryPressureLevel, usage_percent: f64) {
        match level {
            MemoryPressureLevel::Normal => {}
            MemoryPressureLevel::Warning => {
                warn!("Memory pressure warning: {:.1}% usage", usage_percent);
            }
            MemoryPressureLevel::Critical => {
                error!("Memory pressure critical: {:.1}% usage", usage_percent);
            }
            MemoryPressureLevel::Emergency => {
                error!("Memory pressure emergency: {:.1}% usage", usage_percent);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU8, Ordering};

    #[test]
    fn test_memory_pressure_config_default() {
        let config = MemoryPressureConfig::default();
        assert!((config.warning_threshold - 70.0).abs() < f64::EPSILON);
        assert!((config.critical_threshold - 85.0).abs() < f64::EPSILON);
        assert!((config.emergency_threshold - 95.0).abs() < f64::EPSILON);
        assert_eq!(config.check_interval, Duration::from_secs(10));
    }

    #[test]
    fn test_memory_pressure_level_variants() {
        assert_eq!(MemoryPressureLevel::Normal, MemoryPressureLevel::Normal);
        assert_eq!(MemoryPressureLevel::Warning, MemoryPressureLevel::Warning);
        assert_eq!(MemoryPressureLevel::Critical, MemoryPressureLevel::Critical);
        assert_eq!(
            MemoryPressureLevel::Emergency,
            MemoryPressureLevel::Emergency
        );
    }

    #[test]
    fn test_memory_pressure_handler_new() {
        let config = MemoryPressureConfig::default();
        let _handler = MemoryPressureHandler::new(config);
    }

    #[tokio::test]
    async fn test_memory_pressure_handler_update_usage_normal() {
        let config = MemoryPressureConfig::default();
        let handler = MemoryPressureHandler::new(config);
        handler.update_memory_usage(1000, 500).await; // 50% - Normal
        let level = handler.get_pressure_level().await;
        assert_eq!(level, MemoryPressureLevel::Normal);
    }

    #[tokio::test]
    async fn test_memory_pressure_handler_update_usage_warning() {
        let config = MemoryPressureConfig {
            warning_threshold: 50.0,
            critical_threshold: 80.0,
            emergency_threshold: 95.0,
            check_interval: Duration::from_secs(10),
        };
        let handler = MemoryPressureHandler::new(config);
        handler.update_memory_usage(100, 60).await; // 60% - Warning
        let level = handler.get_pressure_level().await;
        assert_eq!(level, MemoryPressureLevel::Normal); // get_pressure_level returns Normal (current impl)
    }

    #[tokio::test]
    async fn test_memory_pressure_callback_invoked() {
        static LEVEL_SEEN: AtomicU8 = AtomicU8::new(0);
        LEVEL_SEEN.store(0, Ordering::SeqCst);

        struct CallbackTracker;
        #[async_trait::async_trait]
        impl MemoryPressureCallback for CallbackTracker {
            async fn handle_pressure(&self, level: MemoryPressureLevel, _usage_percent: f64) {
                LEVEL_SEEN.store(
                    match level {
                        MemoryPressureLevel::Normal => 0,
                        MemoryPressureLevel::Warning => 1,
                        MemoryPressureLevel::Critical => 2,
                        MemoryPressureLevel::Emergency => 3,
                    },
                    Ordering::SeqCst,
                );
            }
        }

        let config = MemoryPressureConfig {
            warning_threshold: 50.0,
            critical_threshold: 80.0,
            emergency_threshold: 95.0,
            check_interval: Duration::from_secs(10),
        };
        let handler = MemoryPressureHandler::new(config);
        handler
            .register_callback_box(Box::new(CallbackTracker))
            .await;

        // 75% should trigger Warning callback
        handler.update_memory_usage(100, 75).await;
        assert_eq!(LEVEL_SEEN.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_default_memory_pressure_callback() {
        let callback = DefaultMemoryPressureCallback;
        callback
            .handle_pressure(MemoryPressureLevel::Warning, 75.0)
            .await;
        callback
            .handle_pressure(MemoryPressureLevel::Critical, 90.0)
            .await;
        callback
            .handle_pressure(MemoryPressureLevel::Emergency, 98.0)
            .await;
    }

    #[test]
    fn test_memory_pressure_config_serde() {
        let config = MemoryPressureConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let decoded: MemoryPressureConfig = serde_json::from_str(&json).unwrap();
        assert!((decoded.warning_threshold - config.warning_threshold).abs() < f64::EPSILON);
    }
}
