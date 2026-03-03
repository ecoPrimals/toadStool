// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(deprecated)] // This example demonstrates legacy configuration APIs
//! Configuration Management Demo
//!
//! This demo showcases ToadStool's comprehensive environment variable configuration system
//! that eliminates hardcoded values and provides configurable deployment options.
//!
//! ## Running the Demo
//!
//! ```bash
//! # Run with default values
//! cargo run --example config_management_demo
//!
//! # Run with custom environment variables
//! TOADSTOOL_ENV=production \
//! TOADSTOOL_SONGBIRD_PORT=9080 \
//! TOADSTOOL_DEBUG=true \
//! TOADSTOOL_MAX_CPU_PERCENT=80 \
//! cargo run --example config_management_demo
//!
//! # Run with a complete production configuration
//! TOADSTOOL_ENV=production \
//! TOADSTOOL_BIND_ADDRESS=0.0.0.0 \
//! TOADSTOOL_TOADSTOOL_PORT=8080 \
//! TOADSTOOL_SONGBIRD_PORT=8081 \
//! TOADSTOOL_BEARDOG_PORT=8082 \
//! TOADSTOOL_NESTGATE_PORT=8083 \
//! TOADSTOOL_SQUIRREL_PORT=8084 \
//! TOADSTOOL_TLS_ENABLED=true \
//! TOADSTOOL_AUTH_ENABLED=true \
//! TOADSTOOL_METRICS_ENABLED=true \
//! TOADSTOOL_LOG_LEVEL=info \
//! TOADSTOOL_MAX_CPU_PERCENT=75 \
//! TOADSTOOL_MAX_MEMORY_BYTES=17179869184 \
//! TOADSTOOL_WORKER_THREADS=16 \
//! cargo run --example config_management_demo
//! ```

use std::time::Duration;
use tracing::{info, warn};

