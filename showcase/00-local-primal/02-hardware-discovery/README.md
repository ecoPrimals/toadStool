# Hardware Discovery

ToadStool showcase demo: hardware substrate probing (CPU, memory, disk, network, load).

## What It Demonstrates

- **CPU Substrate**: Core count, brand, per-CPU usage snapshot
- **Memory Substrate**: Total, available, used memory in human-readable GB
- **Disk Substrate**: Mount points with total/used/available space
- **Network Substrate**: Interfaces with rx/tx bytes
- **Load Average**: 1/5/15 minute load

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

- Banner: "ToadStool Showcase: Hardware Discovery"
- CPU: cores, brand, per-CPU usage (first 8)
- Memory: total/available/used in GB
- Disk: list of mount points with space info
- Network: interfaces with rx/tx bytes
- Load: 1/5/15 min
- Summary: count of substrates discovered

## Prerequisites

- Rust 1.85+
- Linux (sysmon reads `/proc`, disk uses `statvfs`)

## ToadStool Capabilities Shown

| Crate | Capability |
|-------|------------|
| toadstool-sysmon | cpu_count, cpu_brand, per_cpu_usage, memory_info, disk_usage, network_stats, load_average |
