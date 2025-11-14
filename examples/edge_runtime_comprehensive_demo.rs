//! # ToadStool Edge Runtime Comprehensive Demo
//!
//! This example demonstrates the complete edge runtime capabilities including:
//! - Automatic device discovery (Arduino, ESP32, etc.)
//! - Cross-platform deployment and execution
//! - Device management and monitoring
//! - Cross-compilation toolchain
//! - Communication protocols
//! - Resource management for edge devices
//! - Real-time sensor data collection
//! - Actuator control

use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};
use uuid::Uuid;

use toadstool::{
    config::ToadStoolConfig,
    execution::{ExecutionInput, ExecutionRequest, RuntimeEngine, RuntimeType},
    init,
    resources::ResourceRequirements,
    security::{IsolationLevel, SecurityContext},
    workload::WorkloadSpec,
};

use toadstool_runtime_edge::{
    discovery::DeviceDiscoveryService,
    platforms::{
        ArduinoBoard, ArduinoDevice, AuthenticationInfo, AuthenticationMethod, ConnectionInfo,
        ConnectionType, DeviceStatus, ESP32Device, ESP32Framework, ESP32Variant, EdgeDevice,
        EdgePlatform,
    },
    toolchain::CrossCompilationToolchain,
    EdgeRuntimeConfig, EdgeRuntimeEngine, EdgeSecurityLevel, ResourceAllocationStrategy,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("🍄 Starting ToadStool Edge Runtime Comprehensive Demo");

    // Initialize ToadStool
    let mut config = ToadStoolConfig::default();
    let toadstool = init(config).await?;

    // Create edge runtime configuration
    let edge_config = EdgeRuntimeConfig {
        discovery_enabled: true,
        discovery_timeout_secs: 30,
        max_devices: 50,
        communication_timeout_ms: 5000,
        cross_compile_cache_path: "/tmp/toadstool_edge_cache".to_string(),
        auto_provisioning: true,
        security_level: EdgeSecurityLevel::Standard,
        resource_strategy: ResourceAllocationStrategy::Adaptive,
    };

    // Create edge runtime engine
    let edge_runtime = EdgeRuntimeEngine::new(edge_config.clone()).await?;

    // Phase 1: Device Discovery
    demo_device_discovery(&edge_runtime).await?;

    // Phase 2: Arduino Platform Demo
    demo_arduino_platform(&edge_runtime).await?;

    // Phase 3: ESP32 Platform Demo
    demo_esp32_platform(&edge_runtime).await?;

    // Phase 4: Cross-Compilation Demo
    demo_cross_compilation(&edge_config).await?;

    // Phase 5: Multi-Device Orchestration
    demo_multi_device_orchestration(&edge_runtime).await?;

    // Phase 6: Edge Computing Scenarios
    demo_edge_computing_scenarios(&edge_runtime).await?;

    // Phase 7: Performance and Monitoring
    demo_performance_monitoring(&edge_runtime).await?;

    // Cleanup
    edge_runtime.cleanup().await?;

    info!("🎉 ToadStool Edge Runtime Demo completed successfully!");
    Ok(())
}

/// Demonstrate device discovery capabilities
async fn demo_device_discovery(
    edge_runtime: &EdgeRuntimeEngine,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("🔍 Phase 1: Device Discovery Demo");

    // Get discovered devices
    let devices = edge_runtime.get_connected_devices().await;
    info!("Found {} edge devices", devices.len());

    for device in &devices {
        info!("  📱 Device: {} ({})", device.name, device.id);
        info!("     Platform: {:?}", device.platform);
        info!("     Status: {:?}", device.status);
        info!(
            "     Resources: {} MB RAM, {} cores",
            device.resources.memory_bytes / 1024 / 1024,
            device.resources.cpu_cores
        );
        info!("     Capabilities: {:?}", device.capabilities);
    }

    // Demonstrate discovery methods
    info!("🔍 Testing different discovery methods...");

    // Mock discovery for demonstration
    let mock_arduino = create_mock_arduino_device();
    let mock_esp32 = create_mock_esp32_device();

    info!("  ✅ Mock Arduino Uno discovered");
    info!("  ✅ Mock ESP32 discovered");

    sleep(Duration::from_secs(2)).await;
    Ok(())
}

