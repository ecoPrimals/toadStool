//! # ToadStool Auto-Configuration
//! 
//! Intelligent auto-discovery and zero-touch configuration for ToadStool.
//! Makes ToadStool "just work" for grandma while being AI-friendly.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use tracing::info;

use toadstool::error::ToadStoolResult;

pub mod hardware;
pub mod ecosystem;
pub mod natural_language;
pub mod installer;

/// Main auto-configuration engine
pub struct IntelligentAutoConfig {
    /// Hardware detection and optimization
    pub hardware_detector: hardware::HardwareDetector,
    /// Ecosystem service discovery
    pub ecosystem_discoverer: ecosystem::EcosystemDiscoverer,
    /// Natural language configuration
    pub nlp_processor: natural_language::NaturalLanguageProcessor,
}

impl Default for IntelligentAutoConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl IntelligentAutoConfig {
    /// Create new auto-configuration engine
    pub fn new() -> Self {
        Self {
            hardware_detector: hardware::HardwareDetector::new(),
            ecosystem_discoverer: ecosystem::EcosystemDiscoverer::new(),
            nlp_processor: natural_language::NaturalLanguageProcessor::new(),
        }
    }
    
    /// Zero-configuration startup - just works!
    pub async fn auto_configure() -> ToadStoolResult<AutoConfigResult> {
        info!("🧠 ToadStool Auto-Configuration Starting...");
        
        let mut auto_config = Self::new();
        
        // 1. Detect hardware capabilities
        let hardware = auto_config.hardware_detector.scan_system().await?;
        info!("🖥️ Detected: {} cores, {:.1}GB RAM, {} GPUs", 
              hardware.cpu_cores, hardware.memory_gb, hardware.gpu_count);
        
        // 2. Discover ecosystem services
        let ecosystem = auto_config.ecosystem_discoverer.discover_services().await?;
        info!("🌐 Found ecosystem services: {:?}", 
              ecosystem.discovered_services.keys().collect::<Vec<_>>());
        
        // 3. Generate optimal configuration
        let config = Self::generate_optimal_config(&hardware, &ecosystem).await?;
        info!("✅ Auto-configuration complete - ready to execute workloads!");
        
        // 4. Generate recommendations before moving values
        let recommendations = Self::generate_recommendations(&hardware, &ecosystem);
        
        Ok(AutoConfigResult {
            hardware_capabilities: hardware,
            ecosystem_services: ecosystem,
            generated_config: config,
            recommendations,
        })
    }
    
    /// Generate optimal configuration from detected capabilities
    async fn generate_optimal_config(
        hardware: &hardware::SystemCapabilities,
        ecosystem: &ecosystem::EcosystemMap,
    ) -> ToadStoolResult<OptimalConfiguration> {
        let mut config = OptimalConfiguration::default();
        
        // Configure based on hardware
        config.runtime_configs = Self::configure_runtimes(hardware);
        config.resource_limits = Self::configure_resource_limits(hardware);
        config.performance_profile = Self::determine_performance_profile(hardware);
        
        // Configure based on ecosystem
        if let Some(songbird) = ecosystem.get_service("songbird") {
            config.songbird_config = Some(SongbirdConfig {
                endpoint: songbird.endpoint.clone(),
                auto_register: true,
                heartbeat_interval: Duration::from_secs(30),
                capabilities: Self::generate_capabilities(hardware),
            });
        }
        
        // Security configuration based on environment
        config.security_profile = Self::determine_security_profile(hardware, ecosystem);
        
        Ok(config)
    }
    
    /// Configure resource limits based on hardware
    fn configure_resource_limits(hardware: &hardware::SystemCapabilities) -> ResourceLimits {
        ResourceLimits {
            max_cpu_cores: hardware.cpu_cores as f64,
            max_memory_gb: hardware.memory_gb * 0.9, // Leave 10% for system
            max_storage_gb: hardware.storage_info.iter()
                .map(|s| s.available_gb)
                .fold(0.0, |acc, x| acc + x),
            max_network_mbps: 1000.0, // Default network limit
        }
    }
    
