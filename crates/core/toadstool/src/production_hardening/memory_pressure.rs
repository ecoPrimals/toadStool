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
#[async_trait]
pub trait MemoryPressureCallback: Send + Sync {
    async fn handle_pressure(&self, level: MemoryPressureLevel, usage_percent: f64);
}

/// Memory pressure handler
pub struct MemoryPressureHandler {
    config: MemoryPressureConfig,
    current_usage: Arc<RwLock<u64>>,
    callbacks: Arc<RwLock<Vec<Box<dyn MemoryPressureCallback>>>>,
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

    pub async fn register_callback(&self, callback: Box<dyn MemoryPressureCallback>) {
        let mut callbacks = self.callbacks.write().await;
        callbacks.push(callback);
    }

    pub async fn update_memory_usage(&self, total_memory: u64, used_memory: u64) {
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
            let callbacks = self.callbacks.read().await;
            for callback in callbacks.iter() {
                callback.handle_pressure(level, usage_percent).await;
            }
        }
    }

    pub async fn get_pressure_level(&self) -> MemoryPressureLevel {
        MemoryPressureLevel::Normal
    }
}

/// Default memory pressure callback
pub struct DefaultMemoryPressureCallback;

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