/// Demonstrate Arduino platform capabilities
async fn demo_arduino_platform(
    edge_runtime: &EdgeRuntimeEngine,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("🤖 Phase 2: Arduino Platform Demo");

    // Create mock Arduino device for demo
    let arduino_device = create_mock_arduino_device();

    info!("📡 Connecting to Arduino Uno...");

    // Simulate Arduino sketches
    let led_blink_sketch = r#"
        // ToadStool Arduino LED Blink Example
        void setup() {
            pinMode(13, OUTPUT);
            Serial.begin(9600);
            Serial.println("ToadStool Arduino Demo Started");
        }
        
        void loop() {
            digitalWrite(13, HIGH);
            delay(1000);
            digitalWrite(13, LOW);
            delay(1000);
            Serial.println("LED blinked");
        }
    "#;

    let sensor_reading_sketch = r#"
        // ToadStool Arduino Sensor Reading Example
        void setup() {
            Serial.begin(9600);
            Serial.println("ToadStool Sensor Demo Started");
        }
        
        void loop() {
            int temp = analogRead(A0);
            int light = analogRead(A1);
            
            Serial.print("Temperature: ");
            Serial.print(temp);
            Serial.print(", Light: ");
            Serial.println(light);
            
            delay(5000);
        }
    "#;

    // Create execution requests
    let led_request = ExecutionRequest {
        id: Uuid::new_v4(),
        code: led_blink_sketch.as_bytes().to_vec(),
        language: "arduino".to_string(),
        args: vec![],
        env: HashMap::new(),
        security_context: SecurityContext {
            isolation_level: IsolationLevel::Process,
            network_access: false,
            filesystem_access: false,
            resource_limits: ResourceRequirements {
                max_memory_mb: 2,
                max_cpu_percent: 100.0,
                max_execution_time_ms: 60000,
                max_storage_mb: 32,
            },
        },
        priority: 1,
        timeout_ms: 30000,
        resource_requirements: ResourceRequirements {
            max_memory_mb: 2,
            max_cpu_percent: 100.0,
            max_execution_time_ms: 60000,
            max_storage_mb: 32,
        },
    };

    let sensor_request = ExecutionRequest {
        id: Uuid::new_v4(),
        code: sensor_reading_sketch.as_bytes().to_vec(),
        language: "arduino".to_string(),
        args: vec![],
        env: HashMap::new(),
        security_context: SecurityContext {
            isolation_level: IsolationLevel::Process,
            network_access: false,
            filesystem_access: false,
            resource_limits: ResourceRequirements {
                max_memory_mb: 2,
                max_cpu_percent: 100.0,
                max_execution_time_ms: 60000,
                max_storage_mb: 32,
            },
        },
        priority: 1,
        timeout_ms: 30000,
        resource_requirements: ResourceRequirements {
            max_memory_mb: 2,
            max_cpu_percent: 100.0,
            max_execution_time_ms: 60000,
            max_storage_mb: 32,
        },
    };

    // Execute LED blink sketch
    info!("💡 Deploying LED blink sketch...");
    let response = edge_runtime.execute(led_request).await?;
    info!("✅ LED blink sketch deployed successfully");

    sleep(Duration::from_secs(2)).await;

    // Execute sensor reading sketch
    info!("🌡️ Deploying sensor reading sketch...");
    let response = edge_runtime.execute(sensor_request).await?;
    info!("✅ Sensor reading sketch deployed successfully");

    // Simulate sensor data collection
    info!("📊 Collecting sensor data...");
    for i in 0..5 {
        sleep(Duration::from_secs(1)).await;
        let temp = 20.0 + (i as f64 * 2.5);
        let light = 512 + (i * 100);
        info!(
            "  📈 Reading {}: Temperature: {:.1}°C, Light: {}",
            i + 1,
            temp,
            light
        );
    }

    sleep(Duration::from_secs(1)).await;
    Ok(())
}

