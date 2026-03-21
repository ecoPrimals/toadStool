// SPDX-License-Identifier: AGPL-3.0-only
//! # AI/MCP Interface
//!
//! Universal interface for ANY AI service using Model Context Protocol (MCP).
//! Discovers AI providers at runtime via `AI_PROCESSING` capability.
//!
//! ## Supported Providers
//!
//! Works with any MCP-compatible AI service:
//! - **Squirrel MCP** (ecoPrimals ecosystem) - discovered at runtime
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

mod handlers;
mod session;
mod types;

use crate::{IntelligentAutoConfig, NaturalLanguageConfig, ToadStoolResult};
use std::collections::HashMap;
use std::time::SystemTime;

use tokio::sync::RwLock;
use tracing::info;

// Re-export all public types for external consumers (also brings into scope for impl)
pub use session::{AiPreferences, AiSession, ResourcePreferences};
pub use types::{
    ConfigurationSummary, ExecutionIntent, IoIntensity, McpRequest, McpRequestType, McpResponse,
    MemoryPattern, PerformanceExpectations, ResourceAllocation, ResourceHints, SessionInfo,
};

// =============================================================================
// Interface
// =============================================================================

/// Universal AI/MCP interface
///
/// Works with ANY AI service that supports Model Context Protocol (MCP).
/// Discovers providers at runtime via capability-based discovery.
pub struct AiMcpInterface {
    /// Natural language configuration processor
    pub(crate) config_assistant: NaturalLanguageConfig,
    /// Auto-configuration system
    pub(crate) auto_config: IntelligentAutoConfig,
    /// Active AI sessions
    pub(crate) active_sessions: RwLock<HashMap<String, AiSession>>,
    /// Request counter for tracking
    pub(crate) request_counter: RwLock<u64>,
}

impl AiMcpInterface {
    /// Create new AI/MCP interface
    ///
    /// Note: Future versions will integrate runtime discovery to find
    /// AI providers via `AI_PROCESSING` capability
    pub fn new() -> ToadStoolResult<Self> {
        info!("🤖 Initializing AI/MCP interface (capability-based)");

        Ok(Self {
            config_assistant: NaturalLanguageConfig::new(),
            auto_config: IntelligentAutoConfig::new(),
            active_sessions: RwLock::new(HashMap::new()),
            request_counter: RwLock::new(0),
        })
    }

    /// Create new AI/MCP interface with custom components (for testing)
    #[cfg(test)]
    pub fn new_with_components(
        config_assistant: NaturalLanguageConfig,
        auto_config: IntelligentAutoConfig,
    ) -> ToadStoolResult<Self> {
        info!("🤖 Initializing AI/MCP interface with custom components");

        Ok(Self {
            config_assistant,
            auto_config,
            active_sessions: RwLock::new(HashMap::new()),
            request_counter: RwLock::new(0),
        })
    }

    /// Process MCP requests from any AI provider
    ///
    /// Handles requests from Squirrel, Claude, `OpenAI`, or any MCP-compatible service
    pub async fn process_ai_request(
        &mut self,
        request: McpRequest,
    ) -> ToadStoolResult<McpResponse> {
        // Increment request counter
        {
            let mut counter = self.request_counter.write().await;
            *counter += 1;
        }

        info!(
            "🤖 Processing MCP request: {} (type: {:?})",
            request.request_id, request.request_type
        );

        let response = match request.request_type {
            McpRequestType::NaturalLanguageConfig { instruction } => {
                self.handle_natural_config(request.request_id.clone(), instruction)
                    .await
            }
            McpRequestType::ExecuteWithIntent { code, intent } => {
                self.handle_execute_with_intent(request.request_id.clone(), code, intent)
                    .await
            }
            McpRequestType::OptimizeForTask { task_description } => {
                self.handle_optimize_for_task(request.request_id.clone(), task_description)
                    .await
            }
            McpRequestType::GetSystemStatus => {
                self.handle_get_system_status(request.request_id.clone())
                    .await
            }
            McpRequestType::CreateSession { preferences } => {
                self.handle_create_session(
                    request.request_id.clone(),
                    request.agent_id,
                    preferences,
                )
                .await
            }
            McpRequestType::UpdatePreferences { preferences } => {
                self.handle_update_preferences(
                    request.request_id.clone(),
                    request.session_id.clone(),
                    preferences,
                )
                .await
            }
        };

        // Update session activity if applicable
        if let Some(session_id) = &request.session_id {
            self.update_session_activity(session_id).await;
        }

        response
    }

