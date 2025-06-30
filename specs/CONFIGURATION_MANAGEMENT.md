---
title: ToadStool Configuration Management Specification
description: Hierarchical configuration system with runtime adaptability and zero hardcoding
version: 1.0.0
date: 2025-01-26
author: ToadStool Configuration Team
priority: CRITICAL
status: CONFIG_SPEC
---

# ⚙️ Configuration Management Specification

## Executive Summary

ToadStool implements **zero-hardcoding configuration management** with hierarchical overrides, runtime adaptability, environment-specific settings, and complete type safety to ensure maximum flexibility and future-proofing.

---

## 🎯 **Configuration Philosophy**

### **Zero Hardcoding Principles**
```yaml
configuration_principles:
  no_magic_numbers: "All values configurable via configuration files"
  environment_driven: "Environment variables override configuration files"
  runtime_adaptable: "Configuration can be updated without restarts where safe"
  type_safe: "All configuration validated at load time"
  hierarchical: "Configuration inheritance with clear precedence rules"
  platform_agnostic: "Same configuration works across all platforms"
```

### **Configuration Hierarchy**
```rust
/// Configuration precedence (highest to lowest)
#[derive(Debug, Clone)]
pub enum ConfigurationSource {
    /// Command line arguments (highest precedence)
    CommandLine { args: Vec<String> },
    /// Environment variables
    Environment { prefix: String },
    /// Local configuration file (./toadstool.toml)
    LocalFile { path: PathBuf },
    /// User configuration file (~/.config/toadstool/config.toml)
    UserFile { path: PathBuf },
    /// System configuration file (/etc/toadstool/config.toml)
    SystemFile { path: PathBuf },
    /// Built-in defaults (lowest precedence)
    Defaults,
}
```

---

## 🏗️ **Configuration Structure**

