---
title: Compute Infrastructure Migration from Squirrel to Toadstool
description: Detailed migration plan for moving compute infrastructure from Squirrel to Toadstool-Compute
version: 1.0.0
date: 2025-01-26
author: Migration Team
priority: CRITICAL
status: MIGRATION_PLANNING
---

# 🚚 Compute Infrastructure Migration Plan

## Executive Summary

This plan details the migration of compute infrastructure from **Squirrel** to **Toadstool-Compute**, enabling Squirrel to focus on pure MCP platform responsibilities while Toadstool becomes the dedicated compute platform.

---

## 🎯 **Migration Objectives**

### **Primary Goals**
```yaml
migration_goals:
  separation_of_concerns: "Clear separation between MCP platform and compute infrastructure"
  zero_downtime: "Migrate without disrupting existing functionality"
  performance_improvement: "Better performance through specialized compute platform"
  maintainability: "Easier maintenance with focused responsibilities"
```

### **Success Criteria**
```yaml
success_metrics:
  functionality: "All existing compute functionality preserved"
  performance: "Performance maintained or improved"
  reliability: "No increase in error rates"
  developer_experience: "Seamless transition for developers"
```

---

## 📋 **Migration Inventory**

### **🚚 Components Moving FROM Squirrel TO Toadstool**

#### **Execution Infrastructure**
```yaml
execution_systems:
  wasm_runtime:
    source: "code/crates/services/app/src/wasm/"
    target: "toadstool/code/crates/runtime/wasm/"
    components:
      - wasmtime_integration
      - wasi_support
      - module_caching
      - memory_management
  
  container_runtime:
    source: "code/crates/services/app/src/container/"
    target: "toadstool/code/crates/runtime/container/"
    components:
      - docker_integration
      - image_management
      - network_isolation
      - volume_management
  
  native_runtime:
    source: "code/crates/services/app/src/native/"
    target: "toadstool/code/crates/runtime/native/"
    components:
      - process_execution
      - library_loading
      - security_isolation
```

#### **Sandboxing System**
```yaml
sandboxing_infrastructure:
  cross_platform_sandbox:
    source: "code/crates/sdk/src/sandbox.rs"
    target: "toadstool/code/crates/security/sandboxing/"
    components:
      - windows_job_objects
      - macos_app_sandbox
      - linux_namespaces
      - unified_api
  
  security_policies:
    source: "code/crates/services/app/src/plugin/sandbox/"
    target: "toadstool/code/crates/security/policies/"
    components:
      - permission_system
      - resource_limits
      - capability_management
      - audit_logging
```

#### **Resource Management**
```yaml
resource_systems:
  resource_monitoring:
    source: "code/crates/services/app/src/monitoring/"
    target: "toadstool/code/crates/management/resources/"
    components:
      - cpu_monitoring
      - memory_tracking
      - disk_usage
      - network_monitoring
  
  performance_profiling:
    source: "code/crates/tools/src/performance/"
    target: "toadstool/code/crates/management/performance/"
    components:
      - execution_profiling
      - resource_profiling
      - bottleneck_detection
      - optimization_hints
```

### **🏠 Components STAYING in Squirrel**

#### **MCP Platform Core**
```yaml
mcp_components:
  protocol_implementation:
    location: "code/crates/core/mcp/"
    reason: "Core MCP platform responsibility"
    
  plugin_registry:
    location: "code/crates/services/app/src/plugin/registry/"
    reason: "Plugin metadata and discovery"
    
  ai_integration:
    location: "code/crates/core/ai/"
    reason: "AI agent coordination and workflows"
    
  context_management:
    location: "code/crates/core/context/"
    reason: "MCP context storage and management"
```

#### **Plugin Platform (Modified)**
```yaml
plugin_platform_changes:
  plugin_execution:
    current: "Direct execution in Squirrel"
    future: "Delegate to Toadstool via Songbird"
    
  plugin_metadata:
    current: "Stored in Squirrel"
    future: "Remains in Squirrel (enhanced with MCP context)"
    
  plugin_discovery:
    current: "Local discovery in Squirrel"
    future: "Enhanced MCP-aware discovery in Squirrel"
```

