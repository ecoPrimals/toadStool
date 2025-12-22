# Songbird Integration Guide

## 📘 Overview

Toadstool integrates with Songbird (the universal signal coordinator) for distributed workload execution across multiple nodes. This integration enables:

- **Distributed Job Execution**: Break massive jobs into subtasks and distribute them across the network
- **Node Discovery**: Find and connect to other compute nodes in the network
- **Load Balancing**: Intelligently distribute workloads based on node capacity
- **Capability Matching**: Find nodes with specific capabilities (GPU, specific architectures, etc.)

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         Toadstool                           │
│  ┌───────────────────────────────────────────────────────┐  │
│  │           ToadStoolSongbirdIntegration                │  │
│  ├───────────────────────────────────────────────────────┤  │
│  │  • SongbirdConnection     (connectivity)              │  │
│  │  • LocalCapacityManager   (resource tracking)         │  │
│  │  • UniversalScheduler     (local job scheduling)      │  │
│  └───────────────────────────────────────────────────────┘  │
│                           ↕                                 │
│                 Protocol Layer (HTTP/gRPC/WS/MQ)            │
│                           ↕                                 │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│                        Songbird                             │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  • Job Distribution                                   │  │
│  │  • Node Registry                                      │  │
│  │  • Capability Discovery                               │  │
│  │  • Load Balancing                                     │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│              Distributed Toadstool Nodes                    │
│  Node 1      Node 2      Node 3      ...      Node N        │
└─────────────────────────────────────────────────────────────┘
```

## 🔌 Protocol Support

Toadstool supports multiple protocols for Songbird communication:

1. **HTTP** - RESTful API, best for simple integrations
2. **gRPC** - High-performance RPC, best for low latency
3. **WebSocket** - Real-time bidirectional communication
4. **MessageQueue** - Async messaging, best for resilience

### Protocol Selection

```rust
use toadstool_distributed::songbird_integration::*;

// HTTP (default)
let connection = SongbirdConnection {
    active_endpoint: "http://songbird.local:8080".to_string(),
    protocol_config: ProtocolConfig {
        protocol: SongbirdProtocol::HTTP,
        connection_pool: ConnectionPoolConfig::default(),
        timeout_ms: 30000,
    },
    // ...
};

// gRPC (low latency)
let connection = SongbirdConnection {
    active_endpoint: "grpc://songbird.local:50051".to_string(),
    protocol_config: ProtocolConfig {
        protocol: SongbirdProtocol::GRPC,
        connection_pool: ConnectionPoolConfig::default(),
        timeout_ms: 5000,
    },
    // ...
};
```

## 🚀 Usage Examples

### Basic Integration Setup

```rust
use toadstool_distributed::songbird_integration::*;
use toadstool::UniversalScheduler;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create Songbird connection
    let connection = SongbirdConnection {
        active_endpoint: "http://songbird.local:8080".to_string(),
        protocol_config: ProtocolConfig {
            protocol: SongbirdProtocol::HTTP,
            connection_pool: ConnectionPoolConfig::default(),
            timeout_ms: 30000,
        },
        auth_config: AuthConfig {
            auth_type: AuthType::None,
            credentials: None,
        },
        // ... other fields
    };
    
    // Create capacity config
    let capacity_config = CapacityConfig {
        max_concurrent_jobs: 100,
        max_cpu_cores: 16.0,
        max_memory_gb: 64.0,
        max_storage_gb: 1000.0,
        max_gpu_count: 2,
        reserved_cpu_cores: 2.0,
        reserved_memory_gb: 8.0,
    };
    
    // Create universal scheduler
    let scheduler = Arc::new(UniversalScheduler::new(/* ... */));
    
    // Create Songbird integration
    let songbird = ToadStoolSongbirdIntegration::new(
        "toadstool-node-1".to_string(),
        connection,
        capacity_config,
        scheduler,
    ).await?;
    
    Ok(())
}
```

### Distributed Job Execution

```rust
use toadstool_distributed::songbird_integration::*;
use toadstool::UniversalJob;

async fn execute_distributed_job(
    songbird: &ToadStoolSongbirdIntegration,
    job: UniversalJob,
) -> Result<Vec<SubTaskHandle>, Box<dyn std::error::Error>> {
    // Analyze job for distribution strategy
    let analysis = songbird.analyze_job_for_distribution(&job).await?;
    
    // Create distribution plan
    let plan = songbird.create_distribution_plan(&job, &analysis).await?;
    
    // Execute distributed job
    let handles = songbird.execute_distributed_job(job, plan).await?;
    
    // Monitor progress
    for handle in &handles {
        println!("SubTask {} submitted to nodes: {:?}",
            handle.subtask_id, handle.target_nodes);
    }
    
    Ok(handles)
}
```

### Node Discovery

```rust
use toadstool_distributed::songbird_integration::*;