    /// Determine performance profile based on hardware
    fn determine_performance_profile(hardware: &hardware::SystemCapabilities) -> PerformanceProfile {
        if hardware.cpu_cores >= 16 && hardware.memory_gb >= 32.0 {
            PerformanceProfile::MaxPerformance
        } else if hardware.cpu_cores >= 8 && hardware.memory_gb >= 16.0 {
            PerformanceProfile::Performance
        } else if hardware.cpu_cores >= 4 && hardware.memory_gb >= 8.0 {
            PerformanceProfile::Balanced
        } else {
            PerformanceProfile::PowerSaver
        }
    }
    
    /// Determine security profile based on environment
    fn determine_security_profile(hardware: &hardware::SystemCapabilities, _ecosystem: &ecosystem::EcosystemMap) -> SecurityProfile {
        if hardware.is_virtualized {
            // Already in a VM, can be less restrictive
            SecurityProfile::Standard
        } else if hardware.has_container_support() {
            // Can use container isolation
            SecurityProfile::High
        } else {
            // Need maximum security without containers
            SecurityProfile::Maximum
        }
    }
    
    /// Generate capabilities list for Songbird registration
    fn generate_capabilities(hardware: &hardware::SystemCapabilities) -> Vec<String> {
        let mut capabilities = vec![
            "execution".to_string(),
            "native_runtime".to_string(),
            "wasm_runtime".to_string(),
        ];
        
        if hardware.has_container_support() {
            capabilities.push("container_runtime".to_string());
        }
        
        if hardware.has_gpu_support() {
            capabilities.push("gpu_runtime".to_string());
            if let Some(platform) = &hardware.gpu_platform {
                capabilities.push(format!("gpu_{}", platform.to_lowercase()));
            }
        }
        
        capabilities.push(format!("cpu_cores_{}", hardware.cpu_cores));
        capabilities.push(format!("memory_gb_{}", hardware.memory_gb as u32));
        
        capabilities
    }
    
    /// Configure runtime engines based on hardware
    fn configure_runtimes(hardware: &hardware::SystemCapabilities) -> RuntimeConfigurations {
        let mut configs = RuntimeConfigurations::default();
        
        // Native runtime - always available
        configs.native = NativeRuntimeConfig {
            max_concurrent_processes: std::cmp::min(hardware.cpu_cores, 100),
            default_timeout_seconds: 300,
            memory_limit_mb: (hardware.memory_gb * 1024.0 * 0.8) as u64, // 80% of RAM
        };
        
        // Container runtime - if sufficient resources
        if hardware.cpu_cores >= 2 && hardware.memory_gb >= 4.0 {
            configs.container = Some(ContainerRuntimeConfig {
                engine: if hardware.has_docker { "docker" } else { "podman" }.to_string(),
                max_concurrent_containers: std::cmp::min(hardware.cpu_cores / 2, 20),
                default_memory_limit_mb: (hardware.memory_gb * 512.0) as u64, // 512MB per container
                enable_gpu_passthrough: hardware.gpu_count > 0,
            });
        }
        
        // WASM runtime - always available, lightweight
        configs.wasm = WasmRuntimeConfig {
            max_concurrent_instances: hardware.cpu_cores * 4, // WASM is lightweight
            default_memory_limit_mb: 128, // Conservative for WASM
            enable_wasi: true,
            cache_enabled: true,
        };
        
        // GPU runtime - if GPUs available
        if hardware.gpu_count > 0 {
            configs.gpu = Some(GpuRuntimeConfig {
                preferred_platform: hardware.gpu_platform.clone().unwrap_or_default(),
                memory_fraction: 0.8, // Use 80% of GPU memory
                enable_profiling: false, // Conservative default
            });
        }
        
        configs
    }
    
    /// Generate recommendations for the user
    fn generate_recommendations(
        hardware: &hardware::SystemCapabilities,
        ecosystem: &ecosystem::EcosystemMap,
    ) -> Vec<Recommendation> {
        let mut recommendations = Vec::new();
        
        // Hardware-based recommendations
        if hardware.memory_gb < 8.0 {
            recommendations.push(Recommendation {
                category: "Performance".to_string(),
                message: "Consider adding more RAM for better performance with large workloads".to_string(),
                priority: RecommendationPriority::Low,
            });
        }
        
        if hardware.gpu_count > 0 && hardware.gpu_memory_gb.unwrap_or(0.0) >= 8.0 {
            recommendations.push(Recommendation {
                category: "Optimization".to_string(),
                message: "Your GPU is perfect for machine learning workloads!".to_string(),
                priority: RecommendationPriority::Info,
            });
        }
        
        // Ecosystem recommendations
        if ecosystem.get_service("songbird").is_none() {
            recommendations.push(Recommendation {
                category: "Integration".to_string(),
                message: "Install Songbird for automatic service discovery and load balancing".to_string(),
                priority: RecommendationPriority::Medium,
            });
        }
        
        recommendations
    }
}

