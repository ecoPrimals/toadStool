# 🎯 Capability-Based Discovery Deployment Guide
**ToadStool Philosophy in Practice**  
**Status**: Production-Ready ✅

---

## 🌟 Philosophy: "Know Yourself, Discover Others at Runtime"

**Core Principle**: Each primal knows only itself. Other primals are discovered at runtime via capabilities, never through hardcoded addresses or identities.

### What This Means:
- ✅ **Self-Knowledge**: Each instance knows its own capabilities
- ✅ **Runtime Discovery**: Services found via mDNS/DNS-SD by capability
- ✅ **No Hardcoding**: Zero hardcoded peer addresses or ports
- ✅ **Capability-Based**: Request "compute", not "connect to 192.168.1.100"
- ✅ **Agnostic**: Works on any network, any deployment

---

## 🚀 Quick Start (5 Minutes)

### 1. Basic Self-Identity
```rust
use toadstool::self_identity::SelfIdentity;

// Know yourself - this is ALL you need to know at startup
let identity = SelfIdentity::new()
    .with_network(
        "toadstool-01".to_string(),
        Some(8084),
        vec!["http".to_string()]
    );

// Your capabilities are auto-detected:
// - CPU cores, memory, GPU availability
// - What you CAN do (compute, orchestration, byob)
// - What you NEED from others (optional capabilities)
```

### 2. Advertise Capabilities
```rust
use toadstool::discovery::MdnsDiscoveryService;

// Broadcast WHAT you can do, not WHO you are
let mdns = MdnsDiscoveryService::new()?;
mdns.advertise(&identity)?;

// Others will discover you by capability, not by address!
```

### 3. Discover Others by Capability
```rust
// Find services by WHAT they can do
let storage_services = mdns
    .discover_by_capability("storage", Duration::from_secs(5))
    .await?;

// No hardcoded addresses, no configuration files!
for service in storage_services {
    println!("Found storage at: {}", service.endpoint);
    // Connect dynamically at runtime
}
```

---

## 📋 Complete Deployment Example

### Scenario: Deploy ToadStool + Nestgate + Songbird

**Traditional (Hardcoded) Approach** ❌:
```yaml
# Bad: Hardcoded addresses
toadstool:
  nestgate_url: "http://192.168.1.100:8082"
  songbird_url: "http://192.168.1.101:9080"
  
# Problems:
# - Breaks if IPs change
# - Requires configuration files
# - Tightly coupled
# - Not portable
```

**ToadStool (Capability-Based) Approach** ✅:
```rust
// Good: No hardcoded addresses!

// Step 1: Know yourself
let identity = SelfIdentity::new()
    .with_network(hostname(), Some(8084), vec!["http"]);

// Step 2: Advertise what you can do
let mdns = MdnsDiscoveryService::new()?;
mdns.advertise(&identity)?;

// Step 3: Discover others by capability when needed
async fn get_storage() -> Result<StorageClient> {
    let services = mdns
        .discover_by_capability("storage", Duration::from_secs(5))
        .await?;
    
    // Pick first available (or implement selection logic)
    let service = services.first()
        .ok_or("No storage service available")?;
    
    StorageClient::connect(&service.endpoint).await
}

async fn get_coordinator() -> Result<CoordinatorClient> {
    let services = mdns
        .discover_by_capability("coordination", Duration::from_secs(5))
        .await?;
    
    let service = services.first()
        .ok_or("No coordination service available")?;
    
    CoordinatorClient::connect(&service.endpoint).await
}

// Benefits:
// ✅ Works on any network
// ✅ Services can move freely
// ✅ No configuration files needed
// ✅ Self-healing if services restart
// ✅ Portable across environments
```

---

## 🔧 Environment Variable Configuration

### Supported Environment Variables

