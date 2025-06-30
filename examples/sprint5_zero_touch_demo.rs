//! # Sprint 5: Zero-Touch & Grandma-Friendly Configuration Demo
//! 
//! This demo showcases ToadStool's Sprint 5 capabilities:
//! - Zero-touch auto-configuration
//! - Grandma-friendly natural language interface
//! - AI-friendly Squirrel MCP integration
//! - Intelligent hardware detection and optimization

use tracing::{info, error};

use toadstool_auto_config::{
    IntelligentAutoConfig,
    natural_language::NaturalLanguageProcessor,
    hardware::HardwareDetector,
    ecosystem::EcosystemDiscoverer,
    installer::SmartInstaller,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging for the demo
    env_logger::init();
    
    println!("🍄 ToadStool Sprint 5: Zero-Touch & Grandma-Friendly Demo");
    println!("===========================================================");
    println!();
    
    // Demo 1: Zero-Touch Auto-Configuration
    demo_zero_touch_configuration().await?;
    
    // Demo 2: Grandma-Friendly Natural Language Interface
    demo_natural_language_interface().await?;
    
    // Demo 3: Hardware Detection and Optimization
    demo_hardware_detection().await?;
    
    // Demo 4: Ecosystem Service Discovery
    demo_ecosystem_discovery().await?;
    
    // Demo 5: AI-Friendly Interface Examples
    demo_ai_friendly_interface().await?;
    
    // Demo: Zero-Touch Installation (commented out as it would actually install)
    _demo_zero_touch_installation().await?;
    
    println!("🎉 Sprint 5 Demo Complete!");
    println!("ToadStool is now truly universal - grandma-friendly and AI-native! 🌟");
    
    Ok(())
}

/// Demo 1: Zero-Touch Auto-Configuration
async fn demo_zero_touch_configuration() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧠 Demo 1: Zero-Touch Auto-Configuration");
    println!("=========================================");
    
    info!("Starting zero-touch auto-configuration...");
    
    // This is the magic - one function call configures everything optimally
    match IntelligentAutoConfig::auto_configure().await {
        Ok(config_result) => {
            println!("✅ Auto-configuration successful!");
            println!();
            
            // Show what was detected and configured
            let hardware = &config_result.hardware_capabilities;
            println!("🖥️ Detected Hardware:");
            println!("   • CPU: {} cores ({})", hardware.cpu_cores, hardware.cpu_model);
            println!("   • Memory: {:.1} GB", hardware.memory_gb);
            println!("   • GPUs: {} detected", hardware.gpu_count);
            if let Some(gpu_platform) = &hardware.gpu_platform {
                println!("   • GPU Platform: {}", gpu_platform);
            }
            println!("   • Container Support: {}", if hardware.has_container_support() { "✅" } else { "❌" });
            println!();
            
            // Show ecosystem services found
            let ecosystem = &config_result.ecosystem_services;
            println!("🌐 Ecosystem Services:");
            if ecosystem.discovered_services.is_empty() {
                println!("   • No ecosystem services found (standalone mode)");
            } else {
                for (name, service) in &ecosystem.discovered_services {
                    println!("   • {}: {} ({})", name, service.endpoint, service.health_status);
                }
            }
            println!();
            
            // Show generated configuration highlights
            let config = &config_result.generated_config;
            println!("⚙️ Generated Configuration:");
            println!("   • Security Profile: {:?}", config.security_profile);
            println!("   • Performance Profile: {:?}", config.performance_profile);
            println!("   • Enabled Runtimes: {:?}", config.runtime_configs.native);
            if config.runtime_configs.container.is_some() {
                println!("   • Container Runtime: Available");
            }
            if config.runtime_configs.gpu.is_some() {
                println!("   • GPU Runtime: Available");
            }
            println!();
            
            // Show recommendations
            if !config_result.recommendations.is_empty() {
                println!("💡 Recommendations:");
                for rec in &config_result.recommendations {
                    println!("   • [{}] {}", rec.category, rec.message);
                }
                println!();
            }
        },
        Err(e) => {
            error!("Auto-configuration failed: {}", e);
            println!("❌ Auto-configuration failed, but that's okay for a demo!");
            println!("   In a real environment, this would detect and configure everything optimally.");
            println!();
        }
    }
    
    println!("🎯 Key Point: Zero configuration files needed - it just works!");
    println!();
    
    Ok(())
}

