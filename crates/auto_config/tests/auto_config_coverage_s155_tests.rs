// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive coverage tests for under-covered `auto_config` modules
//!
//! Tests MCP interface, intelligent pipeline, capability traits, natural language,
//! ecosystem types, and installer integration.
//!
//! Run: cargo test -p toadstool-auto-config --test `auto_config_coverage_s155_tests`
//! For capability trait mocks: add --features test-mocks (4 extra tests)

#![allow(
    clippy::float_cmp,
    reason = "exact comparison intended in this context"
)]

use std::collections::HashMap;
use std::time::SystemTime;

use toadstool_auto_config::ai_mcp_interface::{
    AiMcpInterface, AiPreferences, AiSession, ConfigurationSummary, ExecutionIntent, IoIntensity,
    McpRequest, McpRequestType, McpResponse, MemoryPattern, PerformanceExpectations,
    ResourceAllocation, ResourceHints,
};
use toadstool_auto_config::ecosystem::{
    DiscoverySummary, ServiceInfo, ServicePattern, ServiceStatus, ServiceType,
};
use toadstool_auto_config::installer::{ConfigManager, InstallationConfig, SmartInstaller};
use toadstool_auto_config::intelligent::{
    ConfigGenerator, ConfigValidator, PlatformConfig, PlatformInfo, PlatformOptimization,
    PlatformOptimizer, PlatformSupport, UsageHints, UsageLearner,
};
use toadstool_auto_config::natural_language::intent::{analyze_intent, create_intent_patterns};
use toadstool_auto_config::natural_language::templates::{create_templates, get_template};
use toadstool_auto_config::natural_language::types::{RuntimePreferences, RuntimeType};
use toadstool_auto_config::{DiscoveredServices, NaturalLanguageConfig, SystemCapabilities};
#[cfg(feature = "test-mocks")]
use toadstool_auto_config::{
    EcosystemServiceDiscoverer, HardwareCapabilityDetector, MockEcosystemDiscoverer,
    MockHardwareDetector,
};

// =============================================================================
// MCP Interface Types and Handlers
// =============================================================================

#[test]
fn test_mcp_request_type_serialization() {
    let request = McpRequest {
        request_id: "req-1".to_string(),
        session_id: None,
        agent_id: "agent-1".to_string(),
        request_type: McpRequestType::GetSystemStatus,
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };
    let json = serde_json::to_value(&request);
    assert!(json.is_ok());
}

#[test]
fn test_mcp_response_creation() {
    let response = McpResponse {
        request_id: "req-1".to_string(),
        success: true,
        message: "OK".to_string(),
        data: Some(serde_json::json!({"key": "value"})),
        suggestions: vec!["suggestion 1".to_string()],
        session_info: None,
        config_applied: None,
    };
    assert!(response.success);
    assert!(response.data.is_some());
}

#[test]
fn test_execution_intent_creation() {
    let intent = ExecutionIntent {
        purpose: "test".to_string(),
        security_requirements: vec!["high_security".to_string()],
        performance_expectations: PerformanceExpectations {
            expected_duration: None,
            cpu_intensity: 0.8,
            memory_pattern: MemoryPattern::Large,
            io_intensity: IoIntensity::Medium,
        },
        resource_hints: ResourceHints {
            cpu_cores: Some(8.0),
            memory_gb: Some(16.0),
            gpu_required: true,
            storage_gb: None,
        },
        runtime_hint: Some("python".to_string()),
    };
    assert_eq!(intent.purpose, "test");
    assert!(matches!(
        intent.performance_expectations.memory_pattern,
        MemoryPattern::Large
    ));
}

#[test]
fn test_configuration_summary_creation() {
    let summary = ConfigurationSummary {
        name: "Test Config".to_string(),
        description: "Test description".to_string(),
        security_level: "High".to_string(),
        performance_level: "Optimized".to_string(),
        enabled_runtimes: vec!["Native".to_string(), "Container".to_string()],
        resource_allocation: ResourceAllocation {
            cpu_cores: 8.0,
            memory_gb: 16.0,
            gpu_enabled: true,
            storage_gb: 500.0,
        },
    };
    assert_eq!(summary.name, "Test Config");
    assert_eq!(summary.resource_allocation.cpu_cores, 8.0);
}

#[test]
fn test_ai_session_creation() {
    let session = AiSession {
        session_id: "s1".to_string(),
        agent_id: "a1".to_string(),
        current_config: None,
        started_at: SystemTime::now(),
        last_activity: SystemTime::now(),
        preferences: AiPreferences::default(),
    };
    assert_eq!(session.session_id, "s1");
}

