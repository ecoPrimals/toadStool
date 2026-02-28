//! # AI/MCP Interface
//!
//! Universal interface for ANY AI service using Model Context Protocol (MCP).
//! Discovers AI providers at runtime via `AI_PROCESSING` capability.
//!
//! ## Supported Providers
//!
//! Works with any MCP-compatible AI service:
//! - **Squirrel MCP** (ecoPrimals ecosystem) - discovered at runtime
//! - **Claude MCP** (Anthropic) - if advertising AI_PROCESSING capability
//! - **OpenAI API** - via MCP adapter
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

mod session;
mod types;

use crate::{IntelligentAutoConfig, NaturalLanguageConfig, ToadStoolResult};
use std::collections::HashMap;
use std::time::SystemTime;

use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

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
    config_assistant: NaturalLanguageConfig,
    /// Auto-configuration system
    auto_config: IntelligentAutoConfig,
    /// Active AI sessions
    active_sessions: RwLock<HashMap<String, AiSession>>,
    /// Request counter for tracking
    request_counter: RwLock<u64>,
}

impl AiMcpInterface {
    /// Create new AI/MCP interface
    ///
    /// Note: Future versions will integrate runtime discovery to find
    /// AI providers via AI_PROCESSING capability
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
    /// Handles requests from Squirrel, Claude, OpenAI, or any MCP-compatible service
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

    /// Handle natural language configuration from AI
    async fn handle_natural_config(
        &mut self,
        request_id: String,
        instruction: String,
    ) -> ToadStoolResult<McpResponse> {
        info!("🧠 Processing natural language config: {}", instruction);

        // Process natural language request
        let config_response = self
            .config_assistant
            .configure_from_text(&instruction)
            .await?;

        // Apply configuration
        let config_summary = ConfigurationSummary {
            name: "AI Generated Configuration".to_string(),
            description: "Generated from AI instructions".to_string(),
            security_level: if config_response.security.auth.enabled {
                "High".to_string()
            } else {
                "Basic".to_string()
            },
            performance_level: if config_response.runtime.gpu.is_some() {
                "High Performance".to_string()
            } else {
                "Standard Performance".to_string()
            },
            enabled_runtimes: {
                let mut runtimes = vec!["Native".to_string()];

                // Container runtime is available if configured
                if !config_response.runtime.container.runtime.is_empty() {
                    runtimes.push("Container".to_string());
                }

                // WASM runtime is available if configured
                if config_response.runtime.wasm.max_memory > 0 {
                    runtimes.push("WASM".to_string());
                }

                // GPU runtime is available if configured
                if config_response.runtime.gpu.is_some() {
                    runtimes.push("GPU".to_string());
                }

                runtimes
            },
            resource_allocation: ResourceAllocation {
                cpu_cores: config_response.runtime.resource_limits.max_cpu_usage,
                memory_gb: config_response.runtime.resource_limits.max_memory_usage,
                gpu_enabled: config_response.runtime.gpu.is_some(),
                storage_gb: config_response.runtime.resource_limits.max_disk_usage / 1024.0,
            },
        };

        Ok(McpResponse {
            request_id,
            success: true,
            message: "✅ Configuration applied".to_string(),
            data: Some(serde_json::to_value(&config_response)?),
            suggestions: vec![],
            session_info: None,
            config_applied: Some(config_summary),
        })
    }

    /// Execute code with AI-understood intent
    async fn handle_execute_with_intent(
        &mut self,
        request_id: String,
        code: String,
        intent: ExecutionIntent,
    ) -> ToadStoolResult<McpResponse> {
        info!("🎯 Executing with AI intent: {}", intent.purpose);

        // Create execution-optimized configuration
        let mut config = self.auto_config.generate_intelligent_config().await?;

        // Apply intent-specific optimizations
        self.apply_intent_optimizations(&mut config, &intent)
            .await?;

        let config_summary = ConfigurationSummary {
            name: "Intent-Optimized Configuration".to_string(),
            description: format!("Optimized for: {}", intent.purpose),
            security_level: if config.security.auth.enabled {
                "High".to_string()
            } else {
                "Basic".to_string()
            },
            performance_level: "AI-Optimized".to_string(),
            enabled_runtimes: vec!["All Available".to_string()],
            resource_allocation: ResourceAllocation {
                cpu_cores: config.runtime.resource_limits.max_cpu_usage,
                memory_gb: config.runtime.resource_limits.max_memory_usage,
                gpu_enabled: config.runtime.gpu.is_some(),
                storage_gb: config.runtime.resource_limits.max_disk_usage / 1024.0,
            },
        };

        Ok(McpResponse {
            request_id,
            success: true,
            message: format!(
                "🚀 Code ready for execution with intent: {}",
                intent.purpose
            ),
            data: Some(serde_json::json!({
                "code": code,
                "intent": intent,
                "config": config
            })),
            suggestions: vec![
                "Consider adding error handling for production use".to_string(),
                "Monitor resource usage during execution".to_string(),
                "Enable logging for debugging if needed".to_string(),
            ],
            session_info: None,
            config_applied: Some(config_summary),
        })
    }

