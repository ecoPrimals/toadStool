# biomeOS Neural API Adapter Specification

**Last Updated**: January 11, 2026  
**Status**: Ready for Implementation (awaiting biomeOS neural API spec)  
**ToadStool Version**: 2.2.0+

---

## Overview

This document specifies the adapter layer for converting between biomeOS's Neural API graph format and ToadStool's execution graph format. This enables seamless integration where biomeOS can submit graphs in its native format and ToadStool transparently converts them for resource planning.

### Goals

✅ **Bidirectional Conversion**: biomeOS ↔ ToadStool graph formats  
✅ **Zero Information Loss**: Preserve all metadata and semantics  
✅ **Type-Safe**: Compile-time validation of conversions  
✅ **Extensible**: Easy to add new node/edge types  
✅ **Performance**: Minimal overhead (<1ms for typical graphs)

---

## Table of Contents

1. [Architecture](#architecture)
2. [Format Comparison](#format-comparison)
3. [Conversion Traits](#conversion-traits)
4. [Implementation Plan](#implementation-plan)
5. [Testing Strategy](#testing-strategy)
6. [Integration Examples](#integration-examples)

---

## Architecture

### Adapter Pattern

```
┌─────────────────────┐
│   biomeOS Client    │
│   (Python/Rust)     │
└──────────┬──────────┘
           │
           │ Neural API Graph JSON
           ▼
┌─────────────────────┐
│  JSON-RPC Endpoint  │
│  (ToadStool Server) │
└──────────┬──────────┘
           │
           │ Detect format
           ▼
┌─────────────────────┐
│  Neural API Adapter │◄─── Conversion Traits
│  (This Module)      │
└──────────┬──────────┘
           │
           │ ToadStool ExecutionGraph
           ▼
┌─────────────────────┐
│  Resource Estimator │
│  Validator          │
│  Optimizer          │
└─────────────────────┘
```

### Design Principles

1. **Transparent**: biomeOS clients don't need to know ToadStool's internal format
2. **Backward Compatible**: Existing ToadStool format still works
3. **Format Detection**: Auto-detect which format is being used
4. **Validated**: Conversions validate both input and output
5. **Documented**: Clear error messages for invalid conversions

---

## Format Comparison

### Assumed biomeOS Neural API Format

**Note**: Draft based on common neural graph representations. Update when biomeOS specification stabilizes.

```json
{
  "graph_id": "neural_workflow_123",
  "graph_type": "neural_api",
  "version": "1.0",
  "nodes": [
    {
      "node_id": "input_layer",
      "node_type": "data_input",
      "operation": {
        "op_type": "load_data",
        "params": {
          "source": "s3://bucket/data",
          "format": "parquet"
        }
      },
      "resources": {
        "compute": {
          "cpu_cores": 4,
          "memory_gb": 16
        },
        "duration_secs": 30
      },
      "metadata": {
        "layer_type": "input",
        "data_size_gb": 50
      }
    },
    {
      "node_id": "transform_layer",
      "node_type": "computation",
      "operation": {
        "op_type": "transform",
        "params": {
          "function": "normalize_scale"
        }
      },
      "resources": {
        "compute": {
          "cpu_cores": 16,
          "memory_gb": 64,
          "gpu_required": true,
          "gpu_memory_gb": 24
        },
        "duration_secs": 300
      },
      "metadata": {
        "layer_type": "processing"
      }
    }
  ],
  "edges": [
    {
      "source": "input_layer",
      "target": "transform_layer",
      "edge_type": "data_dependency",
      "metadata": {
        "data_volume_gb": 50
      }
    }
  ],
  "metadata": {
    "workflow_name": "neural_training_pipeline",
    "owner": "biomeos",
    "priority": "high"
  }
}
```

### ToadStool Execution Graph Format

```json
{
  "id": "neural_workflow_123",
  "nodes": [
    {
      "id": "input_layer",
      "primal": "toadstool",
      "operation": "data_input",
      "requirements": {
        "cpu": {
          "min_cores": 4.0,
          "max_cores": null,
          "architecture": null
        },
        "memory": {
          "min_bytes": 17179869184,
          "max_bytes": null
        }
      },
      "duration": 30,
      "metadata": {
        "layer_type": "input",
        "data_size_gb": "50",
        "biomeos_node_type": "data_input",
        "biomeos_op_type": "load_data",
        "source": "s3://bucket/data",
        "format": "parquet"
      }
    },
    {
      "id": "transform_layer",
      "primal": "toadstool",
      "operation": "computation",
      "requirements": {
        "cpu": {
          "min_cores": 16.0,
          "max_cores": null,
          "architecture": null
        },
        "memory": {
          "min_bytes": 68719476736,
          "max_bytes": null
        },
        "gpu": {
          "min_units": 1,
          "max_units": null,
          "gpu_type": null,
          "min_memory_bytes": 25769803776
        }
      },
      "duration": 300,
      "metadata": {
        "layer_type": "processing",
        "biomeos_node_type": "computation",
        "biomeos_op_type": "transform",
        "function": "normalize_scale"
      }
    }
  ],
  "edges": [
    {
      "from": "input_layer",
      "to": "transform_layer",
      "edge_type": "data_flow",
      "metadata": {
        "data_volume_gb": "50",
        "biomeos_edge_type": "data_dependency"
      }
    }
  ],
  "metadata": {
    "workflow_name": "neural_training_pipeline",
    "owner": "biomeos",
    "priority": "high",
    "source_format": "biomeos_neural_api_v1"
  }
}
```

### Key Differences

| Aspect | biomeOS Neural API | ToadStool |
|--------|-------------------|-----------|
| Node ID | `node_id` | `id` |
| Node Type | `node_type` | Stored in metadata |
| Operation | Nested `operation` object | Flat `operation` string |
| Resources | Simplified units (GB, cores) | Precise bytes/units |
| Duration | `duration_secs` in resources | Top-level `duration` field |
| Edge | `source`/`target` | `from`/`to` |
| GPU | `gpu_required` bool + GB | Detailed `GpuRequirements` |
| Primal | Not specified | Required (`toadstool` default) |
| Versioning | `version` field | Inferred from format |

---

## Conversion Traits

### Core Trait Definitions

```rust
/// Trait for converting from biomeOS Neural API format
pub trait FromNeuralApi: Sized {
    type Error: std::error::Error;
    
    fn from_neural_api(value: &NeuralApiGraph) -> Result<Self, Self::Error>;
}

/// Trait for converting to biomeOS Neural API format
pub trait ToNeuralApi {
    type Error: std::error::Error;
    
    fn to_neural_api(&self) -> Result<NeuralApiGraph, Self::Error>;
}

/// biomeOS Neural API graph representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuralApiGraph {
    pub graph_id: String,
    pub graph_type: String,
    pub version: String,
    pub nodes: Vec<NeuralApiNode>,
    pub edges: Vec<NeuralApiEdge>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// biomeOS Neural API node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuralApiNode {
    pub node_id: String,
    pub node_type: String,
    pub operation: NeuralApiOperation,
    pub resources: NeuralApiResources,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// biomeOS Neural API operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuralApiOperation {
    pub op_type: String,
    pub params: HashMap<String, serde_json::Value>,
}

/// biomeOS Neural API resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuralApiResources {
    pub compute: NeuralApiCompute,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_secs: Option<u64>,
}

/// biomeOS Neural API compute resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuralApiCompute {
    pub cpu_cores: f64,
    pub memory_gb: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_memory_gb: Option<u64>,
}

/// biomeOS Neural API edge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuralApiEdge {
    pub source: String,
    pub target: String,
    pub edge_type: String,
    pub metadata: HashMap<String, serde_json::Value>,
}
```

### Conversion Implementation

```rust
use crate::graph_types::{ExecutionGraph, GraphNode, GraphEdge, EdgeType};
use std::time::Duration;

impl FromNeuralApi for ExecutionGraph {
    type Error = NeuralApiConversionError;
    
    fn from_neural_api(neural: &NeuralApiGraph) -> Result<Self, Self::Error> {
        // Validate version compatibility
        if neural.version != "1.0" {
            return Err(NeuralApiConversionError::UnsupportedVersion(neural.version.clone()));
        }
        
        // Convert nodes
        let nodes: Result<Vec<GraphNode>, _> = neural.nodes.iter()
            .map(|n| GraphNode::from_neural_api_node(n))
            .collect();
        let nodes = nodes?;
        
        // Convert edges
        let edges: Result<Vec<GraphEdge>, _> = neural.edges.iter()
            .map(|e| GraphEdge::from_neural_api_edge(e))
            .collect();
        let edges = edges?;
        
        // Preserve original metadata + add source format marker
        let mut metadata = neural.metadata.iter()
            .map(|(k, v)| (k.clone(), v.to_string()))
            .collect::<HashMap<_, _>>();
        metadata.insert("source_format".to_string(), 
                       format!("biomeos_neural_api_v{}", neural.version));
        
        Ok(ExecutionGraph {
            id: neural.graph_id.clone(),
            nodes,
            edges,
            metadata,
        })
    }
}

impl GraphNode {
    fn from_neural_api_node(neural: &NeuralApiNode) -> Result<Self, NeuralApiConversionError> {
        let mut builder = GraphNode::builder(&neural.node_id, &neural.operation.op_type);
        
        // Convert compute resources
        builder = builder
            .cpu(neural.resources.compute.cpu_cores)
            .memory_gb(neural.resources.compute.memory_gb);
        
        // Convert GPU resources if specified
        if neural.resources.compute.gpu_required.unwrap_or(false) {
            if let Some(gpu_gb) = neural.resources.compute.gpu_memory_gb {
                builder = builder.gpu_memory_gb(gpu_gb);
            }
        }
        
        // Convert duration
        if let Some(duration_secs) = neural.resources.duration_secs {
            builder = builder.duration_secs(duration_secs);
        }
        
        // Preserve original metadata
        for (key, value) in &neural.metadata {
            builder = builder.metadata(key, value.to_string());
        }
        
        // Add original node type and operation params to metadata
        builder = builder
            .metadata("biomeos_node_type", &neural.node_type)
            .metadata("biomeos_op_type", &neural.operation.op_type);
        
        for (key, value) in &neural.operation.params {
            builder = builder.metadata(key, value.to_string());
        }
        
        Ok(builder.build())
    }
}

impl GraphEdge {
    fn from_neural_api_edge(neural: &NeuralApiEdge) -> Result<Self, NeuralApiConversionError> {
        // Map edge types
        let edge_type = match neural.edge_type.as_str() {
            "data_dependency" => EdgeType::DataFlow,
            "control_dependency" => EdgeType::Control,
            _ => EdgeType::Dependency,
        };
        
        let mut metadata: HashMap<String, String> = neural.metadata.iter()
            .map(|(k, v)| (k.clone(), v.to_string()))
            .collect();
        
        // Preserve original edge type
        metadata.insert("biomeos_edge_type".to_string(), neural.edge_type.clone());
        
        Ok(GraphEdge {
            from: neural.source.clone(),
            to: neural.target.clone(),
            edge_type,
            metadata,
        })
    }
}

/// Conversion errors
#[derive(Debug, thiserror::Error)]
pub enum NeuralApiConversionError {
    #[error("Unsupported Neural API version: {0}")]
    UnsupportedVersion(String),
    
    #[error("Missing required field: {0}")]
    MissingField(String),
    
    #[error("Invalid resource value: {0}")]
    InvalidResource(String),
    
    #[error("Unknown node type: {0}")]
    UnknownNodeType(String),
}
```

### Format Detection

```rust
/// Auto-detect graph format and convert if necessary
pub fn parse_graph(json: &serde_json::Value) -> Result<ExecutionGraph, Box<dyn std::error::Error>> {
    // Check for Neural API markers
    if json.get("graph_type").and_then(|v| v.as_str()) == Some("neural_api") {
        // Parse as Neural API format
        let neural: NeuralApiGraph = serde_json::from_value(json.clone())?;
        ExecutionGraph::from_neural_api(&neural)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    } else {
        // Parse as ToadStool format
        serde_json::from_value(json.clone())
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }
}
```

---

## Implementation Plan

### Phase 1: Core Types (2 hours)

- [ ] Define `NeuralApiGraph`, `NeuralApiNode`, `NeuralApiEdge` structs
- [ ] Implement `Serialize` and `Deserialize`
- [ ] Define conversion error types

### Phase 2: Conversion Traits (3 hours)

- [ ] Implement `FromNeuralApi` for `ExecutionGraph`
- [ ] Implement node conversion helper
- [ ] Implement edge conversion helper
- [ ] Handle edge cases and validation

### Phase 3: Reverse Conversion (3 hours)

- [ ] Implement `ToNeuralApi` for `ExecutionGraph`
- [ ] Handle metadata extraction
- [ ] Round-trip testing

### Phase 4: Format Detection (1 hour)

- [ ] Implement auto-detection
- [ ] Add to JSON-RPC handlers
- [ ] Update documentation

### Phase 5: Testing (3 hours)

- [ ] Unit tests for each conversion function
- [ ] Round-trip tests
- [ ] Edge case tests
- [ ] Integration tests with estimator

**Total Estimated Time**: 12 hours (1.5 days)

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_neural_api_to_toadstool() {
        let neural = NeuralApiGraph {
            graph_id: "test_graph".to_string(),
            graph_type: "neural_api".to_string(),
            version: "1.0".to_string(),
            nodes: vec![
                NeuralApiNode {
                    node_id: "node1".to_string(),
                    node_type: "computation".to_string(),
                    operation: NeuralApiOperation {
                        op_type: "transform".to_string(),
                        params: HashMap::new(),
                    },
                    resources: NeuralApiResources {
                        compute: NeuralApiCompute {
                            cpu_cores: 4.0,
                            memory_gb: 8,
                            gpu_required: Some(true),
                            gpu_memory_gb: Some(16),
                        },
                        duration_secs: Some(60),
                    },
                    metadata: HashMap::new(),
                },
            ],
            edges: vec![],
            metadata: HashMap::new(),
        };
        
        let toadstool = ExecutionGraph::from_neural_api(&neural).unwrap();
        
        assert_eq!(toadstool.id, "test_graph");
        assert_eq!(toadstool.nodes.len(), 1);
        assert_eq!(toadstool.nodes[0].id, "node1");
        assert!(toadstool.nodes[0].requirements.gpu.is_some());
        assert_eq!(toadstool.nodes[0].duration, Some(Duration::from_secs(60)));
    }
    
    #[test]
    fn test_round_trip_conversion() {
        let neural = create_sample_neural_graph();
        let toadstool = ExecutionGraph::from_neural_api(&neural).unwrap();
        let back_to_neural = toadstool.to_neural_api().unwrap();
        
        // Check key fields preserved
        assert_eq!(neural.graph_id, back_to_neural.graph_id);
        assert_eq!(neural.nodes.len(), back_to_neural.nodes.len());
        assert_eq!(neural.edges.len(), back_to_neural.edges.len());
    }
    
    #[test]
    fn test_format_detection() {
        let neural_json = serde_json::json!({
            "graph_id": "test",
            "graph_type": "neural_api",
            "version": "1.0",
            "nodes": [],
            "edges": []
        });
        
        let graph = parse_graph(&neural_json).unwrap();
        assert_eq!(graph.id, "test");
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_neural_api_with_estimator() {
    let neural = create_complex_neural_graph();
    let toadstool = ExecutionGraph::from_neural_api(&neural).unwrap();
    
    let estimator = ResourceEstimator::new();
    let estimate = estimator.estimate(&toadstool).await.unwrap();
    
    assert!(estimate.cpu_cores > 0.0);
    assert!(estimate.memory_bytes > 0);
}
```

---

## Integration Examples

### JSON-RPC with Auto-Detection

```rust
// In manual_jsonrpc.rs
async fn handle_estimate_resources(&self, request: JsonRpcRequest) -> Value {
    // Parse params
    let params: Value = request.params;
    let graph_json = params.get("graph").ok_or("Missing graph param")?;
    
    // Auto-detect and convert format
    let graph = parse_graph(graph_json)?;
    
    // Continue with estimation
    let estimate = self.estimator.estimate(&graph).await?;
    // ... rest of handler
}
```

### Python Client with Neural API

```python
# biomeOS client can send native format
neural_graph = {
    "graph_id": "ml_pipeline",
    "graph_type": "neural_api",
    "version": "1.0",
    "nodes": [
        {
            "node_id": "train",
            "node_type": "computation",
            "operation": {
                "op_type": "gpu_training",
                "params": {"model": "transformer"}
            },
            "resources": {
                "compute": {
                    "cpu_cores": 16,
                    "memory_gb": 128,
                    "gpu_required": True,
                    "gpu_memory_gb": 80
                },
                "duration_secs": 3600
            },
            "metadata": {}
        }
    ],
    "edges": []
}

# ToadStool automatically converts
client = ToadStoolClient()
estimate = client.call_rpc("resources.estimate", {"graph": neural_graph})
```

---

## Conclusion

The Neural API Adapter provides seamless integration between biomeOS and ToadStool graph formats. Once the actual biomeOS Neural API specification is available, this adapter can be implemented quickly (estimated 1.5 days).

### Benefits

✅ **Transparent**: biomeOS uses native format  
✅ **Type-Safe**: Compile-time validation  
✅ **Fast**: Minimal overhead (<1ms)  
✅ **Robust**: Comprehensive error handling  
✅ **Testable**: Unit and integration tests  
✅ **Documented**: Clear examples and guide

### Next Steps

1. **Obtain biomeOS Neural API Spec**: Coordinate with biomeOS team
2. **Update This Document**: Refine based on actual spec
3. **Implement**: Follow 5-phase plan
4. **Test**: Comprehensive test suite
5. **Document**: Update integration guide

Different orders of the same architecture. 🍄🐸

