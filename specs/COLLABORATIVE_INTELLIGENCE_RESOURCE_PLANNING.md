# ToadStool Resource Planning API Specification

**Version**: 1.0.0  
**Date**: January 11, 2026  
**Status**: Draft → Implementation  
**For**: biomeOS Collaborative Intelligence Integration

---

## Overview

This specification defines ToadStool's resource planning API for biomeOS's collaborative intelligence system. These methods enable human-AI collaboration by providing real-time resource estimation, availability validation, and optimization suggestions for workflow graphs.

### Vision

> "Human and AI collaborate as equals" - biomeOS

ToadStool provides the **resource intelligence** that enables:
- Users to understand resource requirements before deployment
- AI to validate feasibility of proposed workflows
- System to suggest optimizations for better resource utilization

---

## API Methods

### 1. resources.estimate

**Purpose**: Estimate total resource requirements for executing a workflow graph.

**Method**: `resources.estimate`

**Parameters**:
```typescript
{
  graph: ExecutionGraph  // Workflow graph structure
}
```

**Returns**:
```typescript
{
  total_cpu_cores: number,           // Total CPU cores needed
  total_memory_bytes: number,        // Total memory in bytes
  total_storage_bytes: number,       // Total storage in bytes
  total_gpu_count: number,           // Total GPUs needed
  estimated_duration_secs?: number,  // Estimated execution time
  parallelism_factor: number,        // 1.0 = sequential, >1.0 = parallel
  confidence: number,                // 0.0-1.0 confidence in estimate
  per_node_estimates: {              // Per-node breakdown
    [node_id: string]: {
      cpu_cores: number,
      memory_bytes: number,
      storage_bytes: number,
      gpu_count: number,
      estimated_duration_secs?: number
    }
  },
  coordination_overhead: {           // Multi-node coordination overhead
    cpu_cores: number,
    memory_bytes: number,
    percentage: number               // % of total resources
  }
}
```

**Example Request**:
```json
{
  "jsonrpc": "2.0",
  "method": "resources.estimate",
  "params": {
    "graph": {
      "id": "workflow-123",
      "nodes": [
        {
          "id": "node-1",
          "primal": "toadstool",
          "operation": "gpu_compute",
          "requirements": {
            "cpu": { "min_cores": 4 },
            "memory": { "min_bytes": 8589934592 },
            "gpu": { "min_units": 1 }
          },
          "metadata": {
            "workload_type": "neural_inference",
            "model_size": "7B"
          }
        },
        {
          "id": "node-2",
          "primal": "toadstool",
          "operation": "cpu_compute",
          "requirements": {
            "cpu": { "min_cores": 8 },
            "memory": { "min_bytes": 4294967296 }
          }
        }
      ],
      "edges": [
        {
          "from": "node-1",
          "to": "node-2",
          "edge_type": "DataFlow"
        }
      ]
    }
  },
  "id": 1
}
```

**Example Response**:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "total_cpu_cores": 12.0,
    "total_memory_bytes": 12884901888,
    "total_storage_bytes": 0,
    "total_gpu_count": 1,
    "estimated_duration_secs": 180.0,
    "parallelism_factor": 1.0,
    "confidence": 0.85,
    "per_node_estimates": {
      "node-1": {
        "cpu_cores": 4.0,
        "memory_bytes": 8589934592,
        "storage_bytes": 0,
        "gpu_count": 1,
        "estimated_duration_secs": 120.0
      },
      "node-2": {
        "cpu_cores": 8.0,
        "memory_bytes": 4294967296,
        "storage_bytes": 0,
        "gpu_count": 0,
        "estimated_duration_secs": 60.0
      }
    },
    "coordination_overhead": {
      "cpu_cores": 0.5,
      "memory_bytes": 268435456,
      "percentage": 4.0
    }
  },
  "id": 1
}
```

**Estimation Algorithm**:
1. Parse graph topology (topological sort)
2. Identify parallel execution stages
3. Estimate per-node resources from requirements + metadata
4. Aggregate resources considering parallelism:
   - Sequential nodes: SUM resources
   - Parallel nodes: MAX resources
5. Add coordination overhead (10-20% for distributed execution)
6. Calculate duration estimates if metadata provides hints

**Error Cases**:
- `-32602` Invalid params: Malformed graph structure
- `-32603` Internal error: Estimation algorithm failure

---

### 2. resources.validate_availability

**Purpose**: Check if current system has sufficient resources to execute the graph.

**Method**: `resources.validate_availability`

**Parameters**:
```typescript
{
  graph: ExecutionGraph  // Workflow graph structure
}
```

**Returns**:
```typescript
{
  available: boolean,                // True if resources sufficient
  missing_resources: ResourceGap[],  // What's missing (if any)
  warnings: string[],                // Non-blocking warnings
  confidence: number,                // 0.0-1.0 confidence in validation
  current_capacity: {                // Current system state
    total_cpu_cores: number,
    available_cpu_cores: number,
    total_memory_bytes: number,
    available_memory_bytes: number,
    total_gpu_count: number,
    available_gpu_count: number
  },
  required_resources: {              // What graph needs
    cpu_cores: number,
    memory_bytes: number,
    storage_bytes: number,
    gpu_count: number
  }
}

