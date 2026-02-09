# Songbird + ToadStool Integration Showcase

Demonstrates distributed workload coordination across multiple towers.

## What It Shows

1. **Tower Discovery**: Songbird discovers ToadStool towers by capability
2. **Workload Distribution**: Splits training job across available resources
3. **Parallel Execution**: All towers execute simultaneously
4. **Result Aggregation**: Songbird collects and combines results
5. **Fault Tolerance**: System handles tower failures gracefully

## Running the Showcase

### With Songbird Installed
```bash
export SONGBIRD_ENDPOINT=http://localhost:8080
cargo run --bin songbird-distributed-coordination
```

### Demonstration Mode (No Songbird)
```bash
cargo run --bin songbird-distributed-coordination
# Automatically falls back to demonstration
```

## Architecture

```
Songbird Orchestrator
    ↓ (discovers)
ToadStool Tower 1 (2 GPUs)
ToadStool Tower 2 (4 GPUs) ← capability-based
ToadStool Tower 3 (2 GPUs)
    ↓ (distributes)
Training Job Split
    ↓ (parallel execution)
Results Aggregation
```

## Benefits

- **8x Speedup**: Through parallel execution
- **Auto-Discovery**: No manual configuration
- **Fault Tolerance**: Towers can fail/rejoin
- **Optimal Scheduling**: Resources allocated by capability
- **Zero Lock-in**: Capability-based, not vendor-specific

