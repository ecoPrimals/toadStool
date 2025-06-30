---
title: ToadStool Migration and Integration Specification
description: Complete migration of Squirrel compute infrastructure with zero functionality loss
version: 1.0.0
date: 2025-01-26
author: ToadStool Migration Team
priority: CRITICAL
status: MIGRATION_SPEC
---

# 🚚 Migration and Integration Specification

## Executive Summary

This specification defines the **complete migration strategy** for all compute infrastructure from Squirrel to ToadStool, ensuring zero functionality loss, seamless integration, and backward compatibility during the transition.

---

## 📋 **Migration Inventory and Mapping**

### **Complete Component Mapping**
```yaml
migration_mapping:
  from_squirrel:
    # Sandbox System (toToadStool/sandbox/)
    sandbox_core:
      source: "toToadStool/sandbox/"
      target: "crates/security/sandbox/"
      components:
        - "basic.rs -> basic_sandbox.rs"
        - "capabilities.rs -> capability_detection.rs"
        - "cross_platform.rs -> cross_platform_sandbox.rs"
        - "errors.rs -> sandbox_errors.rs"
        - "traits.rs -> sandbox_traits.rs"
        - "testing.rs -> sandbox_testing.rs"
        - "seccomp.rs -> linux/seccomp_engine.rs"
    
    platform_implementations:
      linux:
        source: "toToadStool/sandbox/linux/"
        target: "crates/security/sandbox/linux/"
        components:
          - "config.rs -> linux_config.rs"
          - "mod.rs -> mod.rs"
          - "resources.rs -> resource_control.rs"
          - "sandbox.rs -> linux_sandbox.rs"
          - "sandbox_io.rs -> io_control.rs"
          - "seccomp.rs -> seccomp_filters.rs"
          - "tests.rs -> integration_tests.rs"
          - "trait_impl.rs -> trait_implementations.rs"
          - "utils.rs -> linux_utils.rs"
      
      macos:
        source: "toToadStool/sandbox/macos/"
        target: "crates/security/sandbox/macos/"
        components:
          - "compatibility.rs -> macos_compatibility.rs"
          - "mod.rs -> mod.rs"
          - "platform_optimization.rs -> performance_tuning.rs"
          - "process_management.rs -> process_control.rs"
          - "resource_limits.rs -> resource_management.rs"
          - "sandbox_profiles.rs -> app_sandbox_profiles.rs"
          - "security_context.rs -> security_contexts.rs"
          - "sip_integration.rs -> system_integrity.rs"
          - "tcc_integration.rs -> privacy_permissions.rs"
    
    resource_monitoring:
      source: "toToadStool/resource-monitoring/resource_monitor.rs"
      target: "crates/management/resources/resource_monitor.rs"
      size: "984 lines of production code"
      components:
        - "ResourceMonitor struct -> ResourceManager"
        - "ResourceLimits -> ResourceConstraints"
        - "ResourceUsage -> ResourceMetrics"
    
    sdk_components:
      source: "toToadStool/sdk/sandbox.rs"
      target: "crates/client/sdk/"
      size: "525 lines of production code"
      components:
        - "SandboxConfig -> SecurityConfiguration"
        - "SecurityLevel -> IsolationLevel"
        - "Permission -> Capability"
        - "ResourceLimits -> ResourceConstraints"
        - "SandboxManager -> SecurityManager"
```

---

## 🏗️ **Integration Architecture**

