// SPDX-License-Identifier: AGPL-3.0-only
#![deny(unsafe_code)]
#![allow(async_fn_in_trait)]
#![allow(
    clippy::must_use_candidate,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

//! Performance Management and Optimization for `ToadStool`
//!
//! This crate provides comprehensive performance management including:
//! - Runtime selection algorithms with intelligent workload routing
//! - Performance profiling and metrics collection
//! - Resource pool management and optimization
//! - Usage prediction and recommendation engines
//!
//! ## Architecture
//!
//! The crate is organized into focused modules:
//! - `types`: Core types, configuration, and data structures
//! - `optimizer`: Trait definition for performance optimization
//! - `scoring`: Performance and efficiency scoring algorithms
//! - `implementation`: Main optimizer implementation and domain logic
//!
//! ## Usage
//!
//! ```rust,ignore
//! use toadstool_management_performance::{
//!     IntelligentPerformanceOptimizer,
//!     PerformanceConfig,
//!     RuntimeSelectionStrategy,
//! };
//!
//! let config = PerformanceConfig::default();
//! let strategy = RuntimeSelectionStrategy::FastestExecution;
//! let optimizer = IntelligentPerformanceOptimizer::new(config, strategy);
//! ```

mod implementation;

pub mod optimizer;
pub mod scoring;
pub mod types;

// Re-export main types and traits
pub use implementation::IntelligentPerformanceOptimizer;
pub use optimizer::PerformanceOptimizer;
pub use scoring::{calculate_efficiency_score, calculate_performance_score};
pub use types::{
    OptimizationRecommendation, PerformanceConfig, PerformanceMetrics, RecommendationType,
    ResourcePrediction, RuntimeSelectionStrategy, RuntimeStats, SelectionWeights,
};
