// SPDX-License-Identifier: AGPL-3.0-or-later
//! # AI/MCP Interface
//!
//! Universal interface for ANY AI service using Model Context Protocol (MCP).
//! Discovers AI providers at runtime via `AI_PROCESSING` capability.
//!
//! ## Supported Providers
//!
//! Works with any MCP-compatible AI service:
//! - **intelligence service** (ecoPrimals ecosystem) - discovered at runtime
//! - **Claude MCP** (Anthropic) - if advertising `AI_PROCESSING` capability
//! - **`OpenAI` API** - via MCP adapter
//! - **Custom MCP servers** - any compliant implementation
//!
//! ## Features
//!
//! - **Natural Language Configuration**: Process AI-friendly configuration requests
//! - **Intent-Based Execution**: Execute code with AI-understood intent
//! - **Task Optimization**: Optimize `ToadStool` for specific AI workloads
//! - **Context Management**: Maintain execution context across requests
//! - **AI-Friendly Responses**: Structured responses perfect for AI consumption
//! - **Runtime Discovery**: Find AI providers by capability, not by name
//!
//! ## Sovereignty
//!
//! This module maintains primal sovereignty by:
//! - Zero compile-time knowledge of specific AI providers
//! - Capability-based discovery (`AI_PROCESSING`)
//! - Dynamic learning of provider capabilities at runtime

pub mod session;
pub mod types;

#[cfg(feature = "runtime")]
mod interface;

pub use session::{AiPreferences, AiSession, ResourcePreferences};
pub use types::{
    ConfigurationSummary, ExecutionIntent, IoIntensity, McpRequest, McpRequestType, McpResponse,
    MemoryPattern, PerformanceExpectations, ResourceAllocation, ResourceHints, SessionInfo,
};

#[cfg(feature = "runtime")]
pub use interface::AiMcpInterface;

#[cfg(all(test, feature = "runtime"))]
#[path = "mod_tests.rs"]
mod tests;
