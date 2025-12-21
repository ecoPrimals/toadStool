# Level 1: ToadStool Integration

**Goal**: Demonstrate proper workload orchestration via ToadStool

---

## 🎯 What This Demonstrates

### Without ToadStool (Level 0)
```bash
# ❌ Manual process management
openarena +set dedicated 1 +map dm17 &

# Problems:
# - No resource limits
# - No health monitoring
# - No auto-restart
# - Manual cleanup
# - No metrics
```

### With ToadStool (Level 1)
```bash
# ✅ Professional orchestration
toadstool submit biomes/game-server-openarena.yaml

# Benefits:
# • Resource allocation (CPU, memory)
# • Health monitoring (UDP port check)
# • Auto-restart on failure
# • Process lifecycle management
# • Metrics collection
# • Clean shutdown
```

---

## 🚀 Quick Start

### Prerequisites

1. **ToadStool CLI installed**:
   ```bash
   cd ../../..
   cargo install --path crates/cli
   ```

2. **ToadStool server running**:
   ```bash
   cd ../../..
   cargo run --bin toadstool-server
   ```

3. **OpenArena installed**:
   ```bash
   sudo apt install openarena
   ```

### Run the Demo

```bash
cd level-1-toadstool
./run_via_toadstool.sh
```

---

## 📋 The biome.yaml

```yaml
name: openarena-server
description: OpenArena multiplayer game server

orchestration:
  runtime: native
  
  execution:
    command: openarena
    args: ["+set", "dedicated", "1", "+map", "dm17"]
    
  resources:
    cpu_cores: 2
    memory_mb: 512
    
  health_check:
    type: port
    port: 27960
    protocol: udp
    interval_seconds: 30
    
  restart_policy: always
```

See: `biomes/game-server-openarena.yaml`

---

## 🔧 ToadStool Features Used

### 1. Resource Management
```yaml
resources:
  cpu_cores: 2        # Limit to 2 cores
  memory_mb: 512      # Max 512MB RAM
  disk_mb: 100        # 100MB disk quota
```

**What this does**:
- Prevents server from consuming all resources
- Ensures fair sharing on multi-tenant systems
- Protects against memory leaks

### 2. Health Monitoring
```yaml
health_check:
  type: port
  port: 27960
  protocol: udp
  interval_seconds: 30
  retries: 3
```

**What this does**:
- Checks if server is responding every 30s
- Detects crashes or hangs
- Triggers auto-restart if unhealthy

### 3. Restart Policy
```yaml
restart_policy: always
```

**What this does**:
- Server crashes? ToadStool restarts it
- Ensures high availability
- No manual intervention needed

### 4. Monitoring
```yaml
monitoring:
  metrics:
    - cpu_usage
    - memory_usage
    - network_traffic
  logs:
    stdout: true
    stderr: true
```

**What this does**:
- Collects performance metrics
- Captures all output
- Enables debugging and optimization

---

## 📊 Management Commands

### Check Status
```bash
toadstool status <workload-id>
```

### View Logs
```bash
toadstool logs <workload-id>
```

### Stop Server
```bash
toadstool stop <workload-id>
```

### Restart Server
```bash
toadstool restart <workload-id>
```

### List All Workloads
```bash
toadstool list
```

---

## 🎮 Connecting Clients

### Direct Connection
```bash
openarena +connect <server-ip>:27960
```

### Using Join Script
```bash
cd ..
./join_lan_server.sh <server-ip>
```

---

## 🔍 Validation

### Verify ToadStool is Managing
```bash
# Check ToadStool knows about the server
toadstool list | grep openarena

# Verify health checks are running
toadstool status <workload-id> | grep health

# Confirm resource limits are applied
toadstool status <workload-id> | grep -A 5 resources
```

### Test Auto-Restart
```bash
# Kill the game server process
pkill -9 openarena

# Wait 30 seconds
sleep 30

# ToadStool should have restarted it!
toadstool status <workload-id>
# Should show: status: Running
```

---

## 🆚 Comparison: Level 0 vs Level 1

| Feature | Level 0 (Direct) | Level 1 (ToadStool) |
|---------|------------------|---------------------|
| **Launch** | Manual script | biome.yaml |
| **Resources** | Uncontrolled | Allocated & limited |
| **Health** | None | Automatic checks |
| **Restart** | Manual | Automatic |
| **Monitoring** | None | Full metrics |
| **Cleanup** | Manual | Automatic |
| **Management** | CLI tools | ToadStool API |

---

## 🎓 What You Learn

### Concepts Demonstrated

1. **Workload Orchestration**
   - Declarative configuration
   - Lifecycle management
   - Resource allocation

2. **Self-Healing**
   - Health monitoring
   - Auto-restart
   - High availability

3. **Professional Operations**
   - Metrics collection
   - Log aggregation
   - Centralized management

### ecoPrimals Principles

1. **Declarative**: biome.yaml describes desired state
2. **Self-managing**: ToadStool handles lifecycle
3. **Observable**: Metrics and logs built-in
4. **Resilient**: Auto-recovery from failures

---

## 🔗 Integration Points (Future Levels)

### Level 2: Songbird
```yaml
primals:
  songbird:
    enabled: true
    discovery:
      advertise_capabilities:
        - game-server
        - openarena
```

### Level 3: NestGate
```yaml
primals:
  nestgate:
    enabled: true
    storage:
      configs: /games/openarena/configs
```

### Level 4: BearDog
```yaml
primals:
  beardog:
    enabled: true
    encryption:
      config_files: true
```

---

## 🐛 Troubleshooting

### Server Won't Start

```bash
# Check ToadStool logs
toadstool logs <workload-id>

# Common issues:
# - OpenArena not installed: sudo apt install openarena
# - Port 27960 in use: netstat -an | grep 27960
# - Config errors: Check ~/.openarena/baseoa/server.cfg
```

### Health Checks Failing

```bash
# Verify port is accessible
nc -u -z localhost 27960

# Check server is responding
toadstool logs <workload-id> | grep "Listening"

# Increase retry count in biome.yaml if needed
```

### Resource Limits Too Low

```bash
# Edit biome.yaml:
resources:
  cpu_cores: 4      # Increase cores
  memory_mb: 1024   # Increase RAM

# Resubmit:
toadstool submit biomes/game-server-openarena.yaml
```

---

## ✅ Success Criteria

- [ ] Server launches via ToadStool
- [ ] Health checks pass
- [ ] Resources are limited
- [ ] Auto-restart works
- [ ] Metrics are collected
- [ ] Logs are captured
- [ ] Clients can connect

---

## 🚀 Next Steps

After completing Level 1:

1. **Level 2**: Add Songbird discovery
2. **Level 3**: Integrate NestGate storage
3. **Level 4**: Enable BearDog security
4. **Level 5**: Complete ecosystem integration

---

**Status**: Level 1 demonstrates proper ToadStool orchestration! 🍄✨


