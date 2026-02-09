# NestGate + ToadStool Integration Showcase

Demonstrates persistent workload results using distributed storage.

## What It Shows

1. **Workload Execution**: ToadStool executes compute-intensive workload
2. **Result Storage**: Results automatically stored in NestGate
3. **Data Persistence**: Results survive across sessions
4. **Retrieval**: Later workloads can retrieve previous results
5. **Sovereignty**: All data remains self-hosted

## Running the Showcase

### With NestGate Installed
```bash
cargo run --bin nestgate-persistent-results
```

### Demonstration Mode (No NestGate)
```bash
cargo run --bin nestgate-persistent-results
# Automatically falls back to demonstration
```

## Architecture

```
ToadStool Workload
    ↓ (compute)
Result Data (4MB)
    ↓ (store)
NestGate Storage
    ├─ Node 1 (primary)
    ├─ Node 2 (replica)
    └─ Node 3 (replica)
    ↓ (retrieve)
New ToadStool Workload
```

## Benefits

- **Zero Data Loss**: Replicated storage
- **Fast Access**: Distributed across nodes
- **Sovereignty**: Self-hosted, no cloud
- **Integration**: Seamless ToadStool ↔ NestGate