---

## 🛠️ **Migration Phases**

### **Phase 1: Toadstool Foundation (Weeks 1-2)**

#### **Week 1: Project Setup**
```bash
# 1. Create Toadstool project structure
mkdir -p toadstool/code/{crates/{runtime,security,management},examples,tests}
cd toadstool

# 2. Initialize Cargo workspace
cat > Cargo.toml << 'TOML'
[workspace]
members = [
    "code/crates/runtime/wasm",
    "code/crates/runtime/container", 
    "code/crates/runtime/native",
    "code/crates/security/sandboxing",
    "code/crates/security/policies",
    "code/crates/management/resources",
    "code/crates/management/performance",
    "code/crates/integration/songbird",
    "code/crates/core/toadstool",
]
TOML

# 3. Create basic crate structures
for crate in runtime/wasm runtime/container runtime/native security/sandboxing security/policies management/resources management/performance integration/songbird core/toadstool; do
    cargo new --lib code/crates/$crate
done
```

#### **Week 2: Core Infrastructure**
```bash
# 1. Implement basic Songbird integration
# Create toadstool/code/crates/integration/songbird/src/lib.rs

# 2. Set up basic execution environments
# Implement container and WASM runtime foundations

# 3. Create security framework
# Implement cross-platform sandboxing foundation

# 4. Set up resource management
# Basic resource monitoring and allocation
```

### **Phase 2: Component Migration (Weeks 3-4)**

#### **Week 3: Execution Runtime Migration**
```bash
# 1. Migrate WASM runtime
cp -r squirrel/code/crates/services/app/src/wasm/* \
      toadstool/code/crates/runtime/wasm/src/

# 2. Adapt WASM runtime for Toadstool architecture
# Update imports, error handling, and integration points

# 3. Migrate container runtime
cp -r squirrel/code/crates/services/app/src/container/* \
      toadstool/code/crates/runtime/container/src/

# 4. Update container runtime for ecosystem integration

# 5. Test basic execution functionality
cargo test --package toadstool-runtime-wasm
cargo test --package toadstool-runtime-container
```

#### **Week 4: Security & Resource Migration**
```bash
# 1. Migrate sandboxing system
cp -r squirrel/code/crates/sdk/src/sandbox.rs \
      toadstool/code/crates/security/sandboxing/src/

# 2. Migrate security policies
cp -r squirrel/code/crates/services/app/src/plugin/sandbox/* \
      toadstool/code/crates/security/policies/src/

# 3. Migrate resource management
cp -r squirrel/code/crates/services/app/src/monitoring/* \
      toadstool/code/crates/management/resources/src/

# 4. Update all components for Toadstool architecture
# Fix imports, dependencies, and integration points

# 5. Test security and resource functionality
cargo test --package toadstool-security-sandboxing
cargo test --package toadstool-management-resources
```

### **Phase 3: Integration & Testing (Weeks 5-6)**

#### **Week 5: Songbird Integration**
```rust
// Implement Toadstool-Songbird integration
// toadstool/code/crates/integration/songbird/src/lib.rs

use songbird_client::SongbirdClient;

pub struct ToadstoolSongbirdIntegration {
    client: SongbirdClient,
    execution_engine: ExecutionEngine,
}

impl ToadstoolSongbirdIntegration {
    pub async fn new() -> Result<Self> {
        let client = SongbirdClient::connect("http://localhost:8080").await?;
        
        // Register Toadstool capabilities
        client.register_service(ServiceRegistration {
            service_id: "toadstool-compute".to_string(),
            service_type: "compute-platform".to_string(),
            capabilities: ToadstoolCapabilities::detect_current().await?,
            // ... other registration details
        }).await?;
        
        Ok(Self {
            client,
            execution_engine: ExecutionEngine::new().await?,
        })
    }
    
    pub async fn handle_execution_request(
        &self,
        request: ExecutionRequest
    ) -> Result<ExecutionResponse> {
        self.execution_engine.execute(request).await
    }
}
```