### **Sandbox System Integration**
```rust
/// Unified sandbox integration preserving all existing functionality
#[derive(Debug)]
pub struct IntegratedSandboxSystem {
    /// Cross-platform sandbox manager
    cross_platform_manager: CrossPlatformSandboxManager,
    /// Platform-specific implementations
    platform_implementations: HashMap<Platform, Box<dyn PlatformSandbox>>,
    /// Capability detection system
    capability_detector: CapabilityDetector,
    /// Resource monitoring integration
    resource_monitor: Arc<ResourceMonitor>,
    /// Security context manager
    security_manager: Arc<SecurityContextManager>,
    /// Migration compatibility layer
    compatibility_layer: SquirrelCompatibilityLayer,
}

impl IntegratedSandboxSystem {
    /// Migrate from existing Squirrel sandbox implementation
    pub async fn migrate_from_squirrel(
        squirrel_config: SquirrelSandboxConfig
    ) -> Result<Self> {
        // Initialize platform-specific implementations
        let mut platform_implementations = HashMap::new();
        
        // Linux implementation migration
        #[cfg(target_family = "unix")]
        {
            let linux_sandbox = LinuxSandbox::migrate_from_squirrel(
                &squirrel_config.linux_config
            ).await?;
            platform_implementations.insert(Platform::Linux, Box::new(linux_sandbox));
        }
        
        // macOS implementation migration
        #[cfg(target_os = "macos")]
        {
            let macos_sandbox = MacOSSandbox::migrate_from_squirrel(
                &squirrel_config.macos_config
            ).await?;
            platform_implementations.insert(Platform::MacOS, Box::new(macos_sandbox));
        }
        
        // Windows implementation migration
        #[cfg(target_os = "windows")]
        {
            let windows_sandbox = WindowsSandbox::migrate_from_squirrel(
                &squirrel_config.windows_config
            ).await?;
            platform_implementations.insert(Platform::Windows, Box::new(windows_sandbox));
        }
        
        Ok(Self {
            cross_platform_manager,
            platform_implementations,
            capability_detector,
            resource_monitor,
            security_manager,
            compatibility_layer,
        })
    }
}
```

### **Resource Monitor Migration**
```rust
/// Migrate existing resource monitoring system
impl ResourceMonitor {
    /// Migrate from Squirrel ResourceMonitor preserving all functionality
    pub async fn migrate_from_squirrel(
        squirrel_monitor: &SquirrelResourceMonitor
    ) -> Result<Self> {
        let mut toadstool_monitor = Self::new();
        
        // Migrate existing process registrations
        let registered_processes = squirrel_monitor.get_all_registered_processes().await?;
        for (plugin_id, process_info) in registered_processes {
            toadstool_monitor.register_process(
                plugin_id,
                process_info.process_handle,
                &process_info.executable_path
            ).await?;
            
            // Migrate resource limits
            if let Ok(limits) = squirrel_monitor.get_resource_limits(plugin_id).await {
                let toadstool_limits = Self::convert_squirrel_limits(limits)?;
                toadstool_monitor.set_resource_limits(plugin_id, toadstool_limits).await?;
            }
        }
        
        // Migrate monitoring configuration
        toadstool_monitor.set_monitor_interval(
            squirrel_monitor.get_monitor_interval()
        );
        
        if squirrel_monitor.is_monitoring_enabled() {
            toadstool_monitor.enable_monitoring();
            toadstool_monitor.start_monitoring().await?;
        }
        
        // Migrate historical data if needed
        if let Ok(usage_history) = squirrel_monitor.get_usage_history().await {
            toadstool_monitor.import_usage_history(usage_history).await?;
        }
        
        Ok(toadstool_monitor)
    }
    
    /// Convert Squirrel ResourceLimits to ToadStool ResourceConstraints
    fn convert_squirrel_limits(
        squirrel_limits: SquirrelResourceLimits
    ) -> Result<ResourceConstraints> {
        Ok(ResourceConstraints {
            max_cpu_percent: squirrel_limits.max_cpu_percent,
            max_memory_bytes: squirrel_limits.max_memory_bytes,
            max_disk_mb: squirrel_limits.max_disk_mb,
            max_threads: squirrel_limits.max_threads,
            // Map additional fields that may exist
            max_file_handles: squirrel_limits.max_file_handles.unwrap_or(1000),
            max_network_connections: squirrel_limits.max_network_connections.unwrap_or(100),
            execution_timeout: squirrel_limits.execution_timeout_ms
                .map(|ms| Duration::from_millis(ms)),
        })
    }
}
```

---

## 🔄 **Migration Phases and Compatibility**

### **Phase 1: Infrastructure Migration**
```rust
/// Migration coordinator for systematic component transfer
#[derive(Debug)]
pub struct MigrationCoordinator {
    migration_plan: MigrationPlan,
    compatibility_validator: CompatibilityValidator,
    rollback_manager: RollbackManager,
    progress_tracker: ProgressTracker,
}

impl MigrationCoordinator {
    /// Execute Phase 1: Infrastructure Migration
    pub async fn execute_infrastructure_migration(&self) -> Result<MigrationResult> {
        let mut result = MigrationResult::new();
        
        // Step 1: Migrate core sandbox traits and errors
        result.add_step(
            self.migrate_sandbox_core().await
                .context("Failed to migrate sandbox core")?
        );
        
        // Step 2: Migrate platform-specific implementations
        result.add_step(
            self.migrate_platform_implementations().await
                .context("Failed to migrate platform implementations")?
        );
        
        // Step 3: Migrate resource monitoring system
        result.add_step(
            self.migrate_resource_monitoring().await
                .context("Failed to migrate resource monitoring")?
        );
        
        // Step 4: Migrate SDK components
        result.add_step(
            self.migrate_sdk_components().await
                .context("Failed to migrate SDK components")?
        );
        
        Ok(result)
    }
}
```

