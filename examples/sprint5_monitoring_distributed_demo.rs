// ToadStool - Universal Compute Platform
// Copyright (C) 2025 ToadStool Development Team
//
// Sprint 5 Advanced Monitoring & Distributed Computing Demonstration
//
// This example demonstrates:
// - Advanced monitoring and analytics with predictive capabilities
// - Distributed computing coordination and load balancing
// - Real-time API endpoints and WebSocket communication
// - Comprehensive dashboard and CLI interface
// - Multi-node cluster orchestration

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tokio::time::sleep;
use tracing::{info, warn, error};
use uuid::Uuid;

// Import ToadStool core
use toadstool::execution::RuntimeType;
use toadstool::resources::{MemoryMetrics, CpuMetrics, NetworkMetrics, StorageMetrics, RuntimeMetrics};

// Import Sprint 5 components
use toadstool_management_analytics::{
    IntelligentAnalyticsEngine, AnalyticsConfig, AnalyticsDataPoint, Dashboard,
    DashboardPanel, DashboardLayout, DashboardPermissions, PanelType, TimeRange, PanelPosition
};
use toadstool_distributed::{
    IntelligentDistributedCoordinator, DistributedConfig, DiscoveryConfig, DiscoveryMethod,
    LoadBalancingConfig, LoadBalancingStrategy, HealthCheckConfig, CircuitBreakerConfig,
    AutoScalingConfig, FaultToleranceConfig, DistributedExecutionRequest, ExecutionConstraints,
    ResourceRequirements, LocalityConstraints, ExecutionPriority
};
use toadstool_api::{
    AdvancedApiServer, ApiConfig, ApiEvent, ExecutionRequest
};

/// Comprehensive Sprint 5 demonstration
struct Sprint5Demo {
    analytics_engine: Arc<IntelligentAnalyticsEngine>,
    distributed_coordinator: Arc<IntelligentDistributedCoordinator>,
    api_server: AdvancedApiServer,
}

impl Sprint5Demo {
    /// Initialize Sprint 5 demo with all components
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        info!("🚀 Initializing Sprint 5 Advanced Monitoring & Distributed Computing Demo");
        
        // Initialize analytics engine with default config
        let analytics_config = AnalyticsConfig::default();
        let analytics_engine = Arc::new(IntelligentAnalyticsEngine::new(analytics_config).await?);
        
        // Initialize distributed coordinator with default config  
        let distributed_config = DistributedConfig::default();
        let distributed_coordinator = Arc::new(IntelligentDistributedCoordinator::new(distributed_config).await?);
        
        // Initialize API server with default config
        let api_config = ApiConfig::default();
        let api_server = AdvancedApiServer::new(api_config);
        
        info!("✅ Sprint 5 demo components initialized successfully");
        