#### **Week 6: End-to-End Testing**
```bash
# 1. Test complete execution flow
# Squirrel → Songbird → Toadstool → Results

# 2. Performance benchmarking
# Compare pre-migration vs post-migration performance

# 3. Security validation
# Ensure all security features work correctly

# 4. Integration testing
# Test with real Squirrel plugin executions
```

### **Phase 4: Squirrel Cleanup (Weeks 7-8)**

#### **Week 7: Remove Compute Code from Squirrel**
```bash
# 1. Remove execution infrastructure
rm -rf squirrel/code/crates/services/app/src/wasm/
rm -rf squirrel/code/crates/services/app/src/container/
rm -rf squirrel/code/crates/services/app/src/native/

# 2. Remove sandboxing system
rm -f squirrel/code/crates/sdk/src/sandbox.rs
rm -rf squirrel/code/crates/services/app/src/plugin/sandbox/

# 3. Remove resource management
rm -rf squirrel/code/crates/services/app/src/monitoring/
rm -rf squirrel/code/crates/tools/src/performance/

# 4. Update Cargo.toml dependencies
# Remove compute-related dependencies
# Add toadstool-client dependency
```

#### **Week 8: Update Squirrel Plugin System**
```rust
// Update Squirrel plugin execution to use Toadstool
// squirrel/code/crates/services/app/src/plugin/manager.rs

use toadstool_client::ToadstoolClient;
use songbird_client::SongbirdClient;

pub struct PluginManager {
    registry: PluginRegistry,
    songbird_client: SongbirdClient,
    mcp_context: McpContextManager,
}

impl PluginManager {
    pub async fn execute_plugin(
        &self,
        plugin_id: &str,
        context: McpContext
    ) -> Result<PluginExecutionResult> {
        // Get plugin metadata (stays in Squirrel)
        let plugin_metadata = self.registry.get_plugin(plugin_id)?;
        
        // Create execution request
        let execution_request = ExecutionRequest {
            plugin_metadata,
            mcp_context: Some(context),
            environment: ExecutionEnvironment::Wasm,
            // ... other request details
        };
        
        // Route through Songbird to Toadstool
        let result = self.songbird_client
            .route_compute_request("toadstool-compute", execution_request).await?;
        
        Ok(result)
    }
}
```

---

## 🔄 **Data Migration**

### **Configuration Migration**
```yaml
config_migration:
  execution_settings:
    source: "squirrel/config/execution.yaml"
    target: "toadstool/config/execution.yaml"
    transformations:
      - update_paths
      - update_service_endpoints
      - add_songbird_integration
  
  security_policies:
    source: "squirrel/config/security.yaml"
    target: "toadstool/config/security.yaml"
    transformations:
      - enhance_sandbox_policies
      - add_ecosystem_permissions
      - update_audit_settings
```

### **Runtime Data Migration**
```yaml
runtime_migration:
  wasm_modules:
    action: "Copy cached WASM modules to Toadstool"
    validation: "Verify module integrity and compatibility"
    
  container_images:
    action: "Migrate container image cache"
    optimization: "Optimize for Toadstool's architecture"
    
  performance_profiles:
    action: "Migrate execution performance profiles"
    enhancement: "Add Toadstool-specific optimizations"
```

---

## 🧪 **Testing Strategy**

### **Migration Testing**
```yaml
test_phases:
  unit_testing:
    scope: "Test each migrated component individually"
    coverage: "> 90% code coverage"
    focus: "Core functionality preservation"
    
  integration_testing:
    scope: "Test Squirrel → Songbird → Toadstool flow"
    scenarios: "All existing plugin execution scenarios"
    performance: "Verify performance requirements met"
    
  regression_testing:
    scope: "Ensure no existing functionality broken"
    automation: "Automated regression test suite"
    validation: "All existing tests pass"
    
  performance_testing:
    scope: "Compare pre/post migration performance"
    metrics: "Latency, throughput, resource usage"
    targets: "Meet or exceed current performance"
```