### **Phase 2: API Compatibility Layer**
```rust
/// Compatibility layer to ensure Squirrel can use ToadStool seamlessly
#[derive(Debug)]
pub struct SquirrelCompatibilityLayer {
    toadstool_client: ToadStoolClient,
    api_translator: ApiTranslator,
    event_mapper: EventMapper,
    legacy_support: LegacySupport,
}

impl SquirrelCompatibilityLayer {
    /// Create plugin sandbox using legacy Squirrel API
    pub async fn create_sandbox(
        &self,
        plugin_id: Uuid,
        squirrel_config: SquirrelSandboxConfig
    ) -> Result<()> {
        // Translate Squirrel config to ToadStool format
        let toadstool_config = self.api_translator
            .translate_sandbox_config(squirrel_config)
            .await?;
        
        // Execute via ToadStool
        let execution_request = ExecutionRequest {
            execution_id: Uuid::new_v4(),
            plugin_spec: self.create_plugin_spec_from_legacy(plugin_id).await?,
            security_policy: toadstool_config.security_policy,
            resource_requirements: toadstool_config.resource_requirements,
            // ... other fields
        };
        
        self.toadstool_client.execute_plugin(execution_request).await
            .map_err(|e| self.map_toadstool_error_to_squirrel(e))?;
        
        Ok(())
    }
    
    /// Legacy resource monitoring interface
    pub async fn track_resources(&self, plugin_id: Uuid) -> Result<SquirrelResourceUsage> {
        let toadstool_usage = self.toadstool_client
            .get_resource_usage(plugin_id)
            .await?;
        
        // Convert ToadStool metrics back to Squirrel format
        Ok(SquirrelResourceUsage {
            cpu_percent: toadstool_usage.utilization.cpu_percent as f32,
            memory_bytes: toadstool_usage.utilization.memory_bytes,
            disk_mb: toadstool_usage.utilization.storage_io.total_bytes_read as f32 / 1_000_000.0,
            network_mb: toadstool_usage.utilization.network_usage.total_bytes_transferred as f32 / 1_000_000.0,
            timestamp: toadstool_usage.timestamp,
        })
    }
}
```

---

## 🛠️ **Migration Tools and Utilities**

### **Automated Migration Utilities**
```rust
/// Automated code migration utility
#[derive(Debug)]
pub struct CodeMigrationTool {
    file_processor: FileProcessor,
    import_rewriter: ImportRewriter,
    api_translator: ApiTranslator,
    test_migrator: TestMigrator,
}

impl CodeMigrationTool {
    /// Migrate Squirrel codebase files to ToadStool
    pub async fn migrate_codebase(&self, migration_spec: MigrationSpec) -> Result<MigrationReport> {
        let mut report = MigrationReport::new();
        
        for file_mapping in &migration_spec.file_mappings {
            let migration_result = self.migrate_file(file_mapping).await?;
            report.add_file_result(migration_result);
        }
        
        // Update imports and dependencies
        let import_update_result = self.update_imports(&migration_spec).await?;
        report.add_import_updates(import_update_result);
        
        // Migrate tests
        let test_migration_result = self.migrate_tests(&migration_spec).await?;
        report.add_test_migration(test_migration_result);
        
        Ok(report)
    }
    
    async fn migrate_file(&self, file_mapping: &FileMapping) -> Result<FileMigrationResult> {
        let source_content = fs::read_to_string(&file_mapping.source_path).await?;
        
        // Process the file content
        let processed_content = self.file_processor
            .process_file_content(source_content, file_mapping)
            .await?;
        
        // Update imports
        let updated_content = self.import_rewriter
            .rewrite_imports(processed_content, file_mapping)
            .await?;
        
        // Translate API calls if needed
        let translated_content = self.api_translator
            .translate_api_calls(updated_content, file_mapping)
            .await?;
        
        // Write to target location
        fs::create_dir_all(file_mapping.target_path.parent().unwrap()).await?;
        fs::write(&file_mapping.target_path, translated_content).await?;
        
        Ok(FileMigrationResult {
            source_path: file_mapping.source_path.clone(),
            target_path: file_mapping.target_path.clone(),
            lines_migrated: translated_content.lines().count(),
            status: MigrationStatus::Success,
        })
    }
}
```