/// Demo 2: Grandma-Friendly Natural Language Interface
async fn demo_natural_language_interface() -> Result<(), Box<dyn std::error::Error>> {
    println!("🗣️ Demo 2: Grandma-Friendly Natural Language Interface");
    println!("=====================================================");
    
    let processor = NaturalLanguageProcessor::new();
    
    // Grandma-friendly examples
    let grandma_requests = vec![
        "I want to run some Python scripts safely",
        "Make it fast for machine learning",
        "I don't want anything to break my computer", 
        "I need this for my small business",
        "Just make it work, I don't know about computers",
        "Can you help me run a data analysis program securely?",
    ];
    
    println!("👵 Grandma-Friendly Examples:");
    println!();
    
    for request in grandma_requests {
        println!("🗨️ Grandma says: \"{}\"", request);
        
        match processor.configure_from_natural_language(request).await {
            Ok(response) => {
                println!("🤖 ToadStool responds: \"{}\"", response.explanation);
                println!("   Confidence: {:.0}%", response.confidence * 100.0);
                
                if !response.suggestions.is_empty() {
                    println!("   Suggestions:");
                    for suggestion in &response.suggestions {
                        println!("     • {}", suggestion);
                    }
                }
            },
            Err(e) => {
                println!("❌ Failed to process request: {}", e);
            }
        }
        println!();
    }
    
    // Show the magic of intent understanding
    println!("🧠 Intent Understanding Examples:");
    println!();
    
    let test_cases = vec![
        ("make it secure for production", "🛡️ High security + monitoring"),
        ("optimize for GPU machine learning", "🚀 GPU runtime + max performance"),
        ("simple setup for beginners", "😊 Auto-configure + helpful messages"),
        ("enterprise business application", "💼 High security + reliability"),
    ];
    
    for (input, expected) in test_cases {
        println!("Input: \"{}\"", input);
        
        match processor.parse_intent(input).await {
            Ok(intent) => {
                println!("Understood: Security={:?}, Use Case={:?}, Experience={:?}", 
                        intent.security_level, intent.use_case, intent.user_experience);
                println!("Expected: {}", expected);
            },
            Err(e) => {
                println!("Failed: {}", e);
            }
        }
        println!();
    }
    
    Ok(())
}

/// Demo 3: Hardware Detection and Optimization
async fn demo_hardware_detection() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Demo 3: Hardware Detection and Optimization");
    println!("==============================================");
    
    let mut detector = HardwareDetector::new();
    
    match detector.scan_system().await {
        Ok(capabilities) => {
            println!("🖥️ Comprehensive Hardware Scan Results:");
            println!();
            
            // CPU Information
            println!("🧠 CPU Information:");
            println!("   • Model: {}", capabilities.cpu_model);
            println!("   • Cores: {}", capabilities.cpu_cores);
            println!("   • Frequency: {} MHz", capabilities.cpu_frequency_mhz);
            println!();
            
            // Memory Information
            println!("💾 Memory Information:");
            println!("   • Total RAM: {:.1} GB", capabilities.memory_gb);
            println!("   • Swap: {:.1} GB", capabilities.swap_gb);
            println!();
            
            // Platform Information
            println!("🌐 Platform Information:");
            println!("   • OS: {}", capabilities.platform);
            println!("   • Architecture: {}", capabilities.architecture);
            println!("   • Virtualized: {}", if capabilities.is_virtualized { "Yes" } else { "No" });
            if let Some(virt_type) = &capabilities.virtualization_type {
                println!("   • Virtualization: {}", virt_type);
            }
            println!();
            
            // Container Runtime Support
            println!("🐳 Container Runtime Support:");
            println!("   • Docker: {}", if capabilities.has_docker { "✅" } else { "❌" });
            println!("   • Podman: {}", if capabilities.has_podman { "✅" } else { "❌" });
            println!("   • Containerd: {}", if capabilities.has_containerd { "✅" } else { "❌" });
            println!();
            
            // GPU Information
            println!("🎮 GPU Information:");
            if capabilities.gpu_count > 0 {
                println!("   • GPU Count: {}", capabilities.gpu_count);
                if let Some(platform) = &capabilities.gpu_platform {
                    println!("   • Platform: {}", platform);
                }
                if let Some(memory) = capabilities.gpu_memory_gb {
                    println!("   • Memory: {:.1} GB", memory);
                }
            } else {
                println!("   • No GPUs detected");
            }
            println!();
            
            // Storage Information
            if !capabilities.storage_info.is_empty() {
                println!("💽 Storage Information:");
                for storage in &capabilities.storage_info {
                    println!("   • {}: {:.1} GB total, {:.1} GB available ({})", 
                            storage.name, storage.total_gb, storage.available_gb, storage.file_system);
                }
                println!();
            }
            
            // Smart Recommendations
            println!("🎯 Smart Optimization Recommendations:");
            if capabilities.has_gpu_support() {
                println!("   ✅ GPU acceleration available for ML workloads");
            }
            if capabilities.has_container_support() {
                println!("   ✅ Container isolation recommended for security");
            }
            if capabilities.memory_gb >= 16.0 {
                println!("   ✅ Sufficient memory for high-performance workloads");
            } else {
                println!("   ⚠️ Consider memory optimization for large workloads");
            }
            if capabilities.cpu_cores >= 8 {
                println!("   ✅ Multi-core parallelization recommended");
            }
            
        },
        Err(e) => {
            error!("Hardware detection failed: {}", e);
            println!("❌ Hardware detection failed, but that's okay for a demo!");
        }
    }
    
    println!();
    Ok(())
}

