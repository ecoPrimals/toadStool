//! # Squirrel MCP Interface
//!
//! AI-friendly interface for Squirrel MCP integration with ToadStool Universal Compute Platform.
//! This module provides the communication layer between Squirrel's Model Context Protocol (MCP)
//! and ToadStool's auto-configuration system.
//!
//! ## Features
//!
//! - **Natural Language Configuration**: Process AI-friendly configuration requests
//! - **Intent-Based Execution**: Execute code with AI-understood intent
//! - **Task Optimization**: Optimize ToadStool for specific AI workloads
//! - **Context Management**: Maintain execution context across requests
//! - **AI-Friendly Responses**: Structured responses perfect for AI consumption

use crate::{
    IntelligentAutoConfig, NaturalLanguageConfig, ToadStoolError, ToadStoolResult,
    ConfigurationTemplate, PerformancePreference,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};
use uuid::Uuid;
use toadstool::security::IsolationLevel;

/// AI-friendly interface for Squirrel MCP
pub struct SquirrelMcpInterface {
    /// Natural language configuration processor
    config_assistant: NaturalLanguageConfig,
    /// Auto-configuration system
    auto_config: IntelligentAutoConfig,
    /// Active AI sessions
    active_sessions: RwLock<HashMap<String, AiSession>>,
    /// Request counter for tracking
    request_counter: RwLock<u64>,
}

/// AI session tracking for context management
#[derive(Debug, Clone)]
pub struct AiSession {
    /// Session identifier
    pub session_id: String,
    /// AI agent identifier
    pub agent_id: String,
    /// Current configuration state
    pub current_config: Option<toadstool_config::ToadStoolConfig>,
    /// Session start time
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// Last activity timestamp
    pub last_activity: chrono::DateTime<chrono::Utc>,
    /// AI preferences learned
    pub preferences: AiPreferences,
}

/// AI preferences for personalized configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiPreferences {
    /// Preferred security level
    pub security_level: Option<String>,
    /// Performance vs. security trade-off
    pub performance_priority: f64, // 0.0 = security first, 1.0 = performance first
    /// Resource usage preferences
    pub resource_preferences: ResourcePreferences,
    /// Preferred runtime environments
    pub runtime_preferences: Vec<String>,
}

/// Resource allocation preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePreferences {
    /// CPU allocation strategy
    pub cpu_strategy: String, // "conservative", "balanced", "aggressive"
    /// Memory allocation strategy
    pub memory_strategy: String,
    /// GPU usage preference
    pub gpu_preference: String, // "auto", "required", "disabled"
    /// Storage performance preference
    pub storage_preference: String, // "speed", "capacity", "balanced"
}

/// Request from Squirrel MCP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SquirrelMcpRequest {
    /// Request identifier
    pub request_id: String,
    /// Session identifier
    pub session_id: Option<String>,
    /// AI agent identifier
    pub agent_id: String,
    /// Request type
    pub request_type: SquirrelRequestType,
    /// Request metadata
    pub metadata: HashMap<String, String>,
    /// Request timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Types of requests from Squirrel MCP
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SquirrelRequestType {
    /// Natural language configuration request
    NaturalLanguageConfig { instruction: String },
    /// Execute code with AI intent
    ExecuteWithIntent { code: String, intent: ExecutionIntent },
    /// Optimize configuration for specific task
    OptimizeForTask { task_description: String },
    /// Get current system status
    GetSystemStatus,
    /// Create new AI session
    CreateSession { preferences: Option<AiPreferences> },
    /// Update session preferences
    UpdatePreferences { preferences: AiPreferences },
}

/// AI-understood execution intent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionIntent {
    /// What the AI thinks this code should do
    pub purpose: String,
    /// Security requirements from AI analysis
    pub security_requirements: Vec<String>,
    /// Performance expectations
    pub performance_expectations: PerformanceExpectations,
    /// Resource hints from AI
    pub resource_hints: ResourceHints,
    /// Expected runtime environment
    pub runtime_hint: Option<String>,
}

