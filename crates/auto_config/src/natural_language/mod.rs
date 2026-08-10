// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Natural Language Configuration Interface
//!
//! Enables configuration of `ToadStool` through natural language descriptions.
//! Perfect for integration with intelligence service and AI systems that need to configure
//! compute environments through conversation.
//!
//! ## Architecture
//!
//! This module is organized by concerns:
//! - `types` - Core type definitions (preferences, intents, templates)
//! - `intent` - Intent recognition and analysis
//! - `templates` - Pre-configured templates for common use cases

pub mod intent;
pub mod templates;
pub mod types;

pub use types::*;

#[cfg(feature = "runtime")]
mod nl_config;

#[cfg(feature = "runtime")]
pub use nl_config::NaturalLanguageConfig;
