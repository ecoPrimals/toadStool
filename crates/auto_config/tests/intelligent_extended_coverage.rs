// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(deprecated)]
#![allow(
    clippy::float_cmp,
    reason = "exact comparison intended in this context"
)]
//! Extended coverage tests for intelligent auto-configuration (platform detection,
//! optimization paths, and configuration generation).

#[path = "intelligent_extended_coverage/intelligent_extended_auto_config.rs"]
mod intelligent_extended_auto_config;
#[path = "intelligent_extended_coverage/intelligent_extended_performance_classification.rs"]
mod intelligent_extended_performance_classification;
#[path = "intelligent_extended_coverage/intelligent_extended_platform_config.rs"]
mod intelligent_extended_platform_config;
#[path = "intelligent_extended_coverage/intelligent_extended_platform_info.rs"]
mod intelligent_extended_platform_info;
#[path = "intelligent_extended_coverage/intelligent_extended_platform_optimizer.rs"]
mod intelligent_extended_platform_optimizer;
#[path = "intelligent_extended_coverage/intelligent_extended_platform_support.rs"]
mod intelligent_extended_platform_support;
#[path = "intelligent_extended_coverage/intelligent_extended_usage_hints.rs"]
mod intelligent_extended_usage_hints;
#[path = "intelligent_extended_coverage/intelligent_extended_usage_learner.rs"]
mod intelligent_extended_usage_learner;