/// Demonstrate ESP32 platform capabilities
async fn demo_esp32_platform(
    edge_runtime: &EdgeRuntimeEngine,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("🛰️ Phase 3: ESP32 Platform Demo");

    // Create mock ESP32 device for demo
    let esp32_device = create_mock_esp32_device();

    info!("📡 Connecting to ESP32...");

    // Simulate ESP32 firmware
    let wifi_sensor_firmware = r#"
        // ToadStool ESP32 WiFi Sensor Example
        #include <WiFi.h>
        #include <WebServer.h>
        
        const char* ssid = "ToadStool_Network";
        const char* password = "mushroom123";
        
        WebServer server(80);
        
        void setup() {
            Serial.begin(115200);
            Serial.println("ToadStool ESP32 Demo Started");
            
            // Connect to WiFi
            WiFi.begin(ssid, password);
            while (WiFi.status() != WL_CONNECTED) {
                delay(1000);
                Serial.println("Connecting to WiFi...");
            }
            
            Serial.println("WiFi connected!");
            Serial.print("IP address: ");
            Serial.println(WiFi.localIP());
            
            // Setup web server
            server.on("/", handleRoot);
            server.on("/sensors", handleSensors);
            server.begin();
        }
        
        void loop() {
            server.handleClient();
            delay(100);
        }
        
        void handleRoot() {
            server.send(200, "text/html", "<h1>ToadStool ESP32 Edge Device</h1>");
        }
        
        void handleSensors() {
            float temp = random(200, 350) / 10.0;
            float humidity = random(300, 800) / 10.0;
            
            String json = "{\"temperature\":" + String(temp) + 
                         ",\"humidity\":" + String(humidity) + 
                         ",\"timestamp\":" + String(millis()) + "}";
            
            server.send(200, "application/json", json);
        }
    "#;

    // Create execution request
    let esp32_request = ExecutionRequest {
        id: Uuid::new_v4(),
        code: wifi_sensor_firmware.as_bytes().to_vec(),
        language: "c++".to_string(),
        args: vec![],
        env: HashMap::new(),
        security_context: SecurityContext {
            isolation_level: IsolationLevel::Process,
            network_access: true,
            filesystem_access: true,
            resource_limits: ResourceRequirements {
                max_memory_mb: 520,
                max_cpu_percent: 100.0,
                max_execution_time_ms: 120000,
                max_storage_mb: 4,
            },
        },
        priority: 1,
        timeout_ms: 60000,
        resource_requirements: ResourceRequirements {
            max_memory_mb: 520,
            max_cpu_percent: 100.0,
            max_execution_time_ms: 120000,
            max_storage_mb: 4,
        },
    };

    // Execute ESP32 firmware
    info!("🔥 Flashing ESP32 firmware...");
    let response = edge_runtime.execute(esp32_request).await?;
    info!("✅ ESP32 firmware flashed successfully");

    // Simulate WiFi connection and sensor data
    info!("📶 Connecting to WiFi...");
    sleep(Duration::from_secs(2)).await;
    info!("✅ WiFi connected! IP: 192.168.1.100");

    info!("🌐 Starting web server...");
    sleep(Duration::from_secs(1)).await;

    info!("📡 Simulating sensor data over WiFi...");
    for i in 0..5 {
        sleep(Duration::from_secs(1)).await;
        let temp = 22.0 + (i as f64 * 1.5);
        let humidity = 45.0 + (i as f64 * 3.0);
        info!(
            "  🌡️ WiFi Sensor Data {}: Temperature: {:.1}°C, Humidity: {:.1}%",
            i + 1,
            temp,
            humidity
        );
    }

    sleep(Duration::from_secs(1)).await;
    Ok(())
}

