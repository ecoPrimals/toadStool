# 🐦 Songbird Integration - Capability-Based Discovery
## December 19, 2025

## 🎯 Goal

Implement production-ready Songbird discovery integration using **capability-based discovery** while maintaining **self-knowledge principles**.

---

## 🧠 Philosophy

```
ToadStool knows:
  ✅ What IT is (compute platform)
  ✅ What IT can do (capabilities)
  ✅ What IT needs (service-discovery, load-balancing)

ToadStool DOES NOT know:
  ❌ That "Songbird" exists as a service name
  ❌ Where Songbird is running
  ❌ What port Songbird uses

Instead:
  ✅ ToadStool asks: "Who provides 'service-discovery'?"
  ✅ Discovery engine responds: "Here are 3 services with that capability"
  ✅ ToadStool connects to best available service
```

---

## 📊 Current State

### What Exists ✅
1. **Capability System** - `primal-capabilities.toml` defines all capabilities
2. **Discovery Engine** - `infant_discovery` module for runtime discovery
3. **Songbird Types** - Complete type system in `distributed/songbird_integration/types.rs`
4. **HTTP Protocol** - Working HTTP integration (95% production ready)
5. **Reference Implementation** - `capability_discovery.rs` (commented out)

### What's Needed 🔨
1. **Active Integration** - Un-comment and enhance capability discovery
2. **ToadStool API** - Simple API for ToadStool to use
3. **Examples** - Show how to use it
4. **Tests** - Verify it works
5. **Documentation** - Update integration guide

---

## 🏗️ Implementation Plan

### Phase 1: Enable Capability Discovery (30 min)

```rust
// File: crates/distributed/src/songbird_integration/mod.rs
pub mod capability_discovery;  // ✅ Un-comment this line

// Re-export for easy use
pub use capability_discovery::SongbirdConnection;
```

### Phase 2: Create Simple ToadStool API (1 hr)

```rust
// File: crates/core/toadstool/src/discovery/songbird.rs
use toadstool_distributed::songbird_integration::SongbirdConnection;
use toadstool_common::infant_discovery::DiscoveryEngine;

/// Discover and connect to Songbird-compatible services
///
/// **Self-Knowledge**: ToadStool knows it needs orchestration,
/// but doesn't know "Songbird" by name.
pub async fn discover_orchestration_service() -> Result<SongbirdConnection> {
    let discovery = DiscoveryEngine::new();
    
    // Discover by capabilities, not by name!
    SongbirdConnection::discover(
        Arc::new(discovery),
        vec![
            "service-discovery".to_string(),
            "load-balancing".to_string(),
        ],
    ).await
}

/// Submit a distributed job using capability-based discovery
pub async fn submit_distributed_job(job: UniversalJob) -> Result<JobResult> {
    // Discover orchestration service
    let connection = discover_orchestration_service().await?;
    
    // Submit job with automatic failover
    connection.execute_with_failover(|service| async move {
        // Make HTTP request to discovered service
        submit_job_to_endpoint(&service.endpoint, &job).await
    }).await
}
```

### Phase 3: Update Distributed Scheduler (1 hr)

```rust
// File: crates/runtime/gpu/src/distributed/mod.rs

impl DistributedGpuScheduler {
    /// Execute workload with Songbird coordination
    ///
    /// **Capability-Based**: Discovers orchestration services at runtime
    pub async fn execute_with_coordination(
        &self,
        workload: GpuWorkload,
    ) -> ToadStoolResult<ExecutionResult> {
        // Discover orchestration service (not "Songbird"!)
        let orchestration = toadstool::discovery::songbird::discover_orchestration_service().await?;
        
        // Check if we should distribute or execute locally
        if workload.requires_distribution() {
            // Submit to discovered orchestration service
            orchestration.execute_with_failover(|service| async move {
                self.submit_distributed_workload(&service, workload.clone()).await
            }).await
        } else {
            // Execute locally
            self.execute_local(workload).await
        }
    }
}
```

### Phase 4: Environment Override Support (30 min)

```rust
// Already implemented in Phase 2!
// Uses environment variables from ports.rs:
//   SONGBIRD_ENDPOINT=http://prod:8082
```

### Phase 5: Examples & Tests (1 hr)

```rust
// File: examples/songbird_discovery_demo.rs
use toadstool::discovery::songbird::discover_orchestration_service;

#[tokio::main]
async fn main() -> Result<()> {
    // Discover orchestration service by capability
    let connection = discover_orchestration_service().await?;
    
    // Get available services
    let services = connection.get_available_services().await?;
    
    println!("✅ Discovered {} orchestration services:", services.len());
    for service in services {
        println!("  - {} ({})", service.endpoint, service.metadata.health);
    }
    
    Ok(())
}
```

---

## 🎯 Success Criteria

### Must Have ✅
- [x] Discovery by capability (not by name)
- [x] Environment variable overrides work
- [x] Automatic failover between services
- [x] Self-knowledge principle maintained
- [ ] Simple API for ToadStool to use
- [ ] Working examples
- [ ] Tests passing

### Nice to Have 🟡
- [ ] Health-based service selection
- [ ] Caching with TTL (5 minutes)
- [ ] Metrics and monitoring
- [ ] Advanced load balancing

---

## 📚 Dev Knowledge vs Self-Knowledge

### Development Knowledge (We Have) 📖
From `primal-capabilities.toml`:
```toml
[primals.songbird]
name = "songbird"
description = "Service discovery, load balancing, and request routing"
primary_role = "coordination"

capabilities = [
    "service-discovery",
    "load-balancing",
    "request-routing",
    "failover",
]

protocols = ["http", "grpc", "mdns"]
default_port = 8082
```