#[test]
fn test_ai_preferences_default() {
    let prefs = AiPreferences::default();
    assert_eq!(prefs.resource_preferences.cpu_strategy, "balanced");
    assert_eq!(prefs.resource_preferences.gpu_preference, "auto");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_mcp_interface_process_get_status() {
    let mut interface = AiMcpInterface::new().unwrap();
    let request = McpRequest {
        request_id: "status-1".to_string(),
        session_id: None,
        agent_id: "agent".to_string(),
        request_type: McpRequestType::GetSystemStatus,
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };
    let response = interface.process_ai_request(request).await;
    assert!(response.is_ok());
    let resp = response.unwrap();
    assert!(resp.success);
    assert!(resp.data.is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_mcp_interface_session_stats() {
    let interface = AiMcpInterface::new().unwrap();
    let stats = interface.get_session_stats().await;
    assert!(stats.contains_key("active_sessions"));
    assert!(stats.contains_key("total_requests"));
}

// =============================================================================
// Intelligent Analysis, Detection, Validation
// =============================================================================

#[test]
fn test_usage_learner_new() {
    let learner = UsageLearner::new();
    assert!(learner.environment_hints.is_empty());
}

#[test]
fn test_usage_hints_default() {
    let hints = UsageHints::default();
    assert!(hints.predicted_workload_types.is_empty());
    assert!(!hints.is_cpu_intensive());
    assert!(!hints.is_memory_intensive());
}

#[test]
fn test_usage_hints_cpu_intensive() {
    let hints = UsageHints {
        expected_cpu_usage: 0.85,
        ..Default::default()
    };
    assert!(hints.is_cpu_intensive());
}

#[test]
fn test_platform_optimizer_new() {
    let optimizer = PlatformOptimizer::new();
    assert!(!optimizer.platform_info.os_name.is_empty());
    assert!(!optimizer.platform_info.architecture.is_empty());
}

#[test]
fn test_platform_info_detect() {
    let info = PlatformInfo::detect();
    assert!(!info.os_name.is_empty());
    assert!(!info.architecture.is_empty());
}

#[test]
fn test_platform_config_supports() {
    let mut config = PlatformConfig {
        platform_name: "linux".to_string(),
        supported_features: std::collections::HashSet::new(),
        optimizations: Vec::new(),
    };
    config
        .supported_features
        .insert(PlatformSupport::Containers);
    config
        .supported_features
        .insert(PlatformSupport::Sandboxing);

    assert!(config.supports(&PlatformSupport::Containers));
    assert!(config.supports_containers());
    assert!(config.supports_sandboxing());
}

#[test]
fn test_platform_optimization_creation() {
    let opt = PlatformOptimization {
        optimization_type: "memory_mapping".to_string(),
        description: "Use mmap".to_string(),
        performance_gain: 0.15,
    };
    assert_eq!(opt.optimization_type, "memory_mapping");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_platform_optimizer_optimize() {
    let optimizer = PlatformOptimizer::new();
    let hardware = SystemCapabilities::default();
    let result = optimizer.optimize_for_platform(&hardware);
    assert!(result.is_ok());
    let config = result.unwrap();
    assert!(!config.platform_name.is_empty());
}

#[test]
fn test_config_generator_new() {
    let generator = ConfigGenerator::new();
    let _ = generator; // construction succeeds
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_config_generator_generate_optimal() {
    let mut generator = ConfigGenerator::new();
    let hardware = SystemCapabilities::default();
    let platform = PlatformConfig {
        platform_name: "linux".to_string(),
        supported_features: std::collections::HashSet::new(),
        optimizations: Vec::new(),
    };
    let ecosystem = DiscoveredServices {
        discovered_services: HashMap::new(),
        discovery_summary: DiscoverySummary::default(),
        discovery_timestamp: SystemTime::now(),
    };
    let usage_hints = UsageHints::default();

    let result = generator
        .generate_optimal_config(&hardware, &platform, &ecosystem, &usage_hints)
        .await;
    assert!(result.is_ok());
    let config = result.unwrap();
    assert!(config.runtime.max_concurrent_executions > 0);
}

#[test]
fn test_config_validator_new() {
    let validator = ConfigValidator::new();
    let config = toadstool_config::ToadStoolConfig::default();
    assert!(validator.validate_configuration(&config).is_ok());
}

#[test]
fn test_config_validator_rejects_zero_concurrent() {
    let validator = ConfigValidator::new();
    let mut config = toadstool_config::ToadStoolConfig::default();
    config.runtime.max_concurrent_executions = 0;
    let result = validator.validate_configuration(&config);
    assert!(result.is_err());
}

#[test]
fn test_config_validator_rejects_zero_memory() {
    let validator = ConfigValidator::new();
    let mut config = toadstool_config::ToadStoolConfig::default();
    config.runtime.resource_limits.max_memory_usage = 0.0;
    let result = validator.validate_configuration(&config);
    assert!(result.is_err());
}

// =============================================================================
// Capability Traits (requires --features test-mocks)
// =============================================================================

#[cfg(feature = "test-mocks")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_mock_hardware_detector_scan() {
    let mut detector = MockHardwareDetector::new();
    let result: Result<_, _> = detector.scan_system().await;
    assert!(result.is_ok());
    let caps = result.unwrap();
    assert_eq!(caps.cpu_cores, 8.0);
    assert_eq!(caps.memory_gb, 16.0);
}

#[cfg(feature = "test-mocks")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_mock_ecosystem_discoverer() {
    let mut discoverer = MockEcosystemDiscoverer::new();
    let result: Result<_, _> = discoverer.discover_services().await;
    assert!(result.is_ok());
    let services = result.unwrap();
    assert!(services.discovered_services.is_empty());
}

#[cfg(feature = "test-mocks")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_mock_hardware_detector_custom_capabilities() {
    let caps = SystemCapabilities {
        cpu_cores: 32.0,
        memory_gb: 64.0,
        ..Default::default()
    };
    let mut detector = MockHardwareDetector::with_capabilities(caps);
    let result: Result<_, _> = detector.scan_system().await;
    assert!(result.is_ok());
    let scanned = result.unwrap();
    assert_eq!(scanned.cpu_cores, 32.0);
    assert_eq!(scanned.memory_gb, 64.0);
}

#[cfg(feature = "test-mocks")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[allow(clippy::similar_names)]
async fn test_mock_ecosystem_discoverer_with_services() {
    let mut svcs = HashMap::new();
    svcs.insert(
        "test".to_string(),
        ServiceInfo {
            name: "test".to_string(),
            endpoint: "http://localhost:8080".to_string(),
            service_type: "Test".to_string(),
            version: "1.0".to_string(),
            capabilities: vec![],
            status: ServiceStatus::Healthy,
            discovered_via: "test".to_string(),
            response_time_ms: 0,
        },
    );
    let discovered = DiscoveredServices {
        discovered_services: svcs,
        discovery_summary: DiscoverySummary::default(),
        discovery_timestamp: SystemTime::now(),
    };
    let mut discoverer = MockEcosystemDiscoverer::with_services(discovered);
    let result: Result<_, _> = discoverer.discover_services().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().discovered_services.len(), 1);
}

// =============================================================================
// Natural Language Intent and Templates
// =============================================================================

#[test]
fn test_create_intent_patterns() {
    let patterns = create_intent_patterns();
    assert!(patterns.contains_key("machine_learning"));
    assert!(patterns.contains_key("web_development"));
    assert!(patterns.contains_key("data_processing"));
    assert!(patterns.contains_key("gaming"));
}

#[test]
fn test_analyze_intent_machine_learning() {
    let patterns = create_intent_patterns();
    let result = analyze_intent(
        "I want to train neural networks with GPU and pytorch",
        &patterns,
    );
    assert!(result.is_ok());
    let analysis = result.unwrap();
    assert_eq!(analysis.primary_intent, "machine_learning");
    assert!(analysis.confidence > 0.0);
}

#[test]
fn test_analyze_intent_web_development() {
    let patterns = create_intent_patterns();
    let result = analyze_intent("Building a web app with React and REST API", &patterns);
    assert!(result.is_ok());
    let analysis = result.unwrap();
    assert_eq!(analysis.primary_intent, "web_development");
}

#[test]
fn test_analyze_intent_general_purpose_fallback() {
    let patterns = create_intent_patterns();
    let result = analyze_intent("random text xyz123", &patterns);
    assert!(result.is_ok());
    let analysis = result.unwrap();
    assert_eq!(analysis.primary_intent, "general_purpose");
    assert_eq!(analysis.confidence, 0.0);
}

#[test]
fn test_extract_explicit_preferences() {
    use toadstool_auto_config::natural_language::intent::extract_explicit_preferences;

    let prefs = extract_explicit_preferences("I need fast performance with gpu");
    assert_eq!(prefs.performance_priority, Some("high".to_string()));
    assert_eq!(prefs.use_gpu, Some(true));

    let prefs2 = extract_explicit_preferences("secure and isolated environment");
    assert_eq!(prefs2.security_priority, Some("high".to_string()));
}

#[test]
fn test_create_templates() {
    let templates = create_templates();
    assert!(templates.contains_key("machine_learning"));
    assert!(templates.contains_key("web_development"));
    assert!(templates.contains_key("general_purpose"));
}

#[test]
fn test_get_template_by_name() {
    let templates = create_templates();
    let tmpl = get_template(&templates, "machine_learning");
    assert_eq!(tmpl.name, "Machine Learning");
    assert!(tmpl.runtime_preferences.enable_gpu());
}

#[test]
fn test_get_template_fallback_to_general_purpose() {
    let templates = create_templates();
    let tmpl = get_template(&templates, "nonexistent");
    assert_eq!(tmpl.name, "General Purpose");
}

#[test]
fn test_runtime_preferences_is_enabled() {
    use std::collections::HashSet;

    let mut set = HashSet::new();
    set.insert(RuntimeType::Gpu);
    set.insert(RuntimeType::Python);
    let prefs = RuntimePreferences {
        enabled_runtimes: set,
        gpu_memory_fraction: 0.8,
        python_memory_limit_gb: 16.0,
    };
    assert!(prefs.is_enabled(&RuntimeType::Gpu));
    assert!(prefs.enable_gpu());
    assert!(prefs.enable_python());
}

#[test]
fn test_natural_language_config_analyze_intent() {
    let config = NaturalLanguageConfig::new();
    let analysis = config
        .analyze_intent("machine learning with tensorflow and gpu")
        .unwrap();
    assert_eq!(analysis.primary_intent, "machine_learning");
}

#[test]
fn test_natural_language_config_extract_preferences() {
    let config = NaturalLanguageConfig::new();
    let prefs = config
        .extract_explicit_preferences("high performance gpu")
        .unwrap();
    assert_eq!(prefs.performance_priority, Some("high".to_string()));
    assert_eq!(prefs.use_gpu, Some(true));
}

// =============================================================================
// Ecosystem Types
// =============================================================================

#[test]
fn test_service_type_display() {
    assert_eq!(
        ServiceType::NetworkCoordination.to_string(),
        "Network Coordination"
    );
    assert_eq!(ServiceType::Security.to_string(), "Security");
    assert_eq!(ServiceType::AI.to_string(), "AI");
    assert_eq!(ServiceType::Compute.to_string(), "Compute");
}

#[test]
fn test_service_pattern_creation() {
    let pattern = ServicePattern {
        name: "compute".to_string(),
        description: "Compute service".to_string(),
        default_ports: vec![8080, 8443],
        health_endpoints: vec!["/health".to_string()],
        service_type: ServiceType::Compute,
        required_capabilities: vec!["compute".to_string()],
    };
    assert_eq!(pattern.name, "compute");
    assert_eq!(pattern.default_ports.len(), 2);
}

#[test]
fn test_discovery_summary_default() {
    let summary = DiscoverySummary::default();
    assert_eq!(summary.total_services_found, 0);
    assert!(summary.discovery_methods_used.is_empty());
}

#[test]
fn test_service_info_serialization() {
    let info = ServiceInfo {
        name: "test".to_string(),
        endpoint: "http://localhost:8080".to_string(),
        service_type: "Test".to_string(),
        version: "1.0".to_string(),
        capabilities: vec!["cap1".to_string()],
        status: ServiceStatus::Healthy,
        discovered_via: "test".to_string(),
        response_time_ms: 10,
    };
    let json = serde_json::to_value(&info);
    assert!(json.is_ok());
}

#[test]
fn test_discovered_services_creation() {
    let services = DiscoveredServices {
        discovered_services: HashMap::new(),
        discovery_summary: DiscoverySummary::default(),
        discovery_timestamp: SystemTime::now(),
    };
    assert!(services.discovered_services.is_empty());
}

// =============================================================================
// Installer Integration and Runtime Detection
// =============================================================================

#[test]
fn test_smart_installer_new() {
    let installer = SmartInstaller::new();
    let _ = installer; // Construction succeeds with platform detection
}

#[test]
fn test_smart_installer_default() {
    let installer = SmartInstaller::default();
    let _ = installer;
}

#[test]
fn test_config_manager_new() {
    let manager = ConfigManager::new();
    let _ = manager;
}

#[test]
fn test_installation_config_default() {
    let config = InstallationConfig::default();
    assert!(config.add_to_path);
    assert!(config.enable_shell_completion);
}

#[tokio::test]
async fn test_config_manager_apply_configuration() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().to_path_buf();
    let manager = ConfigManager::with_path(config_path.clone());

    let config = toadstool_config::ToadStoolConfig::default();
    let result: Result<(), _> = manager.apply_configuration(&config).await;
    assert!(result.is_ok());
    assert!(config_path.join("toadstool.json").exists());
}