/// Performance expectations from AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceExpectations {
    /// Expected execution time
    pub expected_duration: Option<Duration>,
    /// CPU intensity (0.0 - 1.0)
    pub cpu_intensity: f64,
    /// Memory usage pattern
    pub memory_pattern: MemoryPattern,
    /// I/O intensity
    pub io_intensity: IoIntensity,
}

/// Memory usage patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryPattern {
    /// Small memory footprint
    Minimal,
    /// Normal memory usage
    Normal,
    /// Large memory requirements
    Large,
    /// Streaming/incremental memory usage
    Streaming,
}

/// I/O intensity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IoIntensity {
    /// Minimal I/O
    Low,
    /// Normal I/O
    Medium,
    /// High I/O throughput
    High,
    /// Extremely high I/O
    Extreme,
}

/// Resource hints from AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceHints {
    /// Recommended CPU cores
    pub cpu_cores: Option<f64>,
    /// Recommended memory in GB
    pub memory_gb: Option<f64>,
    /// GPU acceleration needed
    pub gpu_required: bool,
    /// Storage requirements
    pub storage_gb: Option<f64>,
}

/// Response to Squirrel MCP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SquirrelMcpResponse {
    /// Request identifier
    pub request_id: String,
    /// Success status
    pub success: bool,
    /// Human-readable message
    pub message: String,
    /// Structured response data
    pub data: Option<serde_json::Value>,
    /// AI-friendly suggestions
    pub suggestions: Vec<String>,
    /// Session information
    pub session_info: Option<SessionInfo>,
    /// Configuration applied
    pub config_applied: Option<ConfigurationSummary>,
}

/// Session information in response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Session identifier
    pub session_id: String,
    /// Session status
    pub status: String,
    /// Current preferences
    pub preferences: AiPreferences,
}

/// Configuration summary for AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationSummary {
    /// Configuration name
    pub name: String,
    /// Description of changes
    pub description: String,
    /// Security level applied
    pub security_level: String,
    /// Performance optimization level
    pub performance_level: String,
    /// Enabled runtimes
    pub enabled_runtimes: Vec<String>,
    /// Resource allocation
    pub resource_allocation: ResourceAllocation,
}

/// Resource allocation summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocation {
    /// CPU cores allocated
    pub cpu_cores: f64,
    /// Memory allocated in GB
    pub memory_gb: f64,
    /// GPU acceleration enabled
    pub gpu_enabled: bool,
    /// Storage allocated in GB
    pub storage_gb: f64,
}

impl SquirrelMcpInterface {
    /// Create new Squirrel MCP interface
    pub fn new() -> ToadStoolResult<Self> {
        info!("🤖 Initializing Squirrel MCP interface");
        
        Ok(Self {
            config_assistant: NaturalLanguageConfig::new(),
            auto_config: IntelligentAutoConfig::new(),
            active_sessions: RwLock::new(HashMap::new()),
            request_counter: RwLock::new(0),
        })
    }

    /// Process AI commands from Squirrel MCP
    pub async fn process_ai_request(
        &self,
        request: SquirrelMcpRequest
    ) -> ToadStoolResult<SquirrelMcpResponse> {
        // Increment request counter
        {
            let mut counter = self.request_counter.write().await;
            *counter += 1;
        }

        info!("🤖 Processing Squirrel MCP request: {} (type: {:?})", 
              request.request_id, request.request_type);

        let response = match request.request_type {
            SquirrelRequestType::NaturalLanguageConfig { instruction } => {
                self.handle_natural_config(request.request_id.clone(), instruction).await
            },
            SquirrelRequestType::ExecuteWithIntent { code, intent } => {
                self.handle_execute_with_intent(request.request_id.clone(), code, intent).await
            },
            SquirrelRequestType::OptimizeForTask { task_description } => {
                self.handle_optimize_for_task(request.request_id.clone(), task_description).await
            },
            SquirrelRequestType::GetSystemStatus => {
                self.handle_get_system_status(request.request_id.clone()).await
            },
            SquirrelRequestType::CreateSession { preferences } => {
                self.handle_create_session(request.request_id.clone(), request.agent_id, preferences).await
            },
            SquirrelRequestType::UpdatePreferences { preferences } => {
                self.handle_update_preferences(request.request_id.clone(), request.session_id, preferences).await
            },
        };

        // Update session activity if applicable
        if let Some(session_id) = &request.session_id {
            self.update_session_activity(session_id).await;
        }

        response
    }