**This lives in the TOML file, NOT in code**

### ToadStool Self-Knowledge (Code Has) 🧠
From `crates/core/toadstool/src/self_identity.rs`:
```rust
pub struct SelfIdentity {
    pub primal_type: "toadstool",  // We know what WE are
    pub capabilities: [            // We know what WE do
        "universal-compute",
        "gpu-compute",
        // ...
    ],
    pub requirements: [            // We know what WE need
        CapabilityRequirement {
            name: "service-discovery",  // ✅ By capability!
            min_version: "1.0",
        },
    ],
}
```

**No "Songbird" mentioned in code!**

---

## 🔄 Discovery Flow

```
1. ToadStool Startup
   ↓
2. Load Self-Knowledge
   - "I am: toadstool"
   - "I provide: compute"
   - "I need: service-discovery"
   ↓
3. Discovery Engine Queries
   - Check: SONGBIRD_ENDPOINT env var
   - Try: mDNS discovery
   - Try: DNS-SD
   - Fallback: primal-capabilities.toml
   ↓
4. Find Services with "service-discovery" Capability
   - Service A: http://192.168.1.5:8082 (Healthy)
   - Service B: http://10.0.0.3:8082 (Unknown)
   - Service C: http://prod:8082 (Healthy)
   ↓
5. Select Best Service
   - Prefer healthy
   - Prefer local
   - Prefer http protocol
   ↓
6. Connect and Use
   - Cache connection for 5 minutes
   - Auto-failover if fails
   - No service name hardcoded anywhere!
```

---

## 🎨 Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      ToadStool                              │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │          Self-Knowledge                              │  │
│  │  - primal_type: "toadstool"                         │  │
│  │  - capabilities: ["compute", "gpu"]                 │  │
│  │  - requirements: ["service-discovery"]              │  │
│  └──────────────────────────────────────────────────────┘  │
│                          │                                  │
│                          ↓                                  │
│  ┌──────────────────────────────────────────────────────┐  │
│  │      Capability Discovery API                        │  │
│  │  discover_orchestration_service()                    │  │
│  │  submit_distributed_job()                            │  │
│  └──────────────────────────────────────────────────────┘  │
│                          │                                  │
└──────────────────────────┼──────────────────────────────────┘
                           │
                           ↓
┌──────────────────────────────────────────────────────────────┐
│              Discovery Engine (Runtime)                      │
│                                                              │
│  1. Environment Variables                                    │
│     SONGBIRD_ENDPOINT=http://prod:8082                       │
│                                                              │
│  2. mDNS Discovery                                           │
│     _service-discovery._tcp.local                            │
│                                                              │
│  3. DNS-SD                                                   │
│     service-discovery.services.local                         │
│                                                              │
│  4. primal-capabilities.toml (Fallback)                      │
│     primals.songbird.capabilities                            │
└──────────────────────────────────────────────────────────────┘
                           │
                           ↓
┌──────────────────────────────────────────────────────────────┐
│          Discovered Services (Runtime)                       │
│                                                              │
│  Service A: http://192.168.1.5:8082                          │
│    capabilities: [service-discovery, load-balancing]         │
│    health: Healthy                                           │
│    protocols: [http, grpc]                                   │
│                                                              │
│  Service B: http://prod.example.com:8082                     │
│    capabilities: [service-discovery, load-balancing]         │
│    health: Healthy                                           │
│    protocols: [http]                                         │
└──────────────────────────────────────────────────────────────┘
                           │
                           ↓
                    ┌──────────────┐
                    │   Songbird   │
                    │  (or equiv)  │
                    └──────────────┘
```

---

## 💡 Key Insights

### 1. Self-Knowledge is in Code ✅
```rust
// ToadStool knows itself
pub const PRIMAL_TYPE: &str = "toadstool";
pub const CAPABILITIES: &[&str] = &["compute", "gpu"];
```

### 2. Other-Knowledge is in Config 📋
```toml
# primal-capabilities.toml (DEV knowledge, not hardcoded)
[primals.songbird]
capabilities = ["service-discovery"]
```

### 3. Discovery is at Runtime 🔍
```rust
// No "Songbird" in code!
let services = discovery.find_by_capability("service-discovery").await?;
// Discovers ANY service with that capability
```

### 4. Environment Trumps All 🌍
```bash
# Production override
export SONGBIRD_ENDPOINT=https://songbird.prod.example.com:8082

# ToadStool finds it immediately, no discovery needed
```

---

## 🚀 Timeline

### Immediate (2 hours)
- [ ] Un-comment capability_discovery in mod.rs
- [ ] Create simple ToadStool API
- [ ] Update distributed scheduler
- [ ] Add basic example
- [ ] Test manually

### Next Session (2 hours)
- [ ] Add comprehensive tests
- [ ] Add metrics/monitoring
- [ ] Update documentation
- [ ] Add integration guide

### Future (Optional)
- [ ] Advanced load balancing
- [ ] Health-based selection
- [ ] Performance optimization

---

## ✅ Completion Checklist

- [ ] capability_discovery.rs un-commented
- [ ] ToadStool API created
- [ ] Distributed scheduler updated
- [ ] Example working
- [ ] Tests passing
- [ ] Documentation updated
- [ ] Self-knowledge principle maintained
- [ ] No "Songbird" hardcoded in code
- [ ] Environment overrides working

---

**Status**: Ready to implement  
**Time**: 2-4 hours  
**Priority**: HIGH  
**Blocker**: None

Let's proceed! 🚀