**Discovery Configuration**:
```bash
# mDNS discovery timeout (default: 5s)
export TOADSTOOL_DISCOVERY_TIMEOUT=10

# Service cache TTL (default: 60s)
export TOADSTOOL_CACHE_TTL=120

# Discovery refresh interval (default: 30s)
export TOADSTOOL_REFRESH_INTERVAL=45
```

**Network Configuration**:
```bash
# Hostname (default: auto-detected)
export TOADSTOOL_HOSTNAME="toadstool-prod-01"

# Port (default: 8084)
export TOADSTOOL_PORT=8085

# Protocols (default: http)
export TOADSTOOL_PROTOCOLS="http,grpc"
```

**Resource Limits** (Optional):
```bash
# Max CPU cores to use (default: all)
export TOADSTOOL_MAX_CORES=8

# Memory limit in GB (default: auto-detected)
export TOADSTOOL_MEMORY_LIMIT_GB=16
```

**Feature Flags**:
```bash
# Enable GPU support (default: auto-detect)
export TOADSTOOL_GPU_ENABLED=true

# Enable WASM runtime (default: true)
export TOADSTOOL_WASM_ENABLED=true
```

### Full Example `.env` File:
```bash
# ToadStool Configuration
# No peer addresses - everything discovered at runtime!

# Identity
TOADSTOOL_HOSTNAME="toadstool-01"
TOADSTOOL_PORT=8084
TOADSTOOL_PROTOCOLS="http"

# Discovery
TOADSTOOL_DISCOVERY_TIMEOUT=10
TOADSTOOL_CACHE_TTL=120
TOADSTOOL_REFRESH_INTERVAL=45

# Resources
TOADSTOOL_MAX_CORES=8
TOADSTOOL_MEMORY_LIMIT_GB=16
TOADSTOOL_GPU_ENABLED=true

# Note: NO hardcoded addresses for other services!
# They will be discovered at runtime via mDNS
```

---

## 🏗️ Architecture Patterns

### Pattern 1: Standalone Deployment
```rust
// Single ToadStool instance
// Works completely standalone

let identity = SelfIdentity::new()
    .with_network("toadstool-standalone", Some(8084), vec!["http"]);

let mdns = MdnsDiscoveryService::new()?;
mdns.advertise(&identity)?;

// Can optionally discover other services if available
// But works fine without them (requirements are optional)
```

### Pattern 2: Ecosystem Deployment
```rust
// Multiple primals working together
// Each knows only itself, discovers others

// ToadStool
let toadstool = SelfIdentity::new()
    .with_network("toadstool-01", Some(8084), vec!["http"]);

// Nestgate (storage)
let nestgate = SelfIdentity::new()
    .with_network("nestgate-01", Some(8082), vec!["http"])
    .with_capability("storage", "1.0", vec!["object-store"]);

// Songbird (coordination)
let songbird = SelfIdentity::new()
    .with_network("songbird-01", Some(9080), vec!["http"])
    .with_capability("coordination", "1.0", vec!["routing"]);

// Each advertises independently
for identity in [toadstool, nestgate, songbird] {
    let mdns = MdnsDiscoveryService::new()?;
    mdns.advertise(&identity)?;
}

// Discovery happens automatically when needed
```

### Pattern 3: Kubernetes/Docker Deployment
```yaml
# docker-compose.yml - No hardcoded links!

services:
  toadstool:
    image: toadstool:latest
    environment:
      - TOADSTOOL_HOSTNAME=toadstool-01
      - TOADSTOOL_PORT=8084
    # No links or depends_on needed!
    # Services discover each other via mDNS
    
  nestgate:
    image: nestgate:latest
    environment:
      - NESTGATE_HOSTNAME=nestgate-01
      - NESTGATE_PORT=8082
    # Advertises "storage" capability
    
  songbird:
    image: songbird:latest
    environment:
      - SONGBIRD_HOSTNAME=songbird-01
      - SONGBIRD_PORT=9080
    # Advertises "coordination" capability
```

---

## 🎯 Capability Taxonomy

### Standard Capabilities

