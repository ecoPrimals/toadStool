# Collaborative Intelligence - Usage Examples

**Last Updated**: January 11, 2026  
**Status**: Production Ready  
**Version**: 2.2.0

---

## Overview

This document provides comprehensive examples for using ToadStool's Collaborative Intelligence Resource Planning API. These examples demonstrate the modern builder pattern API and integration with biomeOS.

---

## Table of Contents

1. [Quick Start](#quick-start)
2. [Basic Examples](#basic-examples)
3. [Advanced Patterns](#advanced-patterns)
4. [biomeOS Integration](#biomeos-integration)
5. [Best Practices](#best-practices)

---

## Quick Start

### Creating a Simple Graph

```rust
use toadstool_server::graph_types::{ExecutionGraph, GraphNode, GraphEdge};

// Create nodes using the builder pattern
let prepare = GraphNode::builder("prepare", "cpu_compute")
    .cpu(2.0)
    .memory_gb(1)
    .duration_secs(5)
    .build();

let process = GraphNode::builder("process", "gpu_compute")
    .cpu(4.0)
    .memory_gb(8)
    .gpu_memory_gb(16)
    .duration_secs(60)
    .build();

let finalize = GraphNode::builder("finalize", "cpu_compute")
    .cpu(2.0)
    .memory_gb(2)
    .duration_secs(10)
    .build();

// Create graph using builder
let graph = ExecutionGraph::builder("my_pipeline")
    .node(prepare)
    .node(process)
    .node(finalize)
    .connect("prepare", "process")
    .connect("process", "finalize")
    .build();

// Validate the graph
graph.validate().expect("Graph should be valid");
```

### Estimating Resources

```rust
use toadstool_server::resource_estimator::ResourceEstimator;

let estimator = ResourceEstimator::new();
let estimate = estimator.estimate(&graph).await?;

println!("Total CPU cores: {}", estimate.cpu_cores);
println!("Total memory: {} GB", estimate.memory_bytes / (1024 * 1024 * 1024));
println!("Duration: {:?}", estimate.estimated_duration);
println!("Max parallelism: {}", estimate.max_parallelism);
```

### Validating Availability

```rust
use toadstool_server::resource_validator::ResourceValidator;

let validator = ResourceValidator::new();
let availability = validator.validate_availability(&graph).await?;

if availability.available {
    println!("✅ System has sufficient resources");
} else {
    println!("❌ Resource gaps detected:");
    for gap in availability.gaps {
        println!("  - {}: need {} more {}", 
                 gap.resource_type, gap.shortage, gap.suggestion);
    }
}
```

### Getting Optimization Suggestions

```rust
use toadstool_server::resource_optimizer::ResourceOptimizer;

let optimizer = ResourceOptimizer::new();
let suggestions = optimizer.suggest_optimizations(&graph).await?;

println!("Bottlenecks:");
for bottleneck in suggestions.bottlenecks {
    println!("  - {}: {}", bottleneck.bottleneck_type, bottleneck.description);
}

println!("\nOptimization opportunities:");
for opportunity in suggestions.opportunities {
    println!("  - {}: {} (priority: {})", 
             opportunity.optimization_type,
             opportunity.description,
             opportunity.priority);
}
```

---

## Basic Examples

### Example 1: Sequential Data Pipeline

A simple ETL (Extract, Transform, Load) pipeline:

```rust
let graph = ExecutionGraph::builder("etl_pipeline")
    .node(GraphNode::builder("extract", "data_ingestion")
        .cpu(2.0)
        .memory_gb(4)
        .storage_gb(50)
        .duration_secs(30)
        .metadata("source", "s3://data-bucket")
        .build())
    .node(GraphNode::builder("transform", "data_processing")
        .cpu(8.0)
        .memory_gb(16)
        .duration_secs(120)
        .metadata("engine", "spark")
        .build())
    .node(GraphNode::builder("load", "data_storage")
        .cpu(2.0)
        .memory_gb(4)
        .storage_gb(100)
        .duration_secs(45)
        .metadata("target", "postgresql")
        .build())
    .connect("extract", "transform")
    .connect("transform", "load")
    .metadata("pipeline_version", "1.0")
    .build();
```

### Example 2: Parallel Processing

Fan-out pattern for parallel data processing:

```rust
let mut builder = ExecutionGraph::builder("parallel_processing")
    .node(GraphNode::builder("split", "data_partitioning")
        .cpu(2.0)
        .memory_gb(4)
        .duration_secs(10)
        .build());

// Add 10 parallel workers
for i in 0..10 {
    let worker_id = format!("worker_{}", i);
    builder = builder
        .node(GraphNode::builder(&worker_id, "batch_processing")
            .cpu(4.0)
            .memory_gb(8)
            .duration_secs(60)
            .metadata("batch_id", &i.to_string())
            .build())
        .connect("split", &worker_id);
}

// Add aggregation node
let graph = builder
    .node(GraphNode::builder("aggregate", "data_aggregation")
        .cpu(4.0)
        .memory_gb(16)
        .duration_secs(20)
        .build())
    .build();

// Connect all workers to aggregator
for i in 0..10 {
    graph.edges.push(GraphEdge::new(format!("worker_{}", i), "aggregate"));
}
```

### Example 3: Machine Learning Training Pipeline

ML training with data preprocessing and model evaluation:

```rust
let graph = ExecutionGraph::builder("ml_training")
    // Data preparation
    .node(GraphNode::builder("load_dataset", "data_loading")
        .cpu(4.0)
        .memory_gb(16)
        .storage_gb(100)
        .duration_secs(60)
        .build())
    
    // Feature engineering (parallel)
    .node(GraphNode::builder("feature_eng_a", "feature_extraction")
        .cpu(8.0)
        .memory_gb(32)
        .duration_secs(300)
        .build())
    .node(GraphNode::builder("feature_eng_b", "feature_extraction")
        .cpu(8.0)
        .memory_gb(32)
        .duration_secs(300)
        .build())
    
    // Model training (GPU)
    .node(GraphNode::builder("train_model", "gpu_training")
        .cpu(4.0)
        .memory_gb(64)
        .gpu_memory_gb(48)
        .duration_secs(3600)
        .primal("toadstool")  // Specify primal for GPU work
        .metadata("model_type", "transformer")
        .metadata("epochs", "100")
        .build())
    
    // Model evaluation
    .node(GraphNode::builder("evaluate", "model_validation")
        .cpu(8.0)
        .memory_gb(32)
        .gpu_memory_gb(24)
        .duration_secs(300)
        .build())
    
    // Connect the pipeline
    .connect("load_dataset", "feature_eng_a")
    .connect("load_dataset", "feature_eng_b")
    .connect("feature_eng_a", "train_model")
    .connect("feature_eng_b", "train_model")
    .connect("train_model", "evaluate")
    
    .metadata("experiment_id", "exp_2026_01_11")
    .metadata("framework", "pytorch")
    .build();
```

---

## Advanced Patterns

### Pattern 1: Diamond Topology (Fork-Join)

Parallel processing with synchronization:

```rust
let graph = ExecutionGraph::builder("diamond_workflow")
    .node(GraphNode::builder("start", "initialization")
        .cpu(2.0).memory_gb(2).duration_secs(5).build())
    
    // Fork into two branches
    .node(GraphNode::builder("branch_a", "processing_a")
        .cpu(4.0).memory_gb(8).duration_secs(60).build())
    .node(GraphNode::builder("branch_b", "processing_b")
        .cpu(4.0).memory_gb(8).duration_secs(45).build())
    
    // Join at synchronization point
    .node(GraphNode::builder("sync", "synchronization")
        .cpu(4.0).memory_gb(16).duration_secs(20).build())
    
    .connect("start", "branch_a")
    .connect("start", "branch_b")
    .connect("branch_a", "sync")
    .connect("branch_b", "sync")
    .build();
```

### Pattern 2: Multi-Stage Pipeline

Complex pipeline with multiple processing stages:

```rust
let graph = ExecutionGraph::builder("multi_stage_pipeline")
    // Stage 1: Data ingestion (3 sources)
    .node(GraphNode::builder("source_1", "data_ingestion").cpu(2.0).memory_gb(4).build())
    .node(GraphNode::builder("source_2", "data_ingestion").cpu(2.0).memory_gb(4).build())
    .node(GraphNode::builder("source_3", "data_ingestion").cpu(2.0).memory_gb(4).build())
    
    // Stage 2: Validation
    .node(GraphNode::builder("validate", "data_validation").cpu(4.0).memory_gb(8).build())
    
    // Stage 3: Processing (parallel)
    .node(GraphNode::builder("process_a", "transformation").cpu(8.0).memory_gb(16).build())
    .node(GraphNode::builder("process_b", "transformation").cpu(8.0).memory_gb(16).build())
    
    // Stage 4: Aggregation
    .node(GraphNode::builder("aggregate", "data_merge").cpu(4.0).memory_gb(32).build())
    
    // Stage 5: Export
    .node(GraphNode::builder("export", "data_export").cpu(2.0).memory_gb(8).build())
    
    // Connect stages
    .connect("source_1", "validate")
    .connect("source_2", "validate")
    .connect("source_3", "validate")
    .connect("validate", "process_a")
    .connect("validate", "process_b")
    .connect("process_a", "aggregate")
    .connect("process_b", "aggregate")
    .connect("aggregate", "export")
    .build();
```

### Pattern 3: Dynamic Workload Adaptation

Graph with conditional execution based on intermediate results:

```rust
// Note: This pattern shows the graph structure. Actual conditional
// execution would be handled by the workflow engine.

let graph = ExecutionGraph::builder("adaptive_workflow")
    .node(GraphNode::builder("analyze_input", "analysis")
        .cpu(4.0).memory_gb(8).duration_secs(30).build())
    
    // Light path (for small datasets)
    .node(GraphNode::builder("light_process", "cpu_compute")
        .cpu(4.0).memory_gb(16).duration_secs(120)
        .metadata("path", "light").build())
    
    // Heavy path (for large datasets)
    .node(GraphNode::builder("heavy_process", "gpu_compute")
        .cpu(8.0).memory_gb(64).gpu_memory_gb(32).duration_secs(600)
        .metadata("path", "heavy").build())
    
    // Convergence point
    .node(GraphNode::builder("finalize", "aggregation")
        .cpu(4.0).memory_gb(16).duration_secs(60).build())
    
    // Note: Both paths exist in graph; executor chooses based on runtime conditions
    .connect("analyze_input", "light_process")
    .connect("analyze_input", "heavy_process")
    .connect("light_process", "finalize")
    .connect("heavy_process", "finalize")
    
    .metadata("execution_strategy", "conditional")
    .build();
```

---

## biomeOS Integration

### JSON-RPC API Usage

Call ToadStool's collaborative intelligence API via JSON-RPC 2.0:

```bash
# Estimate resources
curl --unix-socket /run/user/1000/toadstool-default.jsonrpc.sock \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "resources.estimate",
    "params": {
      "graph": {
        "id": "ml_training_pipeline",
        "nodes": [
          {
            "id": "data_prep",
            "primal": "toadstool",
            "operation": "cpu_compute",
            "requirements": {
              "cpu": {"min_cores": 4.0},
              "memory": {"min_bytes": 8589934592}
            },
            "duration": 300
          },
          {
            "id": "model_train",
            "primal": "toadstool",
            "operation": "gpu_compute",
            "requirements": {
              "cpu": {"min_cores": 8.0},
              "memory": {"min_bytes": 68719476736},
              "gpu": {"min_units": 1, "min_memory_bytes": 17179869184}
            },
            "duration": 3600
          }
        ],
        "edges": [
          {
            "from": "data_prep",
            "to": "model_train",
            "edge_type": "data_flow"
          }
        ]
      }
    },
    "id": 1
  }'
```

### Python Integration (via biomeOS)

```python
import json
import socket

def call_toadstool_collaborative_intel(method: str, graph: dict) -> dict:
    """Call ToadStool collaborative intelligence API."""
    sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    sock.connect("/run/user/1000/toadstool-default.jsonrpc.sock")
    
    request = {
        "jsonrpc": "2.0",
        "method": method,
        "params": {"graph": graph},
        "id": 1
    }
    
    # Send HTTP request
    http_request = f"""POST / HTTP/1.1\r
Content-Type: application/json\r
Content-Length: {len(json.dumps(request))}\r
\r
{json.dumps(request)}"""
    
    sock.sendall(http_request.encode())
    response = sock.recv(4096).decode()
    
    # Parse HTTP response
    body_start = response.find('\r\n\r\n') + 4
    body = json.loads(response[body_start:])
    
    sock.close()
    return body["result"]

# Example: Estimate resources
graph = {
    "id": "my_pipeline",
    "nodes": [
        {
            "id": "preprocess",
            "operation": "cpu_compute",
            "requirements": {
                "cpu": {"min_cores": 4.0},
                "memory": {"min_bytes": 8 * 1024 * 1024 * 1024}
            },
            "duration": 60
        },
        {
            "id": "train",
            "operation": "gpu_compute",
            "requirements": {
                "cpu": {"min_cores": 8.0},
                "memory": {"min_bytes": 64 * 1024 * 1024 * 1024},
                "gpu": {"min_units": 1, "min_memory_bytes": 16 * 1024 * 1024 * 1024}
            },
            "duration": 3600
        }
    ],
    "edges": [
        {"from": "preprocess", "to": "train"}
    ]
}

# Get resource estimate
estimate = call_toadstool_collaborative_intel("resources.estimate", graph)
print(f"Estimated CPU: {estimate['cpu_cores']} cores")
print(f"Estimated Memory: {estimate['memory_bytes'] / (1024**3):.2f} GB")
print(f"Duration: {estimate['estimated_duration']['secs']} seconds")

# Validate availability
availability = call_toadstool_collaborative_intel("resources.validate_availability", graph)
if availability["available"]:
    print("✅ Resources available")
else:
    print("❌ Resource gaps:")
    for gap in availability["gaps"]:
        print(f"  - {gap['resource_type']}: {gap['shortage']} {gap['unit']} short")

# Get optimization suggestions
suggestions = call_toadstool_collaborative_intel("resources.suggest_optimizations", graph)
print(f"Found {len(suggestions['opportunities'])} optimization opportunities")
for opp in suggestions["opportunities"]:
    print(f"  - {opp['optimization_type']}: {opp['description']}")
```

### Rust Integration (Direct Library Usage)

```rust
use toadstool_server::{
    graph_types::ExecutionGraph,
    resource_estimator::ResourceEstimator,
    resource_validator::ResourceValidator,
    resource_optimizer::ResourceOptimizer,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build graph (using builder pattern)
    let graph = ExecutionGraph::builder("biomeos_workflow")
        .node(GraphNode::builder("analyze", "cpu_compute")
            .cpu(8.0).memory_gb(16).duration_secs(120).build())
        .node(GraphNode::builder("process", "gpu_compute")
            .cpu(16.0).memory_gb(64).gpu_memory_gb(24).duration_secs(600).build())
        .connect("analyze", "process")
        .build();
    
    // Estimate resources
    let estimator = ResourceEstimator::new();
    let estimate = estimator.estimate(&graph).await?;
    
    // Validate availability
    let validator = ResourceValidator::new();
    let availability = validator.validate_availability(&graph).await?;
    
    // Get optimizations
    let optimizer = ResourceOptimizer::new();
    let suggestions = optimizer.suggest_optimizations(&graph).await?;
    
    // Make decisions based on analysis
    if !availability.available {
        println!("Waiting for resources or applying optimizations...");
        // Apply suggested optimizations or defer execution
    } else {
        println!("Proceeding with execution");
        // Execute the graph
    }
    
    Ok(())
}
```

---

## Best Practices

### 1. Use Builder Pattern for Clarity

**❌ Don't:**
```rust
let node = GraphNode {
    id: "node1".to_string(),
    primal: "toadstool".to_string(),
    operation: "compute".to_string(),
    requirements: NodeResourceRequirements {
        cpu: Some(CpuRequirements { min_cores: 4.0, max_cores: None, architecture: None }),
        memory: Some(MemoryRequirements { min_bytes: 8589934592, max_bytes: None }),
        // ... 10 more lines ...
    },
    duration: Some(Duration::from_secs(60)),
    metadata: HashMap::new(),
};
```

**✅ Do:**
```rust
let node = GraphNode::builder("node1", "compute")
    .cpu(4.0)
    .memory_gb(8)
    .duration_secs(60)
    .build();
```

### 2. Validate Graphs Before Execution

```rust
// Always validate before estimating or executing
match graph.validate() {
    Ok(_) => {
        // Proceed with estimation/execution
        let estimate = estimator.estimate(&graph).await?;
    }
    Err(e) => {
        eprintln!("Invalid graph: {}", e);
        // Handle error: fix graph or abort
    }
}
```

### 3. Handle Resource Gaps Gracefully

```rust
let availability = validator.validate_availability(&graph).await?;

if !availability.available {
    // Check if gaps are minor or major
    for gap in &availability.gaps {
        if gap.shortage > gap.available * 2 {
            // Major gap - defer or reject
            return Err("Insufficient resources");
        }
    }
    
    // Minor gaps - try optimizations
    let suggestions = optimizer.suggest_optimizations(&graph).await?;
    // Apply high-priority suggestions
}
```

### 4. Use Metadata for Context

```rust
let node = GraphNode::builder("training", "gpu_compute")
    .cpu(16.0)
    .memory_gb(128)
    .gpu_memory_gb(80)
    .metadata("model_name", "llama-70b")
    .metadata("checkpoint_interval", "100")
    .metadata("distributed", "true")
    .metadata("world_size", "8")
    .build();
```

### 5. Set Realistic Duration Estimates

```rust
// Use historical data or profiling for accurate durations
let node = GraphNode::builder("inference", "gpu_compute")
    .cpu(8.0)
    .memory_gb(32)
    .gpu_memory_gb(24)
    .duration_secs(estimate_from_batch_size(batch_size))
    .metadata("batch_size", &batch_size.to_string())
    .build();

fn estimate_from_batch_size(batch_size: usize) -> u64 {
    // Based on profiling: ~0.05s per item
    (batch_size as f64 * 0.05) as u64
}
```

### 6. Leverage Parallel Execution

```rust
// Identify independent nodes that can run in parallel
let graph = ExecutionGraph::builder("parallel_workflow")
    .node(prep)
    // These can run in parallel (no dependencies)
    .node(task_a).connect("prep", "task_a")
    .node(task_b).connect("prep", "task_b")
    .node(task_c).connect("prep", "task_c")
    // Synchronize at merge point
    .node(merge)
    .connect("task_a", "merge")
    .connect("task_b", "merge")
    .connect("task_c", "merge")
    .build();
```

### 7. Monitor and Adjust

```rust
// Get initial estimate
let initial_estimate = estimator.estimate(&graph).await?;

// After execution, compare actual vs estimated
let actual_duration = execution_result.duration;
let accuracy = (initial_estimate.estimated_duration.as_secs() as f64 
                / actual_duration.as_secs() as f64);

if accuracy < 0.7 || accuracy > 1.3 {
    // Estimation was off by >30% - update duration estimates
    update_duration_model(graph.id, actual_duration);
}
```

---

## Conclusion

ToadStool's Collaborative Intelligence API provides powerful resource planning capabilities with a modern, ergonomic Rust API. The builder pattern reduces boilerplate by ~70%, making it easy to construct complex execution graphs and get accurate resource estimates.

For more information:
- [API Specification](../specs/COLLABORATIVE_INTELLIGENCE_RESOURCE_PLANNING.md)
- [Implementation Tracker](../COLLABORATIVE_INTELLIGENCE_TRACKER.md)
- [biomeOS Integration Guide](./BIOMEOS_INTEGRATION_GUIDE.md)

Different orders of the same architecture. 🍄🐸

