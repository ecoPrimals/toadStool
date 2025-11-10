//! Universal Compute Operations - Advanced Substrate Management
//!
//! Advanced operations for universal compute platform management:
//! - Substrate detection and testing
//! - Performance benchmarking
//! - Workload migration between platforms
//! - Federation with other `ToadStool` instances
//!
//! ## Module Structure
//!
//! - `types`: All type definitions for platforms, benchmarks, federation, and migration
//! - `manager`: Core manager implementation with all operations

// Type definitions
pub mod types;

// Operation traits
pub mod operations;

// Re-export all types for backward compatibility
pub use types::{
    BenchmarkResult, BenchmarkTest, BenchmarkType, DetectedPlatform, FederationPeer,
    FederationStatus, GpuInfo, HardwareInfo, MigrationPlan, MigrationType, PlatformStatus,
    ReplicationHandle, SystemInfo, TrustLevel, WorkloadCheckpoint, WorkloadExport,
    WorkloadSnapshot,
};

// Core imports for manager
use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::fs;
use tracing::{error, info, warn};

use toadstool_distributed::substrate_detection::{SubstrateCapabilities, SubstrateDetector};

/// Universal compute operations manager
pub struct UniversalComputeManager {
    /// Substrate detector
    detector: SubstrateDetector,
    /// Detected platforms
    platforms: HashMap<String, DetectedPlatform>,
    /// Benchmark results
    benchmarks: HashMap<String, BenchmarkResult>,
    /// Federation connections
    federation_peers: HashMap<String, FederationPeer>,
}

// Include the implementation from manager_impl.rs
// This keeps the file size manageable while preserving all functionality
include!("manager_impl.rs");