type ResourceGap = {
  resource_type: string,   // "cpu", "memory", "gpu", "storage"
  required: number,        // What's needed
  available: number,       // What's available
  shortfall: number,       // Deficit amount
  severity: "critical" | "warning"  // Impact level
}
```

**Example Request**:
```json
{
  "jsonrpc": "2.0",
  "method": "resources.validate_availability",
  "params": {
    "graph": {
      "id": "workflow-123",
      "nodes": [ /* same as estimate example */ ],
      "edges": [ /* same as estimate example */ ]
    }
  },
  "id": 2
}
```

**Example Response (Sufficient Resources)**:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "available": true,
    "missing_resources": [],
    "warnings": [
      "GPU memory utilization will be 85% - close to capacity"
    ],
    "confidence": 0.95,
    "current_capacity": {
      "total_cpu_cores": 128,
      "available_cpu_cores": 120,
      "total_memory_bytes": 274877906944,
      "available_memory_bytes": 250000000000,
      "total_gpu_count": 2,
      "available_gpu_count": 2
    },
    "required_resources": {
      "cpu_cores": 12.0,
      "memory_bytes": 12884901888,
      "storage_bytes": 0,
      "gpu_count": 1
    }
  },
  "id": 2
}
```

**Example Response (Insufficient Resources)**:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "available": false,
    "missing_resources": [
      {
        "resource_type": "gpu",
        "required": 3,
        "available": 2,
        "shortfall": 1,
        "severity": "critical"
      },
      {
        "resource_type": "memory",
        "required": 68719476736,
        "available": 25000000000,
        "shortfall": 43719476736,
        "severity": "critical"
      }
    ],
    "warnings": [],
    "confidence": 0.90,
    "current_capacity": {
      "total_cpu_cores": 128,
      "available_cpu_cores": 120,
      "total_memory_bytes": 274877906944,
      "available_memory_bytes": 25000000000,
      "total_gpu_count": 2,
      "available_gpu_count": 2
    },
    "required_resources": {
      "cpu_cores": 12.0,
      "memory_bytes": 68719476736,
      "storage_bytes": 0,
      "gpu_count": 3
    }
  },
  "id": 2
}
```

**Validation Algorithm**:
1. Call `resources.estimate(graph)` to get requirements
2. Query current system capacity (via StandaloneExecutor)
3. Compare required vs available for each resource type
4. Generate warnings for resources >80% utilization
5. Return detailed gaps for any insufficient resources

**Error Cases**:
- `-32602` Invalid params: Malformed graph structure
- `-32603` Internal error: System query failure

---

### 3. resources.suggest_optimizations

**Purpose**: Suggest ways to optimize the graph for better resource utilization.

**Method**: `resources.suggest_optimizations`

**Parameters**:
```typescript
{
  graph: ExecutionGraph  // Workflow graph structure
}
```

**Returns**:
```typescript
{
  suggestions: OptimizationSuggestion[],
  bottlenecks: Bottleneck[],
  parallelization_opportunities: ParallelizationOpportunity[]
}