use toadstool_config::{
    config_utils::ConfigUtils,
    env_config::{
        EnvironmentConfig, MonitoringEnvConfig, NetworkEnvConfig, ResourceEnvConfig,
        SecurityEnvConfig,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("🚀 ToadStool Configuration Management Demo");
    info!("===========================================");

    // Demonstrate the old way (hardcoded values)
    demonstrate_old_hardcoded_approach();

    // Demonstrate the new way (environment-aware configuration)
    demonstrate_new_environment_aware_approach().await?;

    // Demonstrate different environment configurations
    demonstrate_environment_specific_configs().await?;

    // Demonstrate configuration validation and error handling
    demonstrate_configuration_validation().await?;

    // Demonstrate service discovery with configurable endpoints
    demonstrate_service_discovery().await?;

    // Demonstrate resource limit configuration
    demonstrate_resource_configuration().await?;

    // Show all current configuration values
    demonstrate_configuration_inspection();

    info!("✅ Configuration Management Demo completed successfully!");
    Ok(())
}

/// Demonstrate the old hardcoded approach (what we're replacing)
fn demonstrate_old_hardcoded_approach() {
    info!("📚 OLD APPROACH: Hardcoded Values");
    info!("================================");

    // This is what we used to do (hardcoded values)
    let songbird_port = 8080u16; // HARDCODED
    let beardog_port = 8081u16; // HARDCODED
    let nestgate_port = 8082u16; // HARDCODED
    let localhost = "127.0.0.1"; // HARDCODED
    let max_cpu = 90.0f64; // HARDCODED
    let max_memory = 8 * 1024 * 1024 * 1024u64; // HARDCODED - 8GB
    let request_timeout = Duration::from_secs(30); // HARDCODED

    info!("❌ Hardcoded Songbird port: {}", songbird_port);
    info!("❌ Hardcoded BearDog port: {}", beardog_port);
    info!("❌ Hardcoded NestGate port: {}", nestgate_port);
    info!("❌ Hardcoded localhost: {}", localhost);
    info!("❌ Hardcoded max CPU: {}%", max_cpu);
    info!("❌ Hardcoded max memory: {} bytes", max_memory);
    info!("❌ Hardcoded request timeout: {:?}", request_timeout);

    warn!("🚨 Problems with hardcoded values:");
    warn!("   - Cannot adapt to different environments");
    warn!("   - No runtime configuration changes");
    warn!("   - Port conflicts in deployments");
    warn!("   - Not suitable for production");
    warn!("   - No environment-specific settings");

    println!();
}

/// Demonstrate the new environment-aware configuration approach
async fn demonstrate_new_environment_aware_approach() -> Result<(), Box<dyn std::error::Error>> {
    info!("✨ NEW APPROACH: Environment-Aware Configuration");
    info!("===============================================");

    // This is the new way (environment-aware configuration)
    let songbird_port = ConfigUtils::get_songbird_port();
    let beardog_port = ConfigUtils::get_beardog_port();
    let nestgate_port = ConfigUtils::get_nestgate_port();
    let bind_address = ConfigUtils::get_bind_address();
    let max_cpu = ConfigUtils::get_max_cpu_usage();
    let max_memory = ConfigUtils::get_max_memory_usage();
    let request_timeout = ConfigUtils::get_request_timeout();

    info!("✅ Environment-aware Songbird port: {}", songbird_port);
    info!("✅ Environment-aware BearDog port: {}", beardog_port);
    info!("✅ Environment-aware NestGate port: {}", nestgate_port);
    info!("✅ Environment-aware bind address: {}", bind_address);
    info!("✅ Environment-aware max CPU: {}%", max_cpu);
    info!("✅ Environment-aware max memory: {} bytes", max_memory);
    info!(
        "✅ Environment-aware request timeout: {:?}",
        request_timeout
    );

    // Show service endpoints
    let endpoints = ConfigUtils::get_service_endpoints();
    info!("🔗 Service endpoints:");
    for (service, endpoint) in endpoints {
        info!("   {}: {}", service, endpoint);
    }

    info!("🎯 Benefits of environment-aware configuration:");
    info!("   ✅ Adapts to different environments (dev/staging/prod)");
    info!("   ✅ No hardcoded values in source code");
    info!("   ✅ Easy deployment configuration");
    info!("   ✅ Runtime configuration changes via environment");
    info!("   ✅ Proper fallback defaults");

    println!();
    Ok(())
}

/// Demonstrate different environment-specific configurations
async fn demonstrate_environment_specific_configs() -> Result<(), Box<dyn std::error::Error>> {
    info!("🌍 Environment-Specific Configuration Examples");
    info!("=============================================");

    // Show current environment
    let current_env = ConfigUtils::get_environment();
    info!("🏷️  Current environment: {}", current_env);

    // Show different environment configurations
    info!("📋 Configuration examples for different environments:");

    println!();
    info!("🛠️  DEVELOPMENT Environment:");
    info!("   TOADSTOOL_ENV=development");
    info!("   TOADSTOOL_DEBUG=true");
    info!("   TOADSTOOL_LOG_LEVEL=debug");
    info!("   TOADSTOOL_BIND_ADDRESS=127.0.0.1");
    info!("   TOADSTOOL_TLS_ENABLED=false");
    info!("   TOADSTOOL_AUTH_ENABLED=false");
    info!("   TOADSTOOL_METRICS_ENABLED=true");

    println!();
    info!("🧪 STAGING Environment:");
    info!("   TOADSTOOL_ENV=staging");
    info!("   TOADSTOOL_DEBUG=false");
    info!("   TOADSTOOL_LOG_LEVEL=info");
    info!("   TOADSTOOL_BIND_ADDRESS=0.0.0.0");
    info!("   TOADSTOOL_TLS_ENABLED=true");
    info!("   TOADSTOOL_AUTH_ENABLED=true");
    info!("   TOADSTOOL_METRICS_ENABLED=true");
    info!("   TOADSTOOL_MAX_CPU_PERCENT=80");

    println!();
    info!("🚀 PRODUCTION Environment:");
    info!("   TOADSTOOL_ENV=production");
    info!("   TOADSTOOL_DEBUG=false");
    info!("   TOADSTOOL_LOG_LEVEL=warn");
    info!("   TOADSTOOL_BIND_ADDRESS=0.0.0.0");
    info!("   TOADSTOOL_TLS_ENABLED=true");
    info!("   TOADSTOOL_AUTH_ENABLED=true");
    info!("   TOADSTOOL_SANDBOXING_ENABLED=true");
    info!("   TOADSTOOL_METRICS_ENABLED=true");
    info!("   TOADSTOOL_MAX_CPU_PERCENT=70");
    info!("   TOADSTOOL_MAX_MEMORY_BYTES=17179869184");
    info!("   TOADSTOOL_WORKER_THREADS=32");

    println!();
    Ok(())
}

/// Demonstrate configuration validation and error handling
async fn demonstrate_configuration_validation() -> Result<(), Box<dyn std::error::Error>> {
    info!("🔍 Configuration Validation & Error Handling");
    info!("===========================================");

    // Test various configuration scenarios
    info!("🧪 Testing configuration validation...");

    // Test valid configuration
    let network_config = NetworkEnvConfig::from_env();
    info!("✅ Network configuration loaded successfully");
    info!("   Songbird port: {}", network_config.songbird_port);
    info!("   BearDog port: {}", network_config.beardog_port);
    info!("   TLS enabled: {}", network_config.tls_enabled);

    // Test resource configuration
    let resource_config = ResourceEnvConfig::from_env();
    info!("✅ Resource configuration loaded successfully");
    info!("   Max CPU: {}%", resource_config.max_cpu_percent);
    info!("   Max Memory: {} bytes", resource_config.max_memory_bytes);
    info!("   Worker threads: {}", resource_config.worker_threads);

    // Test monitoring configuration
    let monitoring_config = MonitoringEnvConfig::from_env();
    info!("✅ Monitoring configuration loaded successfully");
    info!("   Metrics enabled: {}", monitoring_config.metrics_enabled);
    info!("   Log level: {}", monitoring_config.log_level);
    info!(
        "   Health checks enabled: {}",
        monitoring_config.health_checks_enabled
    );

    // Test security configuration
    let security_config = SecurityEnvConfig::from_env();
    info!("✅ Security configuration loaded successfully");
    info!("   Auth enabled: {}", security_config.auth_enabled);
    info!(
        "   Sandboxing enabled: {}",
        security_config.sandboxing_enabled
    );
    info!(
        "   Encryption enabled: {}",
        security_config.encryption_enabled
    );

    // Test comprehensive configuration
    let env_config = EnvironmentConfig::from_env();
    info!("✅ Complete environment configuration loaded successfully");
    info!("   Environment: {}", env_config.environment);
    info!("   Debug mode: {}", env_config.debug);
    info!("   Data directory: {}", env_config.data_dir.display());

    println!();
    Ok(())
}

/// Demonstrate service discovery with configurable endpoints
async fn demonstrate_service_discovery() -> Result<(), Box<dyn std::error::Error>> {
    info!("🔍 Service Discovery with Configurable Endpoints");
    info!("===============================================");

    // Show how services can be discovered using configurable endpoints
    let service_ports = ConfigUtils::get_service_ports();
    let service_endpoints = ConfigUtils::get_service_endpoints();

    info!("🔗 Available services:");
    for (service_name, port) in &service_ports {
        let default_endpoint = format!("http://localhost:{port}");
        let endpoint = service_endpoints
            .get(service_name)
            .unwrap_or(&default_endpoint);
        info!("   📡 {} service:", service_name);
        info!("      Port: {}", port);
        info!("      Endpoint: {}", endpoint);
        info!("      Status: {}", simulate_service_check(endpoint).await);
    }

    // Show port ranges for containers
    let (container_start, container_end) = ConfigUtils::get_container_port_range();
    let (port_start, port_end) = ConfigUtils::get_port_allocation_range();

    info!(
        "🐳 Container port range: {} - {}",
        container_start, container_end
    );
    info!("⚙️  Port allocation range: {} - {}", port_start, port_end);

    println!();
    Ok(())
}

/// Demonstrate resource limit configuration
async fn demonstrate_resource_configuration() -> Result<(), Box<dyn std::error::Error>> {
    info!("💾 Resource Configuration Management");
    info!("===================================");

    // Show resource limits
    let max_cpu = ConfigUtils::get_max_cpu_usage();
    let max_memory = ConfigUtils::get_max_memory_usage();
    let max_storage = ConfigUtils::get_max_storage_usage();
    let worker_threads = ConfigUtils::get_worker_threads();
    let max_concurrent = ConfigUtils::get_max_concurrent_executions();

    info!("📊 Resource limits:");
    info!("   🔢 Max CPU usage: {}%", max_cpu);
    info!(
        "   🧠 Max memory usage: {} bytes ({} GB)",
        max_memory,
        max_memory / (1024 * 1024 * 1024)
    );
    info!(
        "   💾 Max storage usage: {} bytes ({} GB)",
        max_storage,
        max_storage / (1024 * 1024 * 1024)
    );
    info!("   🧵 Worker threads: {}", worker_threads);
    info!("   ⚡ Max concurrent executions: {}", max_concurrent);

    // Show timeout configurations
    let request_timeout = ConfigUtils::get_request_timeout();
    let connection_timeout = ConfigUtils::get_connection_timeout();
    let execution_timeout = ConfigUtils::get_execution_timeout();

    info!("⏱️  Timeout configurations:");
    info!("   🌐 Request timeout: {:?}", request_timeout);
    info!("   🔌 Connection timeout: {:?}", connection_timeout);
    info!("   ⚡ Execution timeout: {:?}", execution_timeout);

    // Show monitoring intervals
    let metrics_interval = ConfigUtils::get_metrics_interval();
    let health_check_interval = ConfigUtils::get_health_check_interval();

    info!("📈 Monitoring intervals:");
    info!("   📊 Metrics collection: {:?}", metrics_interval);
    info!("   🏥 Health check: {:?}", health_check_interval);

    println!();
    Ok(())
}

/// Show all current configuration values
fn demonstrate_configuration_inspection() {
    info!("🔍 Complete Configuration Inspection");
    info!("===================================");

    // Show all TOADSTOOL environment variables
    let env_vars = ConfigUtils::get_all_toadstool_env_vars();
    if !env_vars.is_empty() {
        info!("🌍 Current TOADSTOOL environment variables:");
        for (key, value) in env_vars {
            info!("   {}: {}", key, value);
        }
    } else {
        info!("ℹ️  No TOADSTOOL environment variables set (using defaults)");
    }

    println!();
}

/// Simulate a service health check
async fn simulate_service_check(endpoint: &str) -> &'static str {
    // In a real implementation, this would make an HTTP request
    // For demo purposes, we'll just simulate different statuses
    if endpoint.contains("8080") {
        "🟢 Online"
    } else if endpoint.contains("8081") {
        "🟡 Degraded"
    } else if endpoint.contains("8082") {
        "🔴 Offline"
    } else {
        "⚪ Unknown"
    }
}

