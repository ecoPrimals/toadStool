#![deny(unsafe_code)]

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
pub mod scheduler;

pub use error::OrchestrationError;
pub use load_balancer::{BalancingStrategy, LoadBalancer, Substrate};
pub use orchestrator::{
    OrchestratorStats, PerformanceHistory, PerformanceTarget, SubstrateHandle,
    WorkloadOrchestrator, WorkloadRequest, WorkloadRequestBuilder, WorkloadResult,
};
pub use policy::SelectionPolicy;
pub use scheduler::{ExecutionSchedule, ScheduledTask, SchedulingStrategy, WorkloadScheduler};