/// Demonstrate cross-compilation capabilities
async fn demo_cross_compilation(
    edge_config: &EdgeRuntimeConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("🔧 Phase 4: Cross-Compilation Demo");

    // Create cross-compilation toolchain
    let toolchain = CrossCompilationToolchain::new(edge_config).await?;

    info!("🛠️ Available toolchains:");
    let available_toolchains = toolchain.get_available_toolchains().await;
    for tc in &available_toolchains {
        info!("  ⚙️ {} ({})", tc.name, tc.target);
    }

    // Sample C code for cross-compilation
    let sample_c_code = r#"
        #include <stdio.h>
        #include <stdlib.h>
        
        int main() {
            printf("Hello from ToadStool Edge Device!\n");
            printf("Compiled for cross-platform execution\n");
            return 0;
        }
    "#;

    // Demonstrate compilation for different targets
    let targets = vec![
        (
            EdgePlatform::Arduino {
                board: ArduinoBoard::Uno,
                version: "1.0".to_string(),
            },
            "Arduino Uno",
        ),
        (
            EdgePlatform::ESP32 {
                chip: ESP32Variant::ESP32,
                framework: ESP32Framework::ESPIDF,
            },
            "ESP32",
        ),
        (
            EdgePlatform::RaspberryPi {
                model: toadstool_runtime_edge::platforms::PiModel::Pi4,
                os: toadstool_runtime_edge::platforms::PiOS::RaspberryPiOS,
            },
            "Raspberry Pi 4",
        ),
    ];

    for (platform, name) in targets {
        info!("🎯 Cross-compiling for {}...", name);

        match toolchain
            .cross_compile(sample_c_code.as_bytes(), &platform)
            .await
        {
            Ok(binary) => {
                info!(
                    "  ✅ Successfully compiled {} bytes for {}",
                    binary.len(),
                    name
                );
            }
            Err(e) => {
                warn!("  ⚠️ Compilation failed for {}: {}", name, e);
            }
        }

        sleep(Duration::from_millis(500)).await;
    }

    // Show cache statistics
    let cache_stats = toolchain.get_cache_stats().await;
    info!("📊 Compilation cache statistics:");
    for (key, value) in cache_stats {
        info!("  📈 {}: {}", key, value);
    }

    sleep(Duration::from_secs(1)).await;
    Ok(())
}

/// Demonstrate multi-device orchestration
async fn demo_multi_device_orchestration(
    edge_runtime: &EdgeRuntimeEngine,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("🌐 Phase 5: Multi-Device Orchestration Demo");

    // Simulate multiple devices
    let devices = vec![
        ("Arduino Uno", "Temperature Sensor"),
        ("ESP32", "WiFi Gateway"),
        ("Raspberry Pi", "Edge Analytics"),
    ];

    info!("🔗 Orchestrating across {} devices...", devices.len());

    // Simulate distributed workload
    for (i, (device, task)) in devices.iter().enumerate() {
        info!("  📤 Deploying {} to {}...", task, device);

        // Simulate deployment time
        sleep(Duration::from_millis(800)).await;

        info!("  ✅ {} deployed successfully on {}", task, device);

        // Simulate task execution
        sleep(Duration::from_millis(500)).await;

        let status = match i % 3 {
            0 => "Collecting data",
            1 => "Processing data",
            2 => "Analyzing results",
            _ => "Running",
        };

        info!("  🔄 {} status: {}", device, status);
    }

    // Simulate data flow between devices
    info!("🔄 Data flow simulation:");
    info!("  Arduino → ESP32: Temperature readings");
    sleep(Duration::from_secs(1)).await;
    info!("  ESP32 → Raspberry Pi: Aggregated sensor data");
    sleep(Duration::from_secs(1)).await;
    info!("  Raspberry Pi → Cloud: Analytics results");
    sleep(Duration::from_secs(1)).await;

    info!("✅ Multi-device orchestration completed successfully");

    sleep(Duration::from_secs(1)).await;
    Ok(())
}