### **Master Configuration Schema**
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ToadStoolConfiguration {
    /// Global settings that apply to all components
    #[validate(nested)]
    pub global: GlobalConfiguration,
    
    /// Runtime-specific configurations
    #[validate(nested)]
    pub runtimes: RuntimeConfigurations,
    
    /// Security and sandboxing configuration
    #[validate(nested)]
    pub security: SecurityConfiguration,
    
    /// Resource management configuration
    #[validate(nested)]
    pub resources: ResourceConfiguration,
    
    /// Communication and integration settings
    #[validate(nested)]
    pub communication: CommunicationConfiguration,
    
    /// Platform-specific overrides
    #[validate(nested)]
    pub platforms: HashMap<Platform, PlatformConfiguration>,
    
    /// Environment-specific overrides
    #[validate(nested)]
    pub environments: HashMap<String, EnvironmentConfiguration>,
    
    /// Feature flags and experimental settings
    pub features: FeatureConfiguration,
    
    /// Logging and observability configuration
    #[validate(nested)]
    pub observability: ObservabilityConfiguration,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct GlobalConfiguration {
    /// Service identification
    pub service_id: Option<String>,
    pub instance_id: Option<String>,
    pub cluster_id: Option<String>,
    
    /// Core behavior settings
    #[validate(range(min = 1, max = 10000))]
    pub max_concurrent_executions: Option<u32>,
    
    #[validate(range(min = 1000, max = 3600000))]
    pub default_timeout_ms: Option<u64>,
    
    #[validate(range(min = 1, max = 100))]
    pub worker_threads: Option<usize>,
    
    /// Startup and shutdown behavior
    #[validate(range(min = 1000, max = 300000))]
    pub startup_timeout_ms: Option<u64>,
    
    #[validate(range(min = 1000, max = 300000))]
    pub shutdown_timeout_ms: Option<u64>,
    
    pub graceful_shutdown: Option<bool>,
    
    /// Data directories
    pub data_dir: Option<PathBuf>,
    pub cache_dir: Option<PathBuf>,
    pub temp_dir: Option<PathBuf>,
    pub log_dir: Option<PathBuf>,
}
```

### **Runtime-Specific Configuration**
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct RuntimeConfigurations {
    /// Container runtime configuration
    #[validate(nested)]
    pub container: Option<ContainerRuntimeConfig>,
    
    /// WASM runtime configuration
    #[validate(nested)]
    pub wasm: Option<WasmRuntimeConfig>,
    
    /// Native runtime configuration
    #[validate(nested)]
    pub native: Option<NativeRuntimeConfig>,
    
    /// GPU compute configuration
    #[validate(nested)]
    pub gpu: Option<GpuRuntimeConfig>,
    
    /// Custom runtime configurations
    #[validate(nested)]
    pub custom: HashMap<String, CustomRuntimeConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ContainerRuntimeConfig {
    /// Container engine selection and configuration
    pub engine: ContainerEngine,
    
    /// Image management settings
    #[validate(nested)]
    pub image_config: ImageConfiguration,
    
    /// Network configuration
    #[validate(nested)]
    pub network_config: NetworkConfiguration,
    
    /// Storage configuration
    #[validate(nested)]
    pub storage_config: StorageConfiguration,
    
    /// Resource defaults
    #[validate(nested)]
    pub resource_defaults: ResourceDefaults,
    
    /// Security defaults
    #[validate(nested)]
    pub security_defaults: SecurityDefaults,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContainerEngine {
    Docker {
        socket_path: Option<PathBuf>,
        api_version: Option<String>,
        registry_mirrors: Vec<String>,
        insecure_registries: Vec<String>,
        storage_driver: Option<String>,
        storage_opts: HashMap<String, String>,
    },
    Containerd {
        socket_path: Option<PathBuf>,
        namespace: Option<String>,
        snapshotter: Option<String>,
        runtime_name: Option<String>,
    },
    Podman {
        socket_path: Option<PathBuf>,
        root_dir: Option<PathBuf>,
        runtime_dir: Option<PathBuf>,
        storage_driver: Option<String>,
    },
    Custom {
        name: String,
        config: HashMap<String, Value>,
        command_templates: HashMap<String, String>,
    },
}
```

---

## 🔧 **Configuration Loading and Merging**

### **Intelligent Configuration Loader**
```rust
#[derive(Debug)]
pub struct ConfigurationLoader {
    sources: Vec<ConfigurationSource>,
    validators: Vec<Box<dyn ConfigurationValidator>>,
    transformers: Vec<Box<dyn ConfigurationTransformer>>,
    watchers: Vec<Box<dyn ConfigurationWatcher>>,
}

impl ConfigurationLoader {
    /// Load configuration with hierarchical merging
    pub async fn load_configuration() -> Result<ToadStoolConfiguration> {
        let mut config = ToadStoolConfiguration::default();
        let loader = Self::new();
        
        // Load from all sources in precedence order
        for source in &loader.sources {
            match loader.load_from_source(source).await {
                Ok(source_config) => {
                    config = config.merge_with(source_config)?;
                }
                Err(e) if source.is_required() => return Err(e),
                Err(e) => {
                    warn!("Optional configuration source failed: {} - {}", source, e);
                }
            }
        }
        
        // Apply transformations
        for transformer in &loader.transformers {
            config = transformer.transform(config).await?;
        }
        
        // Validate final configuration
        config.validate()?;
        
        // Set up configuration watching for runtime updates
        loader.setup_watchers(&config).await?;
        
        Ok(config)
    }
    
    /// Merge configurations with intelligent conflict resolution
    fn merge_configurations(
        base: ToadStoolConfiguration,
        override_config: ToadStoolConfiguration
    ) -> Result<ToadStoolConfiguration> {
        // Use structured merging with type-aware conflict resolution
        let mut merged = base;
        
        // Global settings merge
        merged.global = Self::merge_global_config(merged.global, override_config.global)?;
        
        // Runtime configurations merge
        merged.runtimes = Self::merge_runtime_configs(merged.runtimes, override_config.runtimes)?;
        
        // Platform-specific merging
        for (platform, platform_config) in override_config.platforms {
            match merged.platforms.get(&platform) {
                Some(existing) => {
                    merged.platforms.insert(platform, existing.merge_with(platform_config)?);
                }
                None => {
                    merged.platforms.insert(platform, platform_config);
                }
            }
        }
        
        // Environment-specific merging
        for (env, env_config) in override_config.environments {
            match merged.environments.get(&env) {
                Some(existing) => {
                    merged.environments.insert(env, existing.merge_with(env_config)?);
                }
                None => {
                    merged.environments.insert(env, env_config);
                }
            }
        }
        
        Ok(merged)
    }
}
```

### **Runtime Configuration Updates**
```rust
#[derive(Debug)]
pub struct ConfigurationManager {
    current_config: Arc<RwLock<ToadStoolConfiguration>>,
    update_notifier: broadcast::Sender<ConfigurationUpdate>,
    validation_engine: Arc<ValidationEngine>,
    rollback_manager: Arc<RollbackManager>,
}

impl ConfigurationManager {
    /// Update configuration at runtime with safety checks
    pub async fn update_configuration(
        &self,
        update: ConfigurationUpdate
    ) -> Result<ConfigurationUpdateResult> {
        // Validate the update
        let validation_result = self.validation_engine.validate_update(&update).await?;
        if !validation_result.is_safe_for_runtime_update() {
            return Ok(ConfigurationUpdateResult::RequiresRestart {
                reason: validation_result.restart_reason(),
                changes: update.changes,
            });
        }
        
        // Create backup for rollback
        let backup = {
            let current = self.current_config.read().await;
            current.clone()
        };
        
        // Apply the update
        let updated_config = {
            let mut current = self.current_config.write().await;
            let new_config = current.apply_update(update.clone())?;
            
            // Validate the complete updated configuration
            new_config.validate()?;
            
            *current = new_config.clone();
            new_config
        };
        
        // Store backup for potential rollback
        self.rollback_manager.store_backup(backup).await?;
        
        // Notify components of configuration change
        self.update_notifier.send(update)?;
        
        Ok(ConfigurationUpdateResult::Applied {
            new_config: updated_config,
            backup_id: self.rollback_manager.latest_backup_id(),
        })
    }
    
    /// Rollback to previous configuration
    pub async fn rollback_configuration(&self, backup_id: Option<String>) -> Result<()> {
        let backup_config = self.rollback_manager.restore_backup(backup_id).await?;
        
        {
            let mut current = self.current_config.write().await;
            *current = backup_config;
        }
        
        // Notify components of rollback
        self.update_notifier.send(ConfigurationUpdate::rollback())?;
        
        Ok(())
    }
}
```

---

## 🌍 **Environment-Specific Configuration**

### **Environment Detection and Adaptation**
```rust
#[derive(Debug, Clone)]
pub struct EnvironmentDetector {
    detection_strategies: Vec<Box<dyn EnvironmentDetectionStrategy>>,
    environment_mappings: HashMap<String, EnvironmentProfile>,
}

impl EnvironmentDetector {
    /// Automatically detect environment and load appropriate configuration
    pub async fn detect_and_configure() -> Result<ToadStoolConfiguration> {
        let detector = Self::new();
        let environment = detector.detect_environment().await?;
        
        info!("Detected environment: {}", environment);
        
        // Load base configuration
        let mut config = ConfigurationLoader::load_configuration().await?;
        
        // Apply environment-specific overrides
        if let Some(env_config) = config.environments.get(&environment) {
            config = config.apply_environment_overrides(env_config.clone())?;
        }
        
        // Apply platform-specific overrides for this environment
        let platform = Platform::current();
        if let Some(platform_config) = config.platforms.get(&platform) {
            if let Some(env_platform_config) = platform_config.environment_overrides.get(&environment) {
                config = config.apply_platform_environment_overrides(
                    platform,
                    env_platform_config.clone()
                )?;
            }
        }
        
        // Validate final configuration
        config.validate_for_environment(&environment)?;
        
        Ok(config)
    }
    
    async fn detect_environment(&self) -> Result<String> {
        // Try each detection strategy in order
        for strategy in &self.detection_strategies {
            if let Some(environment) = strategy.detect_environment().await? {
                return Ok(environment);
            }
        }
        
        // Default to "development" if no environment detected
        Ok("development".to_string())
    }
}

#[async_trait::async_trait]
pub trait EnvironmentDetectionStrategy: Send + Sync {
    async fn detect_environment(&self) -> Result<Option<String>>;
    fn priority(&self) -> u8; // Higher number = higher priority
}

/// Environment detection strategies
pub struct KubernetesEnvironmentDetector;
pub struct DockerEnvironmentDetector;
pub struct CloudEnvironmentDetector;
pub struct EnvironmentVariableDetector;
pub struct ConfigFileDetector;
```

---

## 🔒 **Configuration Security and Validation**

### **Comprehensive Validation Framework**
```rust
#[derive(Debug)]
pub struct ValidationEngine {
    validators: HashMap<String, Box<dyn ConfigurationValidator>>,
    security_checker: Box<dyn ConfigurationSecurityChecker>,
    compatibility_checker: Box<dyn CompatibilityChecker>,
}

#[async_trait::async_trait]
pub trait ConfigurationValidator: Send + Sync {
    /// Validate a configuration section
    async fn validate(&self, config: &Value) -> Result<ValidationResult>;
    
    /// Get the configuration path this validator handles
    fn handles_path(&self) -> &str;
    
    /// Whether this validation is required or optional
    fn is_required(&self) -> bool { true }
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
    pub suggestions: Vec<ConfigurationSuggestion>,
    pub is_safe_for_runtime_update: bool,
}

impl ValidationEngine {
    /// Comprehensive configuration validation
    pub async fn validate_configuration(
        &self,
        config: &ToadStoolConfiguration
    ) -> Result<ValidationResult> {
        let mut overall_result = ValidationResult::new();
        
        // Validate each section with appropriate validator
        let config_value = serde_json::to_value(config)?;
        for (path, validator) in &self.validators {
            if let Some(section) = Self::extract_config_section(&config_value, path) {
                let section_result = validator.validate(&section).await?;
                overall_result.merge(section_result);
            } else if validator.is_required() {
                overall_result.add_error(ValidationError::MissingRequiredSection {
                    path: path.clone(),
                });
            }
        }
        
        // Security validation
        let security_result = self.security_checker.check_security(config).await?;
        overall_result.merge(security_result);
        
        // Compatibility validation
        let compatibility_result = self.compatibility_checker.check_compatibility(config).await?;
        overall_result.merge(compatibility_result);
        
        Ok(overall_result)
    }
}
```

### **Configuration Security**
```rust
#[async_trait::async_trait]
pub trait ConfigurationSecurityChecker: Send + Sync {
    /// Check configuration for security issues
    async fn check_security(&self, config: &ToadStoolConfiguration) -> Result<ValidationResult>;
}

pub struct DefaultSecurityChecker;

#[async_trait::async_trait]
impl ConfigurationSecurityChecker for DefaultSecurityChecker {
    async fn check_security(&self, config: &ToadStoolConfiguration) -> Result<ValidationResult> {
        let mut result = ValidationResult::new();
        
        // Check for insecure settings
        if let Some(true) = config.security.disable_sandboxing {
            result.add_warning(ValidationWarning::InsecureSetting {
                setting: "security.disable_sandboxing".to_string(),
                reason: "Disabling sandboxing reduces security".to_string(),
                recommendation: "Enable sandboxing for production environments".to_string(),
            });
        }
        
        // Check for overly permissive settings
        if let Some(level) = &config.security.default_security_level {
            if *level == SecurityLevel::Unrestricted {
                result.add_error(ValidationError::SecurityRisk {
                    setting: "security.default_security_level".to_string(),
                    risk: "Unrestricted security level allows unlimited access".to_string(),
                });
            }
        }
        
        // Check for exposed sensitive data
        self.check_for_exposed_secrets(config, &mut result)?;
        
        Ok(result)
    }
}
```

---

## 🎛️ **Feature Flags and Experimental Configuration**

### **Dynamic Feature Management**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureConfiguration {
    /// Global feature flags
    pub global_flags: HashMap<String, FeatureFlag>,
    
    /// Runtime-specific feature flags
    pub runtime_flags: HashMap<RuntimeType, HashMap<String, FeatureFlag>>,
    
    /// Platform-specific feature flags
    pub platform_flags: HashMap<Platform, HashMap<String, FeatureFlag>>,
    
    /// Environment-specific feature flags
    pub environment_flags: HashMap<String, HashMap<String, FeatureFlag>>,
    
    /// Experimental features configuration
    pub experimental: ExperimentalConfiguration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlag {
    /// Whether the feature is enabled
    pub enabled: bool,
    
    /// Conditions for enabling the feature
    pub conditions: Vec<FeatureCondition>,
    
    /// Rollout percentage (0-100)
    pub rollout_percentage: Option<u8>,
    
    /// Configuration for the feature
    pub config: Option<Value>,
    
    /// Description of the feature
    pub description: Option<String>,
    
    /// Whether this is an experimental feature
    pub experimental: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeatureCondition {
    /// Enable based on environment
    Environment { environments: Vec<String> },
    
    /// Enable based on platform
    Platform { platforms: Vec<Platform> },
    
    /// Enable based on configuration value
    ConfigValue { path: String, value: Value },
    
    /// Enable based on capability availability
    CapabilityAvailable { capability: String },
    
    /// Custom condition
    Custom { condition: String, parameters: HashMap<String, Value> },
}

pub struct FeatureManager {
    config: Arc<RwLock<FeatureConfiguration>>,
    condition_evaluator: Box<dyn ConditionEvaluator>,
}

impl FeatureManager {
    /// Check if a feature is enabled
    pub async fn is_feature_enabled(
        &self,
        feature_name: &str,
        context: &FeatureContext
    ) -> Result<bool> {
        let config = self.config.read().await;
        
        // Check in order of specificity
        if let Some(flag) = self.get_environment_flag(&config, feature_name, &context.environment) {
            return self.evaluate_feature_flag(flag, context).await;
        }
        
        if let Some(flag) = self.get_platform_flag(&config, feature_name, &context.platform) {
            return self.evaluate_feature_flag(flag, context).await;
        }
        
        if let Some(flag) = self.get_runtime_flag(&config, feature_name, &context.runtime) {
            return self.evaluate_feature_flag(flag, context).await;
        }
        
        if let Some(flag) = config.global_flags.get(feature_name) {
            return self.evaluate_feature_flag(flag, context).await;
        }
        
        // Default to disabled
        Ok(false)
    }
}
```

This configuration management specification ensures ToadStool achieves true zero-hardcoding with runtime adaptability, comprehensive validation, and secure configuration handling across all environments and platforms. 