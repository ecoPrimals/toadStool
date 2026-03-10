// SPDX-License-Identifier: AGPL-3.0-only
#![deny(unsafe_code)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::match_same_arms,
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::needless_continue,
    clippy::return_self_not_must_use,
    clippy::unnecessary_wraps,
    clippy::unused_async,
    clippy::unused_self,
    clippy::wildcard_imports
)]

//! Workload Orchestration System
//!
//! **Deep Debt**: Intelligent workload distribution across compute substrates
//!
//! This crate provides a sophisticated orchestration layer that:
//! - Discovers available substrates at runtime
//! - Analyzes workload characteristics
//! - Selects optimal substrate(s) for execution
//! - Load balances across multiple substrates
//! - Handles failures and fallbacks
//! - Learns from actual performance
//!
//! # Architecture
//!
//! ```text
//!                  ┌─────────────────┐
//!                  │  Orchestrator   │
//!                  └────────┬────────┘
//!                           │
//!            ┌──────────────┼──────────────┐
//!            │              │              │
//!      ┌─────▼────┐   ┌────▼─────┐  ┌────▼─────┐
//!      │   CPU    │   │   GPU    │  │   NPU    │
//!      │ Substrate│   │Substrate │  │Substrate │
//!      └──────────┘   └──────────┘  └──────────┘
//! ```
//!
//! # Example
//!
//! ```rust,no_run
//! use toadstool_runtime_orchestration::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), OrchestrationError> {
//!     // Discover all available substrates
//!     let orchestrator = WorkloadOrchestrator::discover().await?;
//!     
//!     println!("Available substrates: {}", orchestrator.num_substrates());
//!     
//!     // Execute workload with automatic substrate selection
//!     let workload = WorkloadRequest::new()
//!         .operation_count(10_000)
//!         .power_budget_watts(50.0)
//!         .target_latency()
//!         .build()?;
//!     
//!     let result = orchestrator.execute(workload).await?;
//!     println!("Executed on: {}", result.substrate_name);
//!     println!("Duration: {:?}", result.duration);
//!     
//!     Ok(())
//! }
//! ```

pub mod error;
pub mod load_balancer;
pub mod orchestrator;
pub mod policy;
pub mod resource_orchestrator;
pub mod scheduler;

pub use error::OrchestrationError;
pub use load_balancer::{BalancingStrategy, LoadBalancer, Substrate};
pub use orchestrator::{
    OrchestratorStats, PerformanceHistory, PerformanceTarget, SubstrateHandle,
    WorkloadOrchestrator, WorkloadRequest, WorkloadRequestBuilder, WorkloadResult,
};
pub use policy::SelectionPolicy;
pub use resource_orchestrator::{
    AvailableDevice, DeploymentModel, ResourceAllocation, ResourceOrchestrator, ResourceRequest,
    TenantQuota, TenantUsage,
};
pub use scheduler::{ExecutionSchedule, ScheduledTask, SchedulingStrategy, WorkloadScheduler};