async fn discover_nodes(
    songbird: &ToadStoolSongbirdIntegration,
) -> Result<Vec<NodeRegistration>, Box<dyn std::error::Error>> {
    // Discover all available nodes
    let nodes = songbird.discover_nodes().await?;
    
    for node in &nodes {
        println!("Found node: {} ({:?})",
            node.node_id, node.node_type);
        println!("  Capabilities: CPU={:.2}, Memory={:.2}GB, GPU={}",
            node.capabilities.cpu_cores,
            node.capabilities.memory_gb,
            node.capabilities.gpu_count);
    }
    
    Ok(nodes)
}
```

### Capability-Based Node Selection

```rust
use toadstool_distributed::songbird_integration::*;

async fn find_gpu_nodes(
    songbird: &ToadStoolSongbirdIntegration,
    min_gpu_count: u32,
) -> Result<Vec<NodeRegistration>, Box<dyn std::error::Error>> {
    let all_nodes = songbird.discover_nodes().await?;
    
    // Filter for GPU nodes
    let gpu_nodes: Vec<_> = all_nodes
        .into_iter()
        .filter(|node| node.capabilities.gpu_count >= min_gpu_count)
        .collect();
    
    println!("Found {} nodes with {} or more GPUs",
        gpu_nodes.len(), min_gpu_count);
    
    Ok(gpu_nodes)
}
```

## 🔐 Security

### Authentication

Songbird integration supports multiple authentication methods:

```rust
// No authentication (local/trusted network)
let auth_config = AuthConfig {
    auth_type: AuthType::None,
    credentials: None,
};

// API Key authentication
let auth_config = AuthConfig {
    auth_type: AuthType::ApiKey,
    credentials: Some(Credentials {
        api_key: Some("your-api-key".to_string()),
        ..Default::default()
    }),
};

// mTLS authentication
let auth_config = AuthConfig {
    auth_type: AuthType::MutualTLS,
    credentials: Some(Credentials {
        client_cert_path: Some("/path/to/cert.pem".to_string()),
        client_key_path: Some("/path/to/key.pem".to_string()),
        ca_cert_path: Some("/path/to/ca.pem".to_string()),
        ..Default::default()
    }),
};
```

### Encryption

All data sent through Songbird can be encrypted using the Toadstool encryption layer:

```rust
use toadstool::ExecutionRequest;

let mut request = ExecutionRequest {
    // ... normal fields
    encryption_config: Some(EncryptionConfig {
        required: true,
        encrypt_results: true,
        min_security_level: SecurityLevel::Enhanced,
        ..Default::default()
    }),
};
```

## 📊 Monitoring & Observability

### Capacity Monitoring

```rust
async fn monitor_capacity(
    songbird: &ToadStoolSongbirdIntegration,
) -> Result<(), Box<dyn std::error::Error>> {
    let capacity = songbird.get_local_capacity().await?;
    
    println!("Local Capacity:");
    println!("  CPU: {:.2} / {:.2} cores",
        capacity.available_capacity.cpu_cores,
        capacity.total_capacity.cpu_cores);
    println!("  Memory: {:.2} / {:.2} GB",
        capacity.available_capacity.memory_gb,
        capacity.total_capacity.memory_gb);
    println!("  Active Jobs: {}", capacity.active_job_count);
    
    Ok(())
}
```

### Job Status Tracking

```rust
async fn track_distributed_job(
    handles: Vec<SubTaskHandle>,
) -> Result<(), Box<dyn std::error::Error>> {
    for handle in handles {
        match handle.status {
            SubTaskStatus::Submitted => println!("SubTask {} submitted", handle.subtask_id),
            SubTaskStatus::Running => println!("SubTask {} running", handle.subtask_id),
            SubTaskStatus::Completed => println!("SubTask {} completed", handle.subtask_id),
            SubTaskStatus::Failed => println!("SubTask {} failed", handle.subtask_id),
        }
    }
    
    Ok(())
}
```

## 🎯 Best Practices

### 1. Graceful Degradation

Always design for Songbird unavailability:

```rust
async fn execute_with_fallback(
    songbird: Option<&ToadStoolSongbirdIntegration>,
    job: UniversalJob,
    local_scheduler: &UniversalScheduler,
) -> Result<(), Box<dyn std::error::Error>> {
    match songbird {
        Some(sb) if sb.is_available().await => {
            // Use Songbird for distributed execution
            sb.execute_distributed_job(job, /* plan */).await?;
        }
        _ => {
            // Fall back to local execution
            local_scheduler.schedule_job(job).await?;
        }
    }
    
    Ok(())
}
```

### 2. Resource Limits

Always configure resource limits to prevent oversubscription:

```rust
let capacity_config = CapacityConfig {
    max_concurrent_jobs: 50,
    max_cpu_cores: num_cpus::get() as f64,
    max_memory_gb: (sysinfo::System::new_all().total_memory() / 1024 / 1024 / 1024) as f64,
    reserved_cpu_cores: 2.0,  // Reserve for system
    reserved_memory_gb: 4.0,  // Reserve for system
    ..Default::default()
};
```

### 3. Retry Logic

Implement exponential backoff for Songbird connection failures:

```rust
use tokio::time::{sleep, Duration};