### **Migration Validation and Testing**
```rust
/// Comprehensive migration validation
#[derive(Debug)]
pub struct MigrationValidator {
    functionality_tester: FunctionalityTester,
    performance_validator: PerformanceValidator,
    compatibility_checker: CompatibilityChecker,
    regression_tester: RegressionTester,
}

impl MigrationValidator {
    /// Validate complete migration with zero functionality loss
    pub async fn validate_migration(&self) -> Result<ValidationReport> {
        let mut report = ValidationReport::new();
        
        // Test core functionality preservation
        let functionality_result = self.test_functionality_preservation().await?;
        report.add_functionality_test(functionality_result);
        
        // Validate performance characteristics
        let performance_result = self.validate_performance_parity().await?;
        report.add_performance_validation(performance_result);
        
        // Check API compatibility
        let compatibility_result = self.check_api_compatibility().await?;
        report.add_compatibility_check(compatibility_result);
        
        // Run regression tests
        let regression_result = self.run_regression_tests().await?;
        report.add_regression_test(regression_result);
        
        Ok(report)
    }
    
    async fn test_functionality_preservation(&self) -> Result<FunctionalityTestResult> {
        let mut test_result = FunctionalityTestResult::new();
        
        // Test each migrated component
        test_result.add_test(
            self.test_sandbox_functionality().await
                .context("Sandbox functionality test failed")?
        );
        
        test_result.add_test(
            self.test_resource_monitoring_functionality().await
                .context("Resource monitoring functionality test failed")?
        );
        
        test_result.add_test(
            self.test_security_functionality().await
                .context("Security functionality test failed")?
        );
        
        test_result.add_test(
            self.test_cross_platform_functionality().await
                .context("Cross-platform functionality test failed")?
        );
        
        Ok(test_result)
    }
}
```

---

## 📊 **Migration Success Criteria**

### **Zero Functionality Loss Validation**
```yaml
success_criteria:
  functionality_preservation:
    - "All Squirrel sandbox features work identically in ToadStool"
    - "Resource monitoring provides same or better accuracy"
    - "Security isolation maintains same security guarantees"
    - "Cross-platform behavior is consistent"
    - "Performance characteristics meet or exceed Squirrel"
  
  api_compatibility:
    - "Existing Squirrel code works without modification during transition"
    - "All public APIs have compatible replacements"
    - "Error handling and edge cases preserved"
    - "Configuration migration is seamless"
  
  migration_completeness:
    - "All 984 lines of resource monitoring code migrated"
    - "All 525 lines of SDK code migrated"
    - "All platform-specific implementations migrated"
    - "All tests migrated and passing"
    - "Documentation updated and complete"
  
  operational_continuity:
    - "Zero downtime migration possible"
    - "Rollback procedures tested and working"
    - "Monitoring and observability maintained"
    - "Production deployment successful"
```

### **Migration Timeline and Checkpoints**
```yaml
migration_schedule:
  week_1:
    - "Core infrastructure migration"
    - "Platform implementations transfer"
    - "Basic functionality testing"
  
  week_2:
    - "Resource monitoring integration"
    - "SDK component migration"
    - "Compatibility layer development"
  
  week_3:
    - "Integration testing"
    - "Performance validation"
    - "Regression testing"
  
  week_4:
    - "Production migration"
    - "Monitoring setup"
    - "Documentation completion"

checkpoints:
  infrastructure_complete:
    description: "All core components migrated and building"
    tests: ["compilation", "basic_functionality", "cross_platform"]
  
  functionality_complete:
    description: "All features working with same behavior"
    tests: ["feature_parity", "api_compatibility", "edge_cases"]
  
  migration_ready:
    description: "Ready for production deployment"
    tests: ["performance", "security", "reliability", "rollback"]
```

This specification ensures that the migration from Squirrel to ToadStool preserves every line of functionality while providing a clear path for integration and future enhancements. 