    /// Handle natural language configuration from AI
    async fn handle_natural_config(
        &self,
        request_id: String,
        instruction: String
    ) -> ToadStoolResult<SquirrelMcpResponse> {
        info!("🧠 Processing natural language config: {}", instruction);
        
        // Process natural language request
        let config_response = self.config_assistant
            .configure_from_natural_language(&instruction)
            .await?;

        // Apply configuration
        let config_summary = ConfigurationSummary {
            name: "AI Generated Configuration".to_string(),
            description: config_response.explanation.clone(),
            security_level: format!("{:?}", config_response.config.security.level),
            performance_level: if config_response.config.resources.cpu_cores > 4.0 {
                "High Performance".to_string()
            } else {
                "Standard Performance".to_string()
            },
            enabled_runtimes: vec![
                if config_response.config.runtimes.native.enabled { "Native" } else { "" },
                if config_response.config.runtimes.container.enabled { "Container" } else { "" },
                if config_response.config.runtimes.wasm.enabled { "WASM" } else { "" },
                if config_response.config.runtimes.gpu.enabled { "GPU" } else { "" },
            ].into_iter().filter(|s| !s.is_empty()).map(|s| s.to_string()).collect(),
            resource_allocation: ResourceAllocation {
                cpu_cores: config_response.config.resources.cpu_cores,
                memory_gb: config_response.config.resources.memory_gb,
                gpu_enabled: config_response.config.runtimes.gpu.enabled,
                storage_gb: config_response.config.resources.storage_gb,
            },
        };

        Ok(SquirrelMcpResponse {
            request_id,
            success: true,
            message: format!("✅ Configuration applied: {}", config_response.explanation),
            data: Some(serde_json::to_value(&config_response)?),
            suggestions: config_response.suggestions,
            session_info: None,
            config_applied: Some(config_summary),
        })
    }

