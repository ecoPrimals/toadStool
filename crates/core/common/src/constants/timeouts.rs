// SPDX-License-Identifier: AGPL-3.0-only
//! Timeout constants
//!
//! Centralized timeout values for various operations across ToadStool.

use std::time::Duration;

// ============================================================================
// Network Timeouts
// ============================================================================

/// Default HTTP request timeout (30 seconds)
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Short HTTP request timeout (5 seconds)
pub const SHORT_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// Long HTTP request timeout (2 minutes)
pub const LONG_REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// Connection/startup timeout (10 seconds). Used for IPC and service startup.
/// Previously named `WS_CONNECT_TIMEOUT`; `WebSocket` has been removed.
pub const CONNECTION_STARTUP_TIMEOUT: Duration = Duration::from_secs(10);

/// Health check timeout (3 seconds)
pub const HEALTH_CHECK_TIMEOUT: Duration = Duration::from_secs(3);

// ============================================================================
// Operation Timeouts
// ============================================================================

/// Biome startup timeout (60 seconds)
pub const BIOME_STARTUP_TIMEOUT: Duration = Duration::from_secs(60);

/// Biome shutdown timeout (30 seconds)
pub const BIOME_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(30);

/// Workload execution timeout (5 minutes)
pub const WORKLOAD_EXECUTION_TIMEOUT: Duration = Duration::from_secs(300);

/// Migration operation timeout (10 minutes)
pub const MIGRATION_TIMEOUT: Duration = Duration::from_secs(600);

/// Backup operation timeout (30 minutes)
pub const BACKUP_TIMEOUT: Duration = Duration::from_secs(1800);

// ============================================================================
// Retry Timeouts
// ============================================================================

/// Initial retry delay (100ms)
pub const INITIAL_RETRY_DELAY: Duration = Duration::from_millis(100);

/// Maximum retry delay (30 seconds)
pub const MAX_RETRY_DELAY: Duration = Duration::from_secs(30);

/// Exponential backoff multiplier
pub const RETRY_BACKOFF_MULTIPLIER: u32 = 2;

/// Maximum retry attempts
pub const MAX_RETRY_ATTEMPTS: u32 = 3;

// ============================================================================
// Polling Intervals
// ============================================================================

/// Health check interval (30 seconds)
pub const HEALTH_CHECK_INTERVAL: Duration = Duration::from_secs(30);

/// Metrics collection interval (10 seconds)
pub const METRICS_INTERVAL: Duration = Duration::from_secs(10);

/// Heartbeat interval (60 seconds)
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(60);

/// Discovery interval (5 minutes)
pub const DISCOVERY_INTERVAL: Duration = Duration::from_secs(300);

// ============================================================================
// Cache Timeouts
// ============================================================================

/// Default cache TTL (5 minutes)
pub const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(300);

/// Short cache TTL (30 seconds)
pub const SHORT_CACHE_TTL: Duration = Duration::from_secs(30);

/// Long cache TTL (1 hour)
pub const LONG_CACHE_TTL: Duration = Duration::from_secs(3600);

// ============================================================================
// Connection Timeouts
// ============================================================================

/// TCP connection timeout (5 seconds)
pub const TCP_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Connection pool idle timeout (5 minutes)
pub const POOL_IDLE_TIMEOUT: Duration = Duration::from_secs(300);

/// Connection pool max lifetime (30 minutes)
pub const POOL_MAX_LIFETIME: Duration = Duration::from_secs(1800);

// ============================================================================
// Zero-Config Deployment Timeouts
// ============================================================================

/// System discovery phase timeout (15 seconds)
pub const DISCOVERY_PHASE_TIMEOUT: Duration = Duration::from_secs(15);

/// Ecosystem discovery phase timeout (15 seconds)
pub const ECOSYSTEM_PHASE_TIMEOUT: Duration = Duration::from_secs(15);

/// Configuration generation phase timeout (10 seconds)
pub const CONFIG_PHASE_TIMEOUT: Duration = Duration::from_secs(10);

/// Service deployment phase timeout (15 seconds)
pub const DEPLOYMENT_PHASE_TIMEOUT: Duration = Duration::from_secs(15);

/// Health verification phase timeout (5 seconds)
pub const VERIFICATION_PHASE_TIMEOUT: Duration = Duration::from_secs(5);

/// Total zero-config bootstrap target (60 seconds)
pub const ZERO_CONFIG_TARGET: Duration = Duration::from_secs(60);

// ============================================================================
// Authentication Timeouts
// ============================================================================

/// Token refresh interval (1 hour)
pub const TOKEN_REFRESH_INTERVAL: Duration = Duration::from_secs(3600);

/// Timestamp validation window (5 minutes)
/// Used for replay protection - messages older than this are rejected
pub const TIMESTAMP_VALIDATION_WINDOW: Duration = Duration::from_secs(300);