/// Result of auto-configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoConfigResult {
    pub hardware_capabilities: hardware::SystemCapabilities,
    pub ecosystem_services: ecosystem::EcosystemMap,
    pub generated_config: OptimalConfiguration,
    pub recommendations: Vec<Recommendation>,
}

/// Optimal configuration generated by auto-config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimalConfiguration {
    pub runtime_configs: RuntimeConfigurations,
    pub resource_limits: ResourceLimits,
    pub performance_profile: PerformanceProfile,
    pub security_profile: SecurityProfile,
    pub songbird_config: Option<SongbirdConfig>,
}

impl Default for OptimalConfiguration {
    fn default() -> Self {
        Self {
            runtime_configs: RuntimeConfigurations::default(),
            resource_limits: ResourceLimits::default(),
            performance_profile: PerformanceProfile::Balanced,
            security_profile: SecurityProfile::Standard,
            songbird_config: None,
        }
    }
}

/// Runtime engine configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct RuntimeConfigurations {
    pub native: NativeRuntimeConfig,
    pub container: Option<ContainerRuntimeConfig>,
    pub wasm: WasmRuntimeConfig,
    pub gpu: Option<GpuRuntimeConfig>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeRuntimeConfig {
    pub max_concurrent_processes: u32,
    pub default_timeout_seconds: u64,
    pub memory_limit_mb: u64,
}

impl Default for NativeRuntimeConfig {
    fn default() -> Self {
        Self {
            max_concurrent_processes: 10,
            default_timeout_seconds: 300,
            memory_limit_mb: 1024,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerRuntimeConfig {
    pub engine: String,
    pub max_concurrent_containers: u32,
    pub default_memory_limit_mb: u64,
    pub enable_gpu_passthrough: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmRuntimeConfig {
    pub max_concurrent_instances: u32,
    pub default_memory_limit_mb: u64,
    pub enable_wasi: bool,
    pub cache_enabled: bool,
}

impl Default for WasmRuntimeConfig {
    fn default() -> Self {
        Self {
            max_concurrent_instances: 20,
            default_memory_limit_mb: 128,
            enable_wasi: true,
            cache_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuRuntimeConfig {
    pub preferred_platform: String,
    pub memory_fraction: f64,
    pub enable_profiling: bool,
}

/// Resource limits configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_cpu_cores: f64,
    pub max_memory_gb: f64,
    pub max_storage_gb: f64,
    pub max_network_mbps: f64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_cpu_cores: 16.0,
            max_memory_gb: 32.0,
            max_storage_gb: 100.0,
            max_network_mbps: 1000.0,
        }
    }
}

/// Performance profiles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PerformanceProfile {
    PowerSaver,
    Balanced,
    Performance,
    MaxPerformance,
}

/// Security profiles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityProfile {
    Minimal,
    Standard,
    High,
    Maximum,
}

/// Songbird integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongbirdConfig {
    pub endpoint: String,
    pub auto_register: bool,
    pub heartbeat_interval: Duration,
    pub capabilities: Vec<String>,
}

/// Recommendation for the user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub category: String,
    pub message: String,
    pub priority: RecommendationPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationPriority {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_auto_configure() {
        let result = IntelligentAutoConfig::auto_configure().await;
        assert!(result.is_ok(), "Auto-configuration should succeed");
        
        let config = result.unwrap();
        assert!(config.hardware_capabilities.cpu_cores > 0, "Should detect CPU cores");
        assert!(config.hardware_capabilities.memory_gb > 0.0, "Should detect memory");
    }
    
    #[test]
    fn test_runtime_configuration_defaults() {
        let config = RuntimeConfigurations::default();
        assert_eq!(config.native.max_concurrent_processes, 10);
        assert!(config.wasm.enable_wasi);
    }
} 