async fn connect_with_retry(
    max_retries: u32,
) -> Result<ToadStoolSongbirdIntegration, Box<dyn std::error::Error>> {
    let mut retries = 0;
    let mut delay = Duration::from_secs(1);
    
    loop {
        match ToadStoolSongbirdIntegration::new(/* ... */).await {
            Ok(integration) => return Ok(integration),
            Err(e) if retries < max_retries => {
                eprintln!("Connection failed: {}, retrying in {:?}", e, delay);
                sleep(delay).await;
                delay *= 2;
                retries += 1;
            }
            Err(e) => return Err(e.into()),
        }
    }
}
```

## 🧪 Testing

See `tests/songbird_integration_tests.rs` for comprehensive integration tests.

### Unit Tests

Test individual components in isolation:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_capacity_tracking() {
        let capacity_config = CapacityConfig {
            max_concurrent_jobs: 10,
            max_cpu_cores: 8.0,
            ..Default::default()
        };
        
        let manager = LocalCapacityManager::new(capacity_config).await.unwrap();
        let info = manager.get_capacity_info().await;
        
        assert_eq!(info.total_capacity.cpu_cores, 8.0);
    }
}
```

### Integration Tests

Test with mock Songbird service:

```rust
#[tokio::test]
async fn test_distributed_job_execution() {
    let mock_songbird = MockSongbirdService::start().await;
    
    let connection = SongbirdConnection {
        active_endpoint: mock_songbird.url(),
        // ... configuration
    };
    
    let songbird = ToadStoolSongbirdIntegration::new(/* ... */).await.unwrap();
    
    let job = create_test_job();
    let handles = songbird.execute_distributed_job(job, /* plan */).await.unwrap();
    
    assert!(!handles.is_empty());
}
```

## 📚 Additional Resources

- [Songbird API Documentation](https://songbird.docs.local)
- [Toadstool Universal Scheduler](../../universal/README.md)
- [Encryption Layer Guide](../../../core/toadstool/src/encryption/README.md)
- [Performance Tuning](../../../docs/performance.md)

## 🐛 Troubleshooting

### Connection Issues

```rust
// Check Songbird availability
if !songbird.is_available().await {
    eprintln!("Songbird unavailable, check network and endpoint configuration");
}

// Test connectivity
songbird.health_check().await?;
```

### Resource Exhaustion

```rust
// Monitor capacity before submission
let capacity = songbird.get_local_capacity().await?;
if capacity.available_capacity.cpu_cores < required_cores {
    eprintln!("Insufficient CPU capacity available");
}
```

### Job Failures

```rust
// Check subtask status
for handle in handles {
    if let SubTaskStatus::Failed = handle.status {
        eprintln!("SubTask {} failed on nodes: {:?}",
            handle.subtask_id, handle.target_nodes);
        // Implement retry logic
    }
}
```

## 🔄 Migration from Hardcoded Integration

If migrating from a hardcoded Songbird integration:

1. **Remove hardcoded URLs**: Use `SongbirdConnection` with discovery
2. **Add fallback logic**: Gracefully handle Songbird unavailability  
3. **Use capability matching**: Replace manual node selection with capability queries
4. **Enable encryption**: Add `encryption_config` to sensitive workloads
5. **Monitor capacity**: Track resource usage to prevent oversubscription

## 📝 License

AGPL-3.0 (same as Toadstool core)