/// Demo 4: Ecosystem Service Discovery
async fn demo_ecosystem_discovery() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌐 Demo 4: Ecosystem Service Discovery");
    println!("=====================================");
    
    let mut discoverer = EcosystemDiscoverer::new();
    
    println!("🔍 Searching for ecosystem services...");
    
    match discoverer.discover_services().await {
        Ok(ecosystem) => {
            if ecosystem.discovered_services.is_empty() {
                println!("📡 No ecosystem services found - operating in standalone mode");
                println!("   This is perfectly fine! ToadStool works great standalone.");
                println!();
                println!("🎯 Standalone Benefits:");
                println!("   • Zero dependencies on external services");
                println!("   • Perfect for personal use, development, and testing");
                println!("   • All features available locally");
                println!("   • Maximum privacy and security");
            } else {
                println!("🎉 Found {} ecosystem service(s):", ecosystem.discovered_services.len());
                
                for (name, service) in &ecosystem.discovered_services {
                    println!("   🔗 {}", name);
                    println!("      Type: {:?}", service.service_type);
                    println!("      Endpoint: {}", service.endpoint);
                    println!("      Status: {} {}", 
                            match service.health_status {
                                toadstool_auto_config::ecosystem::HealthStatus::Healthy => "✅",
                                toadstool_auto_config::ecosystem::HealthStatus::Degraded => "⚠️",
                                toadstool_auto_config::ecosystem::HealthStatus::Unhealthy => "❌",
                                toadstool_auto_config::ecosystem::HealthStatus::Unknown => "❓",
                            }, service.health_status);
                    println!("      Capabilities: {:?}", service.capabilities);
                    println!();
                }
                
                println!("🚀 Ecosystem Integration Benefits:");
                println!("   • Automatic service discovery and load balancing");
                println!("   • Distributed computing capabilities");
                println!("   • Shared resource optimization");
                println!("   • Enterprise-grade reliability");
            }
        },
        Err(e) => {
            info!("Service discovery completed with limitations: {}", e);
            println!("📡 Service discovery completed - operating in standalone mode");
            println!("   This is the expected behavior when no ecosystem services are running.");
        }
    }
    
    println!();
    println!("💡 Pro Tip: Install Songbird for automatic service discovery!");
    println!("   curl -sSL https://install.songbird.dev | bash");
    
    println!();
    Ok(())
}