### **Test Implementation**
```rust
// Example integration test
#[tokio::test]
async fn test_plugin_execution_migration() {
    // Setup test environment
    let songbird = start_test_songbird().await;
    let toadstool = start_test_toadstool().await;
    let squirrel = start_test_squirrel().await;
    
    // Register services
    toadstool.register_with_songbird().await.unwrap();
    squirrel.register_with_songbird().await.unwrap();
    
    // Test plugin execution
    let plugin_id = "test-wasm-plugin";
    let context = McpContext::test_context();
    
    let result = squirrel.execute_plugin(plugin_id, context).await.unwrap();
    
    // Verify result
    assert_eq!(result.status, ExecutionStatus::Success);
    assert!(result.execution_time_ms < 100);
    
    // Verify execution happened in Toadstool
    let toadstool_metrics = toadstool.get_metrics().await.unwrap();
    assert_eq!(toadstool_metrics.executions_count, 1);
}
```

---

## 🚨 **Risk Management**

### **Migration Risks**
```yaml
technical_risks:
  data_loss:
    probability: "Low"
    impact: "High"
    mitigation: "Comprehensive backup strategy"
    
  performance_degradation:
    probability: "Medium"
    impact: "Medium"
    mitigation: "Extensive performance testing"
    
  integration_failures:
    probability: "Medium"
    impact: "High"
    mitigation: "Gradual rollout with rollback plan"
    
  security_vulnerabilities:
    probability: "Low"
    impact: "High"
    mitigation: "Security audit and penetration testing"
```

### **Rollback Strategy**
```yaml
rollback_plan:
  trigger_conditions:
    - "Performance degradation > 20%"
    - "Error rate increase > 5%"
    - "Critical security vulnerability"
    - "Integration failure"
    
  rollback_steps:
    1: "Stop Toadstool services"
    2: "Restore Squirrel compute code from backup"
    3: "Update Squirrel configuration"
    4: "Restart Squirrel services"
    5: "Verify functionality"
    
  rollback_time: "< 30 minutes"
  data_preservation: "All data preserved during rollback"
```

---

## 📊 **Success Metrics**

### **Technical Metrics**
```yaml
performance_targets:
  execution_latency: "< 10ms overhead vs current"
  throughput: "> 95% of current throughput"
  resource_efficiency: "> 90% resource utilization"
  error_rate: "< 0.1% execution errors"

reliability_targets:
  uptime: "> 99.9% service availability"
  recovery_time: "< 5 minutes from failures"
  data_integrity: "100% data preservation"
  rollback_success: "< 30 minutes rollback time"
```

### **Operational Metrics**
```yaml
deployment_success:
  zero_downtime: "No service interruption during migration"
  feature_parity: "100% feature parity maintained"
  developer_impact: "No breaking changes for developers"
  documentation: "Complete migration documentation"
```

---

## 🎯 **Post-Migration Benefits**

### **Architectural Benefits**
```yaml
architecture_improvements:
  separation_of_concerns: "Clear MCP vs Compute separation"
  scalability: "Independent scaling of MCP and Compute"
  maintainability: "Focused codebases easier to maintain"
  specialization: "Each service optimized for its purpose"
```

### **Performance Benefits**
```yaml
performance_improvements:
  compute_optimization: "Toadstool optimized for compute workloads"
  mcp_optimization: "Squirrel optimized for MCP platform"
  resource_efficiency: "Better resource utilization"
  parallel_development: "Teams can optimize independently"
```

### **Development Benefits**
```yaml
development_improvements:
  focused_teams: "Teams focused on their expertise"
  faster_iteration: "Smaller codebases enable faster development"
  better_testing: "Focused testing strategies"
  clearer_responsibilities: "Clear ownership boundaries"
```

---

**This migration plan ensures a smooth transition from monolithic compute in Squirrel to specialized compute in Toadstool, enabling both projects to excel in their focused domains.** 🚀