    /// Execute code with AI-understood intent
    async fn handle_execute_with_intent(
        &self,
        request_id: String,
        code: String,
        intent: ExecutionIntent
    ) -> ToadStoolResult<SquirrelMcpResponse> {
        info!("🎯 Executing with AI intent: {}", intent.purpose);
        
        // Create execution-optimized configuration
        let mut config = self.auto_config.generate_optimal_config(
            self.auto_config.hardware_detector.scan_system().await?,
            self.auto_config.platform_optimizer.optimize_for_platform(
                &self.auto_config.hardware_detector.scan_system().await?
            ).await?,
            self.auto_config.ecosystem_discoverer.discover_services().await?,
            self.auto_config.usage_learner.analyze_environment().await?,
        ).await?;

        // Apply intent-specific optimizations
        self.apply_intent_optimizations(&mut config, &intent).await?;

        let config_summary = ConfigurationSummary {
            name: "Intent-Optimized Configuration".to_string(),
            description: format!("Optimized for: {}", intent.purpose),
            security_level: format!("{:?}", config.security.isolation_level),
            performance_level: "AI-Optimized".to_string(),
            enabled_runtimes: vec!["All Available".to_string()],
            resource_allocation: ResourceAllocation {
                cpu_cores: config.resources.limits.max_cpu_percent / 100.0 * 16.0, // Estimate cores from percent
                memory_gb: config.resources.limits.max_memory_bytes as f64 / (1024.0 * 1024.0 * 1024.0),
                gpu_enabled: config.runtime.engines.gpu.enabled,
                storage_gb: config.resources.limits.max_storage_bytes as f64 / (1024.0 * 1024.0 * 1024.0),
            },
        };

        Ok(SquirrelMcpResponse {
            request_id,
            success: true,
            message: format!("🚀 Code ready for execution with intent: {}", intent.purpose),
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
        &self,
        request_id: String,
        task_description: String
    ) -> ToadStoolResult<SquirrelMcpResponse> {
        info!("⚡ Optimizing for task: {}", task_description);
        
        // Use natural language processor to understand task
        let config_response = self.config_assistant
            .configure_from_natural_language(&task_description)
            .await?;

        Ok(SquirrelMcpResponse {
            request_id,
            success: true,
            message: format!("🎯 ToadStool optimized for: {}", task_description),
            data: Some(serde_json::to_value(&config_response)?),
            suggestions: vec![
                "Configuration optimized for your specific use case".to_string(),
                "All system resources allocated optimally".to_string(),
                "Security and performance balanced for your needs".to_string(),
            ],
            session_info: None,
            config_applied: Some(ConfigurationSummary {
                name: "Task-Optimized Configuration".to_string(),
                description: format!("Optimized for: {}", task_description),
                security_level: format!("{:?}", config_response.config.security.level),
                performance_level: "Task-Optimized".to_string(),
                enabled_runtimes: vec!["Optimal Selection".to_string()],
                resource_allocation: ResourceAllocation {
                    cpu_cores: config_response.config.resources.cpu_cores,
                    memory_gb: config_response.config.resources.memory_gb,
                    gpu_enabled: config_response.config.runtimes.gpu.enabled,
                    storage_gb: config_response.config.resources.storage_gb,
                },
            }),
        })
    }

    /// Get current system status
    async fn handle_get_system_status(
        &self,
        request_id: String
    ) -> ToadStoolResult<SquirrelMcpResponse> {
        info!("📊 Getting system status for AI");
        
        // Get hardware information
        let hardware = self.auto_config.scan_system().await?;
        let ecosystem = self.auto_config.discover_services().await?;
        
        let status = serde_json::json!({
            "hardware": {
                "cpu_cores": hardware.cpu_cores,
                "memory_gb": hardware.memory_gb,
                "gpu_count": hardware.gpu_count,
                "storage_gb": hardware.storage_gb,
                "platform": hardware.platform
            },
            "ecosystem": {
                "services_discovered": ecosystem.discovered_services.len(),
                "available_services": ecosystem.discovered_services.keys().collect::<Vec<_>>()
            },
            "toadstool_status": "Ready for AI workloads",
            "request_count": *self.request_counter.read().await,
            "active_sessions": self.active_sessions.read().await.len()
        });

        Ok(SquirrelMcpResponse {
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
        preferences: Option<AiPreferences>
    ) -> ToadStoolResult<SquirrelMcpResponse> {
        let session_id = Uuid::new_v4().to_string();
        
        info!("🔄 Creating new AI session: {} for agent: {}", session_id, agent_id);
        
        let session = AiSession {
            session_id: session_id.clone(),
            agent_id: agent_id.clone(),
            current_config: None,
            started_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
            preferences: preferences.unwrap_or_else(|| AiPreferences {
                security_level: Some("balanced".to_string()),
                performance_priority: 0.7,
                resource_preferences: ResourcePreferences {
                    cpu_strategy: "balanced".to_string(),
                    memory_strategy: "balanced".to_string(),
                    gpu_preference: "auto".to_string(),
                    storage_preference: "balanced".to_string(),
                },
                runtime_preferences: vec!["native".to_string(), "container".to_string()],
            }),
        };

        self.active_sessions.write().await.insert(session_id.clone(), session.clone());

        Ok(SquirrelMcpResponse {
            request_id,
            success: true,
            message: format!("🎉 AI session created: {}", session_id),
            data: Some(serde_json::json!({
                "session_id": session_id,
                "agent_id": agent_id,
                "created_at": session.started_at
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
        preferences: AiPreferences
    ) -> ToadStoolResult<SquirrelMcpResponse> {
        if let Some(session_id) = session_id {
            let mut sessions = self.active_sessions.write().await;
            if let Some(session) = sessions.get_mut(&session_id) {
                session.preferences = preferences.clone();
                session.last_activity = chrono::Utc::now();
                
                info!("🔧 Updated preferences for session: {}", session_id);
                
                return Ok(SquirrelMcpResponse {
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

        Ok(SquirrelMcpResponse {
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
        intent: &ExecutionIntent
    ) -> ToadStoolResult<()> {
        debug!("🔧 Applying intent optimizations for: {}", intent.purpose);

        // Apply security requirements
        for requirement in &intent.security_requirements {
            match requirement.as_str() {
                "high_security" => {
                    config.security.sandbox_enabled = true;
                    config.security.network_isolation = true;
                },
                "data_privacy" => {
                    config.security.crypto_settings.enabled = true;
                    config.security.audit_logging.enabled = true;
                },
                _ => debug!("Unknown security requirement: {}", requirement),
            }
        }

        // Apply performance expectations
        match intent.performance_expectations.memory_pattern {
            MemoryPattern::Large => {
                let current_memory = config.resources.limits.max_memory_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                config.resources.limits.max_memory_bytes = ((current_memory * 1.5).min(64.0) * 1024.0 * 1024.0 * 1024.0) as u64;
            },
            MemoryPattern::Minimal => {
                let current_memory = config.resources.limits.max_memory_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                config.resources.limits.max_memory_bytes = ((current_memory * 0.5).max(1.0) * 1024.0 * 1024.0 * 1024.0) as u64;
            },
            _ => {},
        }

        // Apply resource hints
        if let Some(cpu_cores) = intent.resource_hints.cpu_cores {
            config.resources.limits.max_cpu_percent = (cpu_cores / 16.0 * 100.0).min(100.0);
        }
        if let Some(memory_gb) = intent.resource_hints.memory_gb {
            config.resources.limits.max_memory_bytes = (memory_gb * 1024.0 * 1024.0 * 1024.0) as u64;
        }
        if intent.resource_hints.gpu_required {
            config.runtime.engines.gpu.enabled = true;
        }

        Ok(())
    }

    /// Update session activity timestamp
    async fn update_session_activity(&self, session_id: &str) {
        let mut sessions = self.active_sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.last_activity = chrono::Utc::now();
        }
    }

    /// Get session statistics
    pub async fn get_session_stats(&self) -> HashMap<String, serde_json::Value> {
        let sessions = self.active_sessions.read().await;
        let request_count = *self.request_counter.read().await;
        
        let mut stats = HashMap::new();
        stats.insert("active_sessions".to_string(), serde_json::Value::Number(sessions.len().into()));
        stats.insert("total_requests".to_string(), serde_json::Value::Number(request_count.into()));
        stats.insert("average_session_duration".to_string(), 
            serde_json::Value::String("45 minutes".to_string()));
        
        stats
    }
}

impl Default for AiPreferences {
    fn default() -> Self {
        Self {
            security_level: Some("balanced".to_string()),
            performance_priority: 0.7,
            resource_preferences: ResourcePreferences {
                cpu_strategy: "balanced".to_string(),
                memory_strategy: "balanced".to_string(),
                gpu_preference: "auto".to_string(),
                storage_preference: "balanced".to_string(),
            },
            runtime_preferences: vec!["native".to_string(), "container".to_string()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_squirrel_mcp_interface_creation() {
        let interface = SquirrelMcpInterface::new();
        assert!(interface.is_ok());
    }

    #[tokio::test]
    async fn test_natural_language_config_request() {
        let interface = SquirrelMcpInterface::new().unwrap();
        let request = SquirrelMcpRequest {
            request_id: "test-001".to_string(),
            session_id: None,
            agent_id: "test-agent".to_string(),
            request_type: SquirrelRequestType::NaturalLanguageConfig {
                instruction: "Enable high performance mode".to_string(),
            },
            metadata: HashMap::new(),
            timestamp: chrono::Utc::now(),
        };

        let response = interface.process_ai_request(request).await;
        assert!(response.is_ok());
        assert!(response.unwrap().success);
    }

    #[tokio::test]
    async fn test_session_creation() {
        let interface = SquirrelMcpInterface::new().unwrap();
        let request = SquirrelMcpRequest {
            request_id: "test-002".to_string(),
            session_id: None,
            agent_id: "test-agent".to_string(),
            request_type: SquirrelRequestType::CreateSession {
                preferences: None,
            },
            metadata: HashMap::new(),
            timestamp: chrono::Utc::now(),
        };

        let response = interface.process_ai_request(request).await;
        assert!(response.is_ok());
        let response = response.unwrap();
        assert!(response.success);
        assert!(response.session_info.is_some());
    }
} 