type OptimizationSuggestion = {
  id: string,
  type: "increase_parallelism" | "reduce_memory" | "add_gpu" | 
        "reorder_nodes" | "split_workload" | "merge_nodes",
  priority: "high" | "medium" | "low",
  description: string,              // Human-readable explanation
  reasoning: string,                // Why this is suggested
  expected_improvement: number,     // % improvement (e.g., 1.5 = 50% better)
  confidence: number,               // 0.0-1.0 confidence
  affected_nodes: string[],         // Which nodes this affects
  implementation_hint: string       // How to apply this suggestion
}

type Bottleneck = {
  node_id: string,
  resource_type: string,  // "cpu", "memory", "gpu", "network"
  severity: number,       // 0.0-1.0 (1.0 = critical bottleneck)
  impact: string          // Description of impact
}

type ParallelizationOpportunity = {
  nodes: string[],              // Nodes that can run in parallel
  current_duration: number,     // Current estimated duration
  parallel_duration: number,    // Duration if parallelized
  speedup_factor: number,       // Improvement factor
  requirements_increase: {      // Additional resources needed
    cpu_cores: number,
    memory_bytes: number
  }
}
```

**Example Request**:
```json
{
  "jsonrpc": "2.0",
  "method": "resources.suggest_optimizations",
  "params": {
    "graph": {
      "id": "workflow-123",
      "nodes": [ /* same as estimate example */ ],
      "edges": [ /* same as estimate example */ ]
    }
  },
  "id": 3
}
```

**Example Response**:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "suggestions": [
      {
        "id": "opt-1",
        "type": "add_gpu",
        "priority": "high",
        "description": "Move node-2 computation to GPU",
        "reasoning": "Node-2 performs parallel operations that are GPU-friendly. Moving to GPU could provide 3-5x speedup.",
        "expected_improvement": 3.0,
        "confidence": 0.85,
        "affected_nodes": ["node-2"],
        "implementation_hint": "Change node-2 operation from 'cpu_compute' to 'gpu_compute' and add GPU requirement"
      },
      {
        "id": "opt-2",
        "type": "reduce_memory",
        "priority": "medium",
        "description": "Reduce memory allocation for node-1",
        "reasoning": "Node-1 requests 8GB but typical workloads of this type use 4-6GB. Reducing allocation would free resources.",
        "expected_improvement": 1.2,
        "confidence": 0.70,
        "affected_nodes": ["node-1"],
        "implementation_hint": "Reduce memory requirement from 8GB to 6GB"
      }
    ],
    "bottlenecks": [
      {
        "node_id": "node-2",
        "resource_type": "cpu",
        "severity": 0.75,
        "impact": "Node-2 CPU computation is the critical path, consuming 60% of total execution time"
      }
    ],
    "parallelization_opportunities": []
  },
  "id": 3
}
```

**Optimization Algorithm**:
1. Analyze graph topology for parallelization opportunities
2. Identify resource bottlenecks (nodes using >50% of total resources)
3. Match node characteristics against optimization patterns:
   - GPU-friendly operations (matrix ops, convolutions)
   - Memory over-allocation (requested >> typical usage)
   - Sequential nodes that could be parallel
4. Generate suggestions with reasoning and confidence scores
5. Rank by expected improvement * confidence

**Error Cases**:
- `-32602` Invalid params: Malformed graph structure
- `-32603` Internal error: Optimization analysis failure

---

## Data Types

### ExecutionGraph

```typescript
type ExecutionGraph = {
  id: string,
  nodes: GraphNode[],
  edges: GraphEdge[],
  metadata?: Record<string, string>
}

type GraphNode = {
  id: string,
  primal: string,        // e.g., "toadstool", "squirrel", "nestgate"
  operation: string,     // e.g., "gpu_compute", "cpu_compute", "storage"
  requirements: {
    cpu?: {
      min_cores: number,
      max_cores?: number,
      architecture?: string
    },
    memory?: {
      min_bytes: number,
      max_bytes?: number
    },
    storage?: {
      min_bytes: number,
      max_bytes?: number,
      storage_type?: string
    },
    gpu?: {
      min_units: number,
      max_units?: number,
      gpu_type?: string,
      min_memory_bytes?: number
    },
    network?: {
      bandwidth_mbps?: number,
      latency_ms?: number
    }
  },
  metadata?: Record<string, string>
}

type GraphEdge = {
  from: string,   // source node_id
  to: string,     // target node_id
  edge_type: "DataFlow" | "Control" | "Dependency",
  metadata?: Record<string, string>
}
```