/// Demonstrate configuration file loading (bonus feature)
fn demonstrate_configuration_file_loading() {
    info!("📄 Configuration File Loading");
    info!("=============================");

    // Show how environment variables can be combined with config files
    info!("🔄 Loading order (highest to lowest priority):");
    info!("   1. Environment variables (TOADSTOOL_*)");
    info!("   2. Command line arguments");
    info!("   3. Configuration files (.env, toadstool.toml)");
    info!("   4. Default values");

    info!("📝 Example .env file:");
    info!("   TOADSTOOL_ENV=production");
    info!("   TOADSTOOL_DEBUG=false");
    info!("   TOADSTOOL_SONGBIRD_PORT=8080");
    info!("   TOADSTOOL_BEARDOG_PORT=8081");
    info!("   TOADSTOOL_TLS_ENABLED=true");

    info!("📝 Example toadstool.toml file:");
    info!("   [network]");
    info!("   bind_address = \"0.0.0.0\"");
    info!("   tls_enabled = true");
    info!("   [security]");
    info!("   auth_enabled = true");
    info!("   sandboxing_enabled = true");

    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_config_management_demo() {
        temp_env::with_vars(
            [
                ("TOADSTOOL_ENV", Some("test")),
                ("SONGBIRD_PORT", Some("9080")),
                ("TOADSTOOL_DEBUG", Some("true")),
            ],
            || {
                assert_eq!(ConfigUtils::get_environment(), "test");
                #[allow(deprecated)]
                {
                    assert_eq!(ConfigUtils::get_songbird_port(), 9080);
                }
                assert!(ConfigUtils::get_debug_mode());

                // Sovereignty: get_service_endpoints only returns toadstool's own
                let endpoints = ConfigUtils::get_service_endpoints();
                assert!(
                    endpoints.contains_key("toadstool"),
                    "should contain toadstool (self-knowledge)"
                );
            },
        );
    }
}
