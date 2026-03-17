# Resource Management

ToadStool showcase demo: resource estimation, availability checking, and optimization suggestions for compute workloads.

## What It Demonstrates

- **System Resources**: CPU count, memory info, disk usage, load average via `toadstool-sysmon`
- **Resource Estimation**: JSON workload requirements (cpu_cores, memory_gb, gpu_memory_mb, estimated_duration_secs)
- **Availability Check**: Compare requirements against actual system resources; pass/fail for CPU and memory
- **Optimization Suggestions**: Context-aware hints based on load, memory pressure, and CPU allocation
- **Summary**: Resource assessment with N/M checks passed

## How to Run

```bash
./demo.sh
```

Or manually:

```bash
cargo build --release
cargo run --release
```

## Expected Output

- Banner: "ToadStool Showcase: Resource Management"
- System Resources: CPU cores, memory total/available/used, load average, disk
- Resource Estimation: JSON object with workload requirements
- Availability Check: pass/fail for CPU and memory; GPU noted as requiring probe
- Optimization Suggestions: load-based, memory-pressure, CPU nominal, GPU probe hint
- Summary: "Resource assessment complete — N/M checks passed"

## Prerequisites

- Rust 1.85+
- Linux (sysmon reads `/proc`)

## ToadStool Capabilities Shown

| Crate | Capability |
|-------|------------|
| toadstool-sysmon | cpu_count, memory_info, disk_usage, load_average |