---

## Implementation Notes

### Performance

- **Estimation**: O(N + E) where N=nodes, E=edges (topological sort)
- **Validation**: O(N) + system query overhead (~10ms)
- **Optimization**: O(N²) worst case (pairwise parallelization analysis)

All operations should complete in <100ms for graphs with <100 nodes.

### Caching

ToadStool may cache:
- System capacity queries (refresh every 5 seconds)
- Graph topology analysis (cache by graph hash)
- Optimization suggestions (cache by graph structure)

### Concurrency

All methods are thread-safe and can be called concurrently. System capacity queries use read locks.

### Error Handling

All methods return JSON-RPC 2.0 compliant error responses:
- `-32700` Parse error: Invalid JSON
- `-32600` Invalid request: Malformed JSON-RPC
- `-32601` Method not found: Unknown method
- `-32602` Invalid params: Malformed parameters
- `-32603` Internal error: Server error

---

## Security Considerations

### Resource Limits

- Maximum graph size: 1000 nodes, 10000 edges
- Maximum estimation time: 5 seconds
- Maximum validation time: 2 seconds
- Maximum optimization time: 10 seconds

Exceeding limits returns `-32603` Internal error with timeout message.

### Authorization

ToadStool itself does not enforce authorization. biomeOS/BearDog should:
1. Validate user permissions before calling ToadStool
2. Sanitize graph inputs (prevent injection attacks)
3. Rate limit requests (prevent DoS)

### Information Disclosure

These methods reveal:
- System capacity (CPU cores, memory, GPU count)
- Current utilization levels
- Workload characteristics

Ensure appropriate access controls at the biomeOS layer.

---

## Testing Strategy

### Unit Tests

1. **Graph Parsing**
   - Valid graphs
   - Invalid graphs (missing fields, cycles)
   - Edge cases (empty graph, single node)

2. **Estimation Logic**
   - Simple sequential graphs
   - Complex parallel graphs
   - Mixed parallel/sequential
   - Various resource types

3. **Validation Logic**
   - Sufficient resources
   - Insufficient resources (each type)
   - Borderline cases (90-110% utilization)

4. **Optimization Logic**
   - Bottleneck detection
   - Parallelization opportunities
   - GPU acceleration suggestions

### Integration Tests

1. **JSON-RPC E2E**
   - Full request/response cycle
   - Error handling
   - Invalid graphs
   - Large graphs (performance)

2. **Cross-Component**
   - Estimate → Validate → Optimize workflow
   - Real system resource queries
   - Multi-instance coordination

### Load Testing

- 100 concurrent requests
- Large graphs (100+ nodes)
- Sustained load (1000 requests/minute)

Target: <100ms p99 latency for graphs <100 nodes

---

## Migration & Compatibility

### Version 1.0.0 (This Spec)

Initial release. No backward compatibility concerns.

### Future Versions

Breaking changes will increment major version (2.0.0).
New optional fields increment minor version (1.1.0).

---

## References

- [biomeOS Collaborative Intelligence Spec](../specs/COLLABORATIVE_INTELLIGENCE_SPEC.md) (if exists)
- [ToadStool Resource Types](../crates/core/toadstool/src/resources.rs)
- [JSON-RPC 2.0 Specification](https://www.jsonrpc.org/specification)
- [Graph Theory - Topological Sort](https://en.wikipedia.org/wiki/Topological_sorting)

---

## Changelog

### 2026-01-11 - Version 1.0.0

- Initial specification
- Defined 3 core methods (estimate, validate, optimize)
- Defined data types
- Documented algorithms and error handling

---

**Status**: Draft → Implementation  
**Next Review**: 2026-01-25 (after implementation)

Different orders of the same architecture. 🍄🐸