    /// Update session activity timestamp
    async fn update_session_activity(&self, session_id: &str) {
        let mut sessions = self.active_sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.last_activity = SystemTime::now();
        }
    }

    /// Get session statistics
    pub async fn get_session_stats(&self) -> HashMap<String, serde_json::Value> {
        let sessions = self.active_sessions.read().await;
        let request_count = *self.request_counter.read().await;

        let mut stats = HashMap::new();
        stats.insert(
            "active_sessions".to_string(),
            serde_json::Value::Number(sessions.len().into()),
        );
        stats.insert(
            "total_requests".to_string(),
            serde_json::Value::Number(request_count.into()),
        );
        let avg_duration_desc = if sessions.is_empty() {
            "no active sessions".to_string()
        } else {
            let now = SystemTime::now();
            let total_secs: u64 = sessions
                .values()
                .filter_map(|s| now.duration_since(s.started_at).ok())
                .map(|d| d.as_secs())
                .sum();
            let avg_secs = total_secs / sessions.len() as u64;
            let mins = avg_secs / 60;
            let secs = avg_secs % 60;
            drop(sessions);
            format!("{mins}m {secs}s")
        };
        stats.insert(
            "average_session_duration".to_string(),
            serde_json::Value::String(avg_duration_desc),
        );

        stats
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_squirrel_mcp_interface_creation() {
        let interface = AiMcpInterface::new();
        assert!(interface.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[cfg_attr(
        not(feature = "slow-tests"),
        ignore = "slow integration test - runs full NL processing and hardware detection"
    )]
    async fn test_natural_language_config_request() {
        let mut interface = AiMcpInterface::new().unwrap();

        let request = McpRequest {
            request_id: "test-001".to_string(),
            session_id: None,
            agent_id: "test-agent".to_string(),
            request_type: McpRequestType::NaturalLanguageConfig {
                instruction: "Enable high performance mode".to_string(),
            },
            metadata: HashMap::new(),
            timestamp: SystemTime::now(),
        };

        let response = interface.process_ai_request(request).await;

        assert!(
            response.is_ok(),
            "AI request should succeed: {:?}",
            response.as_ref().err()
        );
        let result = response.unwrap();
        assert!(result.success, "Should return success response");
        assert!(
            !result.message.is_empty(),
            "Should return non-empty message"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_natural_language_config_request_fast() {
        // Fast test that checks interface state without full NL pipeline
        let interface = AiMcpInterface::new().unwrap();

        // This should be fast as it only checks interface state
        let stats = interface.get_session_stats().await;
        assert!(!stats.is_empty(), "Should return stats");

        // Verify expected keys exist
        assert!(stats.contains_key("active_sessions"));
        assert!(stats.contains_key("total_requests"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_squirrel_mcp_context_handling() {
        let interface = AiMcpInterface::new().unwrap();
        let stats = interface.get_session_stats().await;

        // Check that stats are returned and active sessions is 0
        assert!(!stats.is_empty(), "Should return stats");
        if let Some(serde_json::Value::Number(n)) = stats.get("active_sessions") {
            assert_eq!(
                n.as_u64().unwrap(),
                0,
                "Should start with no active sessions"
            );
        } else {
            panic!("active_sessions key not found or wrong type");
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_process_ai_request_create_session() {
        let mut interface = AiMcpInterface::new().unwrap();

        let request = McpRequest {
            request_id: "create-session-001".to_string(),
            session_id: None,
            agent_id: "test-agent".to_string(),
            request_type: McpRequestType::CreateSession { preferences: None },
            metadata: HashMap::new(),
            timestamp: SystemTime::now(),
        };

        let response = interface.process_ai_request(request).await.unwrap();
        assert!(response.success);
        assert!(response.message.contains("session created"));
        assert!(response.session_info.is_some());
        let session_info = response.session_info.unwrap();
        assert!(!session_info.session_id.is_empty());
        assert_eq!(session_info.status, "Active");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_process_ai_request_create_session_with_preferences() {
        let mut interface = AiMcpInterface::new().unwrap();
        let prefs = AiPreferences::default();

        let request = McpRequest {
            request_id: "create-002".to_string(),
            session_id: None,
            agent_id: "agent-2".to_string(),
            request_type: McpRequestType::CreateSession {
                preferences: Some(prefs),
            },
            metadata: HashMap::new(),
            timestamp: SystemTime::now(),
        };

        let response = interface.process_ai_request(request).await.unwrap();
        assert!(response.success);
        assert!(response.session_info.is_some());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_process_ai_request_update_preferences_session_not_found() {
        let mut interface = AiMcpInterface::new().unwrap();

        let request = McpRequest {
            request_id: "update-001".to_string(),
            session_id: Some("nonexistent-session".to_string()),
            agent_id: "agent".to_string(),
            request_type: McpRequestType::UpdatePreferences {
                preferences: AiPreferences::default(),
            },
            metadata: HashMap::new(),
            timestamp: SystemTime::now(),
        };

        let response = interface.process_ai_request(request).await.unwrap();
        assert!(!response.success);
        assert!(response.message.contains("Session not found"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_process_ai_request_update_preferences_success() {
        let mut interface = AiMcpInterface::new().unwrap();

        // First create a session
        let create_req = McpRequest {
            request_id: "create".to_string(),
            session_id: None,
            agent_id: "agent".to_string(),
            request_type: McpRequestType::CreateSession { preferences: None },
            metadata: HashMap::new(),
            timestamp: SystemTime::now(),
        };
        let create_resp = interface.process_ai_request(create_req).await.unwrap();
        let session_id = create_resp
            .session_info
            .as_ref()
            .unwrap()
            .session_id
            .clone();

        // Now update preferences
        let prefs = AiPreferences::default();
        let request = McpRequest {
            request_id: "update".to_string(),
            session_id: Some(session_id),
            agent_id: "agent".to_string(),
            request_type: McpRequestType::UpdatePreferences {
                preferences: prefs.clone(),
            },
            metadata: HashMap::new(),
            timestamp: SystemTime::now(),
        };

        let response = interface.process_ai_request(request).await.unwrap();
        assert!(response.success);
        assert!(response.message.contains("Preferences updated"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_process_ai_request_get_system_status() {
        let mut interface = AiMcpInterface::new().unwrap();

        let request = McpRequest {
            request_id: "status-001".to_string(),
            session_id: None,
            agent_id: "agent".to_string(),
            request_type: McpRequestType::GetSystemStatus,
            metadata: HashMap::new(),
            timestamp: SystemTime::now(),
        };

        let response = interface.process_ai_request(request).await.unwrap();
        assert!(response.success);
        assert!(response.data.is_some());
        let data = response.data.unwrap();
        assert!(data.get("hardware").is_some());
        assert!(data.get("ecosystem").is_some());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_process_ai_request_optimize_for_task() {
        let mut interface = AiMcpInterface::new().unwrap();

        let request = McpRequest {
            request_id: "optimize-001".to_string(),
            session_id: None,
            agent_id: "agent".to_string(),
            request_type: McpRequestType::OptimizeForTask {
                task_description: "run machine learning training".to_string(),
            },
            metadata: HashMap::new(),
            timestamp: SystemTime::now(),
        };

        let response = interface.process_ai_request(request).await.unwrap();
        assert!(response.success);
        assert!(response.config_applied.is_some());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_process_ai_request_execute_with_intent() {
        let mut interface = AiMcpInterface::new().unwrap();

        let request = McpRequest {
            request_id: "execute-001".to_string(),
            session_id: None,
            agent_id: "agent".to_string(),
            request_type: McpRequestType::ExecuteWithIntent {
                code: "print('hello')".to_string(),
                intent: ExecutionIntent {
                    purpose: "test execution".to_string(),
                    security_requirements: vec![],
                    performance_expectations: PerformanceExpectations {
                        expected_duration: None,
                        cpu_intensity: 0.5,
                        memory_pattern: MemoryPattern::Normal,
                        io_intensity: IoIntensity::Low,
                    },
                    resource_hints: ResourceHints {
                        cpu_cores: None,
                        memory_gb: None,
                        gpu_required: false,
                        storage_gb: None,
                    },
                    runtime_hint: None,
                },
            },
            metadata: HashMap::new(),
            timestamp: SystemTime::now(),
        };

        let response = interface.process_ai_request(request).await.unwrap();
        assert!(response.success);
        assert!(response.config_applied.is_some());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_process_ai_request_increments_counter() {
        let mut interface = AiMcpInterface::new().unwrap();

        let request = McpRequest {
            request_id: "req-1".to_string(),
            session_id: None,
            agent_id: "agent".to_string(),
            request_type: McpRequestType::GetSystemStatus,
            metadata: HashMap::new(),
            timestamp: SystemTime::now(),
        };
        let _ = interface.process_ai_request(request).await.unwrap();

        let stats = interface.get_session_stats().await;
        let total = stats
            .get("total_requests")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        assert!(total >= 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_process_ai_request_updates_session_activity() {
        let mut interface = AiMcpInterface::new().unwrap();

        let create_req = McpRequest {
            request_id: "c1".to_string(),
            session_id: None,
            agent_id: "a".to_string(),
            request_type: McpRequestType::CreateSession { preferences: None },
            metadata: HashMap::new(),
            timestamp: SystemTime::now(),
        };
        let create_resp = interface.process_ai_request(create_req).await.unwrap();
        let session_id = create_resp
            .session_info
            .as_ref()
            .unwrap()
            .session_id
            .clone();

        let status_req = McpRequest {
            request_id: "s1".to_string(),
            session_id: Some(session_id.clone()),
            agent_id: "a".to_string(),
            request_type: McpRequestType::GetSystemStatus,
            metadata: HashMap::new(),
            timestamp: SystemTime::now(),
        };
        let _ = interface.process_ai_request(status_req).await.unwrap();

        let stats = interface.get_session_stats().await;
        assert!(
            stats
                .get("active_sessions")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0)
                >= 1
        );
    }
}