        Ok(Self {
            analytics_engine,
            distributed_coordinator,
            api_server,
        })
    }
    
    /// Demonstrate advanced analytics capabilities
    async fn demonstrate_advanced_analytics(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("📊 Demonstrating Advanced Analytics Engine");
        
        // Simulate collecting performance metrics
        info!("  📈 Collecting performance metrics...");
        for i in 0..20 {
            let base_cpu = 20.0 + (i as f64 * 0.5) + (rand::random::<f64>() * 10.0);
            let base_memory = 30.0 + (i as f64 * 0.3) + (rand::random::<f64>() * 15.0);
            
            let data_points = vec![
                AnalyticsDataPoint {
                    id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                    metric_name: "cpu_usage".to_string(),
                    value: base_cpu,
                    runtime_type: Some(RuntimeType::Native),
                    execution_id: Some(format!("exec-{}", i)),
                    tags: HashMap::from([
                        ("node".to_string(), "node-1".to_string()),
                    ]),
                },
                AnalyticsDataPoint {
                    id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                    metric_name: "memory_usage".to_string(),
                    value: base_memory,
                    runtime_type: Some(RuntimeType::Wasm),
                    execution_id: Some(format!("exec-{}", i)),
                    tags: HashMap::from([
                        ("node".to_string(), "node-2".to_string()),
                    ]),
                },
            ];
            
            for data_point in data_points {
                self.analytics_engine.collect_data_point(data_point).await?;
            }
            
            sleep(Duration::from_millis(100)).await;
        }
        
        // Force processing of buffered data
        sleep(Duration::from_secs(2)).await;
        
        // Perform trend analysis
        info!("  🔍 Analyzing performance trends...");
        
        let cpu_trends = self.analytics_engine.analyze_trends("cpu_usage", 1).await?;
        info!("    📊 CPU Usage Trend: {:?} (confidence: {:.2})", 
              cpu_trends.trend, cpu_trends.confidence);
        info!("    📈 CPU Statistics: mean={:.1}%, std_dev={:.1}%", 
              cpu_trends.statistics.mean, cpu_trends.statistics.std_deviation);
        
        let memory_trends = self.analytics_engine.analyze_trends("memory_usage", 1).await?;
        info!("    📊 Memory Usage Trend: {:?} (confidence: {:.2})", 
              memory_trends.trend, memory_trends.confidence);
        info!("    📈 Memory Statistics: mean={:.1}%, std_dev={:.1}%", 
              memory_trends.statistics.mean, memory_trends.statistics.std_deviation);
        
        // Generate predictions
        info!("  🔮 Generating predictive forecasts...");
        let cpu_predictions = self.analytics_engine.predict_values("cpu_usage", 12).await?;
        info!("    🎯 Generated {} CPU usage predictions for next 12 hours", cpu_predictions.len());
        if let Some(first_prediction) = cpu_predictions.first() {
            info!("    📅 Next hour prediction: {:.1}% (confidence: {:.1}% - {:.1}%)", 
                  first_prediction.predicted_value, 
                  first_prediction.confidence_interval.0, 
                  first_prediction.confidence_interval.1);
        }
        
        // Evaluate alerts
        info!("  🚨 Evaluating alert conditions...");
        let alerts = self.analytics_engine.evaluate_alerts().await?;
        info!("    📢 Found {} active alerts", alerts.len());
        for alert in &alerts {
            warn!("    ⚠️  Alert: {} - {} (severity: {:?})", 
                  alert.name, alert.metric_name, alert.severity);
        }
        
        info!("✅ Advanced analytics demonstration completed");
        Ok(())
    }
    
    /// Demonstrate distributed computing coordination
    async fn demonstrate_distributed_computing(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("🌐 Demonstrating Distributed Computing Coordination");
        
        // Join the cluster
        info!("  🤝 Joining distributed cluster...");
        self.distributed_coordinator.join_cluster().await?;
        
        // Wait for cluster stabilization
        sleep(Duration::from_secs(1)).await;
        
        // Monitor cluster health
        info!("  🏥 Monitoring cluster health...");
        let cluster_health = self.distributed_coordinator.monitor_cluster_health().await?;
        info!("    📊 Cluster Status:");
        info!("      💻 Total Nodes: {}", cluster_health.total_nodes);
        info!("      ✅ Healthy Nodes: {}", cluster_health.healthy_nodes);
        info!("      ⚡ Cluster Load: {:.1}%", cluster_health.cluster_load);
        info!("      🔄 Active Executions: {}", cluster_health.resource_utilization.total_active_executions);
        info!("      📈 Success Rate: {:.1}%", cluster_health.performance_metrics.success_rate * 100.0);
        
        // Get cluster nodes
        let nodes = self.distributed_coordinator.get_cluster_nodes().await?;
        info!("    🏢 Discovered {} cluster nodes:", nodes.len());
        for node in &nodes {
            info!("      🖥️  Node: {} ({:?}) - CPU: {:.1}%, Memory: {:.1}%", 
                  node.node_id, node.status, 
                  node.resource_usage.cpu_usage_percent,
                  node.resource_usage.memory_usage_percent);
        }
        
        info!("✅ Distributed computing demonstration completed");
        Ok(())
    }
    
    /// Demonstrate API capabilities
    async fn demonstrate_api_capabilities(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("🌐 Demonstrating Advanced API & Real-time Communication");
        
        // Broadcast some API events
        info!("  📢 Broadcasting real-time events...");
        for i in 0..3 {
            let event = ApiEvent::ExecutionStarted {
                execution_id: Uuid::new_v4(),
            };
            self.api_server.broadcast_event(event);
            
            let alert_event = ApiEvent::AlertTriggered {
                message: format!("Test alert {} - Resource threshold exceeded", i + 1),
            };
            self.api_server.broadcast_event(alert_event);
            
            sleep(Duration::from_millis(500)).await;
        }
        
        info!("  🌍 API endpoints would be available at:");
        info!("    📋 Dashboard: http://127.0.0.1:8080/dashboard");
        info!("    📡 WebSocket: ws://127.0.0.1:8080/ws");
        info!("    📚 REST API: POST /api/v1/executions, GET /api/v1/cluster/status");
        
        info!("✅ API capabilities demonstration completed");
        Ok(())
    }
    
    /// Run comprehensive demo showcasing all Sprint 5 features
    async fn run_comprehensive_demo(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("🎯 Starting Sprint 5 Comprehensive Demo");
        info!("   Theme: Advanced Monitoring & Distributed Computing");
        
        // Run demonstrations
        self.demonstrate_advanced_analytics().await?;
        sleep(Duration::from_secs(1)).await;
        
        self.demonstrate_distributed_computing().await?;
        sleep(Duration::from_secs(1)).await;
        
        self.demonstrate_api_capabilities().await?;
        
        // Final summary
        info!("🏁 Sprint 5 Demo Summary");
        info!("✅ Sprint 5 demonstration completed successfully!");
        info!("🎉 Advanced Monitoring & Distributed Computing features showcased:");
        info!("   📊 Real-time analytics with ML-powered predictions");
        info!("   🌐 Intelligent distributed computing coordination");
        info!("   🔄 Resource-based load balancing with health monitoring");
        info!("   📡 Comprehensive REST API with WebSocket real-time updates");
        info!("   🚨 Predictive alerting with configurable thresholds");
        
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info,toadstool=debug")
        .init();
    
    info!("🚀 ToadStool Sprint 5: Advanced Monitoring & Distributed Computing Demo");
    info!("============================================================================");
    
    // Initialize and run demo
    let demo = Sprint5Demo::new().await?;
    demo.run_comprehensive_demo().await?;
    
    Ok(())
} 