**Compute**: Workload execution
```rust
Capability {
    name: "compute",
    version: "1.0",
    features: vec!["cpu", "parallel", "gpu"],
}
```

**Orchestration**: Workload management
```rust
Capability {
    name: "orchestration",
    version: "1.0",
    features: vec!["scheduling", "resource-allocation"],
}
```

**Storage**: Persistent storage
```rust
Capability {
    name: "storage",
    version: "1.0",
    features: vec!["object-store", "metadata", "versioning"],
}
```

**Coordination**: Message routing, consensus
```rust
Capability {
    name: "coordination",
    version: "1.0",
    features: vec!["routing", "discovery", "consensus"],
}
```

**Security**: Authentication, policies
```rust
Capability {
    name: "security",
    version: "1.0",
    features: vec!["authentication", "policy", "encryption"],
}
```

**AI**: Orchestration, optimization
```rust
Capability {
    name: "ai",
    version: "1.0",
    features: vec!["orchestration", "optimization", "planning"],
}
```

### Custom Capabilities

```rust
// Define custom capabilities for your use case
Capability {
    name: "video-transcoding",
    version: "1.0",
    features: vec!["h264", "h265", "av1", "gpu-accelerated"],
    characteristics: {
        "max-resolution": "4K",
        "hardware": "NVENC",
    }
}
```

---

## 🔍 Discovery Best Practices

### 1. Timeout Configuration
```rust
// Short timeout for local network
let services = mdns.discover_by_capability(
    "storage",
    Duration::from_secs(2)  // Local network
).await?;

// Longer timeout for larger networks
let services = mdns.discover_by_capability(
    "storage",
    Duration::from_secs(10)  // Larger deployment
).await?;
```

### 2. Caching Strategy
```rust
// Cache discovered services
let services = mdns.discover_all(Duration::from_secs(5)).await?;

// Use cached services for fast lookup
let cached = mdns.get_cached_services().await;

// Refresh periodically (every 30-60s recommended)
```

### 3. Service Selection
```rust
// Strategy 1: First available
let service = services.first()
    .ok_or("No service found")?;

// Strategy 2: Feature matching
let service = services.iter()
    .find(|s| s.has_feature("storage", "versioning"))
    .ok_or("No service with required features")?;

// Strategy 3: Load balancing
let service = services
    .choose(&mut rand::thread_rng())  // Random selection
    .ok_or("No service available")?;

// Strategy 4: Health-based
let service = services.iter()
    .min_by_key(|s| s.metadata.get("load").unwrap_or("0"))
    .ok_or("No service available")?;
```

### 4. Fallback Handling
```rust
// Graceful degradation if service not found
let storage = match discover_storage().await {
    Ok(client) => Some(client),
    Err(e) => {
        tracing::warn!("No storage service found: {}", e);
        None  // Work without storage
    }
};

// Or fail fast if required
let storage = discover_storage().await
    .context("Storage service required but not found")?;
```

---

## 🌍 Multi-Network Scenarios

### Scenario 1: Local Development
```bash
# Everything on localhost, different ports
# mDNS works perfectly

# Terminal 1
TOADSTOOL_PORT=8084 cargo run --release

# Terminal 2
NESTGATE_PORT=8082 cargo run --release

# Terminal 3
SONGBIRD_PORT=9080 cargo run --release

# They discover each other automatically!
```

### Scenario 2: Docker Network
```yaml
# docker-compose.yml with custom network
networks:
  ecoprimal-net:
    driver: bridge
    
services:
  toadstool:
    networks:
      - ecoprimal-net
    # mDNS works within Docker network
```

### Scenario 3: Kubernetes
```yaml
# k8s-deployment.yaml
apiVersion: v1
kind: Service
metadata:
  name: toadstool-discovery
  annotations:
    service.alpha.kubernetes.io/tolerate-unready-endpoints: "true"
spec:
  clusterIP: None  # Headless service for mDNS
  selector:
    app: toadstool
```