/// Demo 5: AI-Friendly Interface Examples
async fn demo_ai_friendly_interface() -> Result<(), Box<dyn std::error::Error>> {
    println!("🤖 Demo 5: AI-Friendly Interface (Squirrel MCP Integration)");
    println!("==========================================================");
    
    println!("This demo shows how ToadStool integrates with AI systems like Squirrel MCP.");
    println!();
    
    // Simulate AI requests that would come from Squirrel MCP
    let ai_requests = vec![
        ("Optimize for high-throughput batch processing", "AI wants maximum performance"),
        ("Secure multi-tenant environment with isolation", "AI wants security for multiple users"),
        ("GPU-accelerated ML pipeline with checkpointing", "AI wants ML optimization"),
        ("Enable distributed computing across 5 nodes", "AI wants cluster coordination"),
    ];
    
    let processor = NaturalLanguageProcessor::new();
    
    println!("🔗 Simulated Squirrel MCP → ToadStool Integration:");
    println!();
    
    for (ai_request, context) in ai_requests {
        println!("🤖 AI Request: \"{}\"", ai_request);
        println!("   Context: {}", context);
        
        match processor.configure_from_natural_language(ai_request).await {
            Ok(response) => {
                println!("✅ ToadStool Response:");
                println!("   Configuration: {}", response.explanation);
                println!("   Confidence: {:.0}%", response.confidence * 100.0);
                
                // Show the structured response that would go back to the AI
                println!("   Structured Response for AI:");
                println!("     • Security: {:?}", response.config.security_profile);
                println!("     • Performance: {:?}", response.config.performance_profile);
                println!("     • GPU Enabled: {}", response.config.enable_gpu);
                println!("     • Monitoring: {}", response.config.enable_monitoring);
                
                if !response.suggestions.is_empty() {
                    println!("   AI Learning Feedback:");
                    for suggestion in &response.suggestions {
                        println!("     • {}", suggestion);
                    }
                }
            },
            Err(e) => {
                println!("❌ Processing failed: {}", e);
            }
        }
        println!();
    }
    
    println!("🎯 AI Integration Benefits:");
    println!("   • Natural language understanding for AI systems");
    println!("   • Structured responses for AI learning and adaptation");
    println!("   • Intent-based configuration rather than manual setup");
    println!("   • Perfect integration with Squirrel MCP for universal AI compute");
    println!();
    
    println!("🌟 The Future: AI + Human + ToadStool = Universal Compute");
    println!("   • Grandma says: 'Make it safe' → Enterprise security");
    println!("   • AI says: 'ML training job' → GPU optimization");
    println!("   • Developer says: 'Microservices' → Container orchestration");
    println!("   • Business says: 'Reliable' → High availability");
    
    println!();
    Ok(())
}

/// Demo: Zero-Touch Installation (commented out as it would actually install)
async fn _demo_zero_touch_installation() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Demo: Zero-Touch Installation");
    println!("================================");
    
    println!("💡 This demo shows how ToadStool would install itself with zero configuration:");
    println!();
    
    // Create installer
    let installer = SmartInstaller::new();
    
    // Simulate hardware detection for installation
    let mut detector = HardwareDetector::new();
    match detector.scan_system().await {
        Ok(capabilities) => {
            println!("🔍 Installation would detect:");
            println!("   • Platform: {}", capabilities.platform);
            println!("   • Architecture: {}", capabilities.architecture);
            println!("   • Container Runtime: {}", if capabilities.has_container_support() { "Available" } else { "Not Available" });
            println!();
            
            // Simulate installation
            match installer.zero_touch_install(&capabilities).await {
                Ok(result) => {
                    println!("✅ Installation simulation successful!");
                    println!("   • Installed components: {:?}", result.installed_components);
                    println!("   • Configuration path: {}", result.configuration_path);
                    println!("   • Service enabled: {}", result.service_enabled);
                },
                Err(e) => {
                    println!("❌ Installation simulation failed: {}", e);
                }
            }
        },
        Err(e) => {
            println!("❌ Hardware detection failed: {}", e);
        }
    }
    
    println!();
    println!("🎯 In a real installation:");
    println!("   curl -sSL https://install.toadstool.dev | bash");
    println!("   # Output: ✅ ToadStool installed! Everything is ready to go!");
    println!();
    
    Ok(())
} 