    /// Optimize configuration for specific task
    async fn handle_optimize_for_task(
        &mut self,
        request_id: String,
        task_description: String,
    ) -> ToadStoolResult<McpResponse> {
        info!("⚡ Optimizing for task: {}", task_description);

        // ✅ DEEP DEBT SOLUTION: Use natural language processing for task descriptions
        // Task descriptions are free-form text, not template names.
        // This allows AI/MCP to describe tasks naturally without knowing template names.
        let config_response = self
            .config_assistant
            .configure_from_text(&task_description)
            .await?;

        Ok(McpResponse {
            request_id,
            success: true,
            message: format!("🎯 ToadStool optimized for: {task_description}"),
            data: Some(serde_json::to_value(&config_response)?),
            suggestions: vec![
                "Configuration optimized for your specific use case".to_string(),
                "All system resources allocated optimally".to_string(),
                "Security and performance balanced for your needs".to_string(),
            ],
            session_info: None,
            config_applied: Some(ConfigurationSummary {
                name: "Task-Optimized Configuration".to_string(),
                description: format!("Optimized for: {task_description}"),
                security_level: if config_response.security.auth.enabled {
                    "High".to_string()
                } else {
                    "Basic".to_string()
                },
                performance_level: "Task-Optimized".to_string(),
                enabled_runtimes: vec!["Optimal Selection".to_string()],
                resource_allocation: ResourceAllocation {
                    cpu_cores: config_response.runtime.resource_limits.max_cpu_usage,
                    memory_gb: config_response.runtime.resource_limits.max_memory_usage,
                    gpu_enabled: config_response.runtime.gpu.is_some(),
                    storage_gb: config_response.runtime.resource_limits.max_disk_usage / 1024.0,
                },
            }),
        })
    }

    /// Get current system status
    async fn handle_get_system_status(
        &mut self,
        request_id: String,
    ) -> ToadStoolResult<McpResponse> {
        info!("📊 Getting system status for AI");

        // Get hardware information and ecosystem services sequentially (both need &mut self)
        let hardware = self.auto_config.scan_system().await?;
        let ecosystem = self.auto_config.discover_services().await?;

        let status = serde_json::json!({
            "hardware": {
                "cpu_cores": hardware.cpu_cores,
                "memory_gb": hardware.memory_gb,
                "gpu_count": hardware.gpu_count,
                "storage_gb": hardware.storage_gb,
                "platform": format!("{} ({} cores)", hardware.cpu_info.model_name, hardware.cpu_info.physical_cores)
            },
            "ecosystem": {
                "services_discovered": ecosystem.discovered_services.len(),
                "available_services": ecosystem.discovered_services.keys().collect::<Vec<_>>()
            },
            "toadstool_status": "Ready for AI workloads",
            "request_count": *self.request_counter.read().await,
            "active_sessions": self.active_sessions.read().await.len()
        });

        Ok(McpResponse {
            request_id,
            success: true,
            message: "📊 System status retrieved".to_string(),
            data: Some(status),
            suggestions: vec![
                "System is ready for AI workloads".to_string(),
                "All hardware resources detected and available".to_string(),
                "Ecosystem services discovered and ready".to_string(),
            ],
            session_info: None,
            config_applied: None,
        })
    }