### Scenario 4: Cloud/Cross-Region
```rust
// For cloud deployments, consider DNS-SD with DNS
// mDNS works for local network, DNS-SD for global

#[cfg(feature = "cloud-discovery")]
use toadstool::discovery::DnsDiscoveryService;

let discovery = DnsDiscoveryService::new("_toadstool._tcp.example.com")?;
discovery.advertise(&identity)?;
```

---

## 🔒 Security Considerations

### 1. Service Authentication
```rust
// Discovered services should still be authenticated
let service = discover_service("storage").await?;

// Verify identity before trusting
let client = StorageClient::connect(&service.endpoint)
    .await?
    .authenticate(credentials)
    .await?;
```

### 2. TLS/Encryption
```rust
// Use TLS for discovered endpoints
let service = discover_service("storage").await?;
let secure_endpoint = service.endpoint.replace("http://", "https://");

let client = StorageClient::connect(&secure_endpoint)
    .await?;
```

### 3. Capability Verification
```rust
// Verify service actually has claimed capabilities
let service = discover_service("storage").await?;

if !service.has_capability("storage") {
    return Err("Service doesn't provide required capability");
}

// Check version compatibility
if !service.version_compatible("1.0") {
    return Err("Service version incompatible");
}
```

---

## 📊 Monitoring & Observability

### Discovery Metrics
```rust
// Monitor discovery health
let metrics = mdns.get_metrics().await;

tracing::info!(
    "Discovery stats: {} services cached, {} discoveries, hit rate: {:.1}%",
    metrics.cached_services,
    metrics.total_discoveries,
    metrics.hit_rate * 100.0
);
```

### Service Health
```rust
// Track discovered service health
for service in mdns.get_cached_services().await {
    let age = service.last_seen.elapsed();
    if age > Duration::from_secs(120) {
        tracing::warn!("Service {} not seen for {:?}", service.instance_id, age);
    }
}
```

---

## 🎓 Migration Guide

### From Hardcoded Configuration

**Before** (hardcoded):
```rust
// config.yaml
services:
  storage:
    url: "http://192.168.1.100:8082"
  coordinator:
    url: "http://192.168.1.101:9080"

// Code
let storage = StorageClient::connect(&config.storage_url).await?;
```

**After** (capability-based):
```rust
// No config file needed!

// Code
async fn get_storage() -> Result<StorageClient> {
    let services = mdns.discover_by_capability("storage", timeout).await?;
    let service = services.first().ok_or("No storage found")?;
    StorageClient::connect(&service.endpoint).await
}
```

### Migration Steps
1. ✅ Remove hardcoded URLs from config
2. ✅ Add capability detection to each service
3. ✅ Implement mDNS advertising
4. ✅ Replace direct connections with discovery
5. ✅ Test service restart scenarios
6. ✅ Monitor discovery metrics

---

## ✅ Checklist for Production

- [ ] Each service knows only itself (self-identity)
- [ ] Services advertise capabilities via mDNS
- [ ] No hardcoded peer addresses in code or config
- [ ] Discovery timeouts configured appropriately
- [ ] Fallback behavior defined for missing services
- [ ] Service authentication implemented
- [ ] TLS enabled for production
- [ ] Discovery metrics monitored
- [ ] Service health checks in place
- [ ] Documentation updated

---

## 🏆 Benefits Achieved

✅ **Portability**: Works on any network without configuration  
✅ **Resilience**: Services can restart/move without breaking  
✅ **Scalability**: Add new instances dynamically  
✅ **Simplicity**: No configuration files to manage  
✅ **Agnostic**: Not tied to specific infrastructure  
✅ **Self-Healing**: Automatic service recovery  
✅ **Zero Trust**: No hardcoded trust relationships  

---

**Status**: Production-Ready ✅  
**Philosophy**: "Know Yourself, Discover Others at Runtime" - Fully Implemented!

🍄 **ToadStool: Capability-based discovery without compromises** 🍄

