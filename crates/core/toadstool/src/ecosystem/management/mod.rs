// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Service Management
//!
//! Manages the lifecycle of ecosystem services including registration,
//! status tracking, health monitoring, and integration.
//!
//! ## Features
//!
//! - **Lifecycle Management**: Track service states from discovery to removal
//! - **Health Monitoring**: Periodic health checks and heartbeats
//! - **Status Tracking**: Real-time service status updates
//! - **Integration**: Automated service integration workflows
//!
//! ## Usage
//!
//! ```rust,ignore
//! let manager = ServiceManager::new();
//!
//! // Register service
//! manager.register_service(service).await?;
//!
//! // Check status
//! let status = manager.get_service_status(&service_id).await?;
//!
//! // Monitor health
//! manager.start_health_monitoring().await?;
//! ```

mod capabilities;
mod health;
mod lifecycle;
mod status;

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::ecosystem::types::{ServiceInstance, ServiceStatus};

/// Service manager for lifecycle and status management
pub struct ServiceManager {
    /// Registered services (keyed by service ID)
    services: Arc<RwLock<HashMap<String, ServiceInstance>>>,
    /// Service status tracking (keyed by service ID)
    statuses: Arc<RwLock<HashMap<String, ServiceStatus>>>,
}

impl ServiceManager {
    /// Create a new service manager
    pub fn new() -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
            statuses: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for ServiceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