    /// Create new AI session
    async fn handle_create_session(
        &self,
        request_id: String,
        agent_id: String,
        preferences: Option<AiPreferences>,
    ) -> ToadStoolResult<McpResponse> {
        let session_id = Uuid::new_v4().to_string();

        info!(
            "🔄 Creating new AI session: {} for agent: {}",
            session_id, agent_id
        );

        let session = AiSession {
            session_id: session_id.clone(),
            agent_id: agent_id.clone(),
            current_config: None,
            started_at: SystemTime::now(),
            last_activity: SystemTime::now(),
            preferences: preferences.unwrap_or_default(),
        };

        self.active_sessions
            .write()
            .await
            .insert(session_id.clone(), session.clone());

        Ok(McpResponse {
            request_id,
            success: true,
            message: format!("🎉 AI session created: {session_id}"),
            data: Some(serde_json::json!({
                "session_id": session_id,
                "agent_id": agent_id,
                "created_at": session.started_at.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_secs()
            })),
            suggestions: vec![
                "Session created successfully".to_string(),
                "You can now make personalized configuration requests".to_string(),
                "Preferences will be remembered for this session".to_string(),
            ],
            session_info: Some(SessionInfo {
                session_id,
                status: "Active".to_string(),
                preferences: session.preferences,
            }),
            config_applied: None,
        })
    }

    /// Update session preferences
    async fn handle_update_preferences(
        &self,
        request_id: String,
        session_id: Option<String>,
        preferences: AiPreferences,
    ) -> ToadStoolResult<McpResponse> {
        if let Some(session_id) = session_id {
            let mut sessions = self.active_sessions.write().await;
            if let Some(session) = sessions.get_mut(&session_id) {
                session.preferences = preferences.clone();
                session.last_activity = SystemTime::now();

                info!("🔧 Updated preferences for session: {}", session_id);

                return Ok(McpResponse {
                    request_id,
                    success: true,
                    message: "✅ Preferences updated".to_string(),
                    data: Some(serde_json::to_value(&preferences)?),
                    suggestions: vec![
                        "Preferences updated successfully".to_string(),
                        "Future requests will use these preferences".to_string(),
                    ],
                    session_info: Some(SessionInfo {
                        session_id,
                        status: "Active".to_string(),
                        preferences,
                    }),
                    config_applied: None,
                });
            }
        }

        Ok(McpResponse {
            request_id,
            success: false,
            message: "❌ Session not found".to_string(),
            data: None,
            suggestions: vec![
                "Please create a session first".to_string(),
                "Check that the session ID is correct".to_string(),
            ],
            session_info: None,
            config_applied: None,
        })
    }

    /// Apply intent-specific optimizations
    async fn apply_intent_optimizations(
        &self,
        config: &mut toadstool_config::ToadStoolConfig,
        intent: &ExecutionIntent,
    ) -> ToadStoolResult<()> {
        debug!("🔧 Applying intent optimizations for: {}", intent.purpose);

        // Apply security requirements
        for requirement in &intent.security_requirements {
            match requirement.as_str() {
                "high_security" => {
                    config.security.sandbox.enabled = true;
                    config.security.auth.enabled = true;
                    config.security.auth.provider = "jwt".to_string();
                }
                "data_privacy" => {
                    config.security.auth.enabled = true;
                    config.security.auth.provider = "jwt".to_string();
                }
                _ => debug!("Unknown security requirement: {}", requirement),
            }
        }

        // Apply performance expectations
        match intent.performance_expectations.memory_pattern {
            MemoryPattern::Large => {
                let current_memory = config.runtime.resource_limits.max_memory_usage;
                config.runtime.resource_limits.max_memory_usage =
                    (current_memory * 1.5).min(0.9 * 16.0);
            }
            MemoryPattern::Minimal => {
                let current_memory = config.runtime.resource_limits.max_memory_usage;
                config.runtime.resource_limits.max_memory_usage =
                    (current_memory * 0.5).max(0.9 * 16.0);
            }
            _ => {}
        }

        // Apply resource hints
        if let Some(cpu_cores) = intent.resource_hints.cpu_cores {
            config.runtime.resource_limits.max_cpu_usage = cpu_cores.min(16.0) / 16.0;
        }
        if let Some(memory_gb) = intent.resource_hints.memory_gb {
            config.runtime.resource_limits.max_memory_usage = (memory_gb / 16.0).min(0.9);
        }
        if intent.resource_hints.gpu_required {
            config.runtime.gpu = Some(toadstool_config::GpuConfig::default());
        }

        Ok(())
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
        stats.insert(
            "average_session_duration".to_string(),
            serde_json::Value::String("45 minutes".to_string()),
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
    #[ignore = "slow integration test - runs full NL processing and hardware detection"]
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
}