/// Demonstrate edge computing scenarios
async fn demo_edge_computing_scenarios(
    edge_runtime: &EdgeRuntimeEngine,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("🏭 Phase 6: Edge Computing Scenarios Demo");

    // Scenario 1: Industrial IoT
    info!("🏭 Scenario 1: Industrial IoT Monitoring");
    let industrial_tasks = vec![
        "PLC Data Collection",
        "Vibration Analysis",
        "Predictive Maintenance",
        "Quality Control",
    ];

    for task in industrial_tasks {
        info!("  🔧 Starting: {}", task);
        sleep(Duration::from_millis(600)).await;
        info!("  ✅ Completed: {}", task);
    }

    // Scenario 2: Smart Agriculture
    info!("🌱 Scenario 2: Smart Agriculture");
    let agriculture_tasks = vec![
        "Soil Moisture Monitoring",
        "Weather Station Data",
        "Irrigation Control",
        "Crop Health Analysis",
    ];

    for task in agriculture_tasks {
        info!("  🌾 Starting: {}", task);
        sleep(Duration::from_millis(600)).await;
        info!("  ✅ Completed: {}", task);
    }

    // Scenario 3: Smart City
    info!("🏙️ Scenario 3: Smart City Infrastructure");
    let smart_city_tasks = vec![
        "Traffic Light Control",
        "Air Quality Monitoring",
        "Street Light Management",
        "Waste Management",
    ];

    for task in smart_city_tasks {
        info!("  🚦 Starting: {}", task);
        sleep(Duration::from_millis(600)).await;
        info!("  ✅ Completed: {}", task);
    }

    sleep(Duration::from_secs(1)).await;
    Ok(())
}

/// Demonstrate performance and monitoring
async fn demo_performance_monitoring(
    edge_runtime: &EdgeRuntimeEngine,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("📊 Phase 7: Performance and Monitoring Demo");

    // Get runtime capabilities
    let capabilities = edge_runtime.get_capabilities().await?;
    info!("🎯 Edge Runtime Capabilities:");
    for capability in &capabilities {
        info!("  ✨ {}", capability);
    }

    // Get resource usage
    let resource_usage = edge_runtime.get_resource_usage().await?;
    info!("📈 Resource Usage:");
    for (metric, value) in &resource_usage {
        info!("  📊 {}: {:.2}", metric, value);
    }

    // Simulate performance monitoring
    info!("🔍 Performance monitoring over time:");
    for i in 0..10 {
        sleep(Duration::from_millis(500)).await;

        let cpu_usage = 20.0 + (i as f64 * 3.0) + (rand::random::<f64>() * 10.0);
        let memory_usage = 15.0 + (i as f64 * 2.0) + (rand::random::<f64>() * 5.0);
        let network_usage = 5.0 + (i as f64 * 1.5) + (rand::random::<f64>() * 3.0);

        info!(
            "  📊 Sample {}: CPU: {:.1}%, Memory: {:.1}MB, Network: {:.1}KB/s",
            i + 1,
            cpu_usage,
            memory_usage,
            network_usage
        );
    }

    info!("✅ Performance monitoring demonstration completed");

    sleep(Duration::from_secs(1)).await;
    Ok(())
}

/// Create a mock Arduino device for demonstration
fn create_mock_arduino_device() -> ArduinoDevice {
    ArduinoDevice::new(
        ArduinoBoard::Uno,
        "1.8.19".to_string(),
        "/dev/ttyUSB0".to_string(),
        9600,
    )
    .unwrap()
}

/// Create a mock ESP32 device for demonstration  
fn create_mock_esp32_device() -> ESP32Device {
    ESP32Device::new(
        ESP32Variant::ESP32,
        ESP32Framework::ESPIDF,
        ConnectionInfo {
            connection_type: ConnectionType::Serial,
            address: "/dev/ttyUSB1".to_string(),
            port: None,
            protocol: "Serial".to_string(),
            authentication: Some(AuthenticationInfo {
                method: AuthenticationMethod::None,
                username: None,
                key_path: None,
                certificate_path: None,
            }),
            encryption: None,
        },
    )
    .unwrap()
}

/// Add a simple random number generator for demo purposes
mod rand {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn random<T: From<f64>>() -> T {
        let mut hasher = DefaultHasher::new();
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .hash(&mut hasher);

        let hash = hasher.finish();
        let normalized = (hash as f64) / (u64::MAX as f64);
        T::from(normalized)
    }
}
