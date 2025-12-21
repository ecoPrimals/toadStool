# 🌱 Capability Discovery Integration Guide
## Modern, Self-Discovering Architecture

**Philosophy**: "Each primal knows only itself. Everything else is discovered - like living organisms."

---

## 🎯 QUICK START

### Before (Hardcoded):
```rust
// ❌ OLD: Hardcoded identity and location
let beardog_url = "http://localhost:8081";
let beardog_client = HttpClient::new(beardog_url)?;
let signature = beardog_client.sign(data).await?;
```

### After (Capability-Based):
```rust
// ✅ NEW: Discover by capability, not identity
let crypto_services = discovery
    .find_by_capability("cryptographic-operations")
    .await?;

for service in crypto_services {
    match try_sign(&service, data).await {
        Ok(sig) => return Ok(sig),
        Err(e) => warn!("Service {} failed, trying next", service.endpoint),
    }
}
Err("No crypto service available")
```

**Benefits**:
- 🔄 Automatic failover
- ⚖️ Load balancing
- 🏥 Health-aware routing
- 🌐 Multi-instance support
- 🔌 Zero hardcoding

---

## 📚 USING THE INFANT DISCOVERY SYSTEM

### 1. Initialize Discovery Engine

```rust
use toadstool_common::infant_discovery::{DiscoveryEngine, ServiceDiscoveryConfig};

// Create engine with default config
let discovery = DiscoveryEngine::new();

// Or with custom config
let config = ServiceDiscoveryConfig {
    enable_cache: true,
    cache_ttl: Duration::from_secs(300),
    default_timeout: Duration::from_secs(30),
    retry_attempts: 3,
    retry_delay: Duration::from_secs(1),
};
let discovery = DiscoveryEngine::with_config(config);
```

### 2. Register Discovery Sources

```rust
use toadstool_common::infant_discovery::{
    EnvironmentSource, MDNSSource, ServiceMeshSource, ConfigFileSource
};

// Environment variables (highest priority)
discovery.register_source(Box::new(EnvironmentSource::new())).await;

// mDNS for local network
discovery.register_source(Box::new(MDNSSource::new())).await;

// Service mesh (Kubernetes, Consul, etc.)
discovery.register_source(Box::new(ServiceMeshSource::new("consul"))).await;

// Config file (lowest priority, fallback)
discovery.register_source(Box::new(ConfigFileSource::new("primal-capabilities.toml"))).await;
```

### 3. Discover Services by Capability

```rust
// Find services with specific capability
let services = discovery
    .find_by_capability("cryptographic-operations")
    .await?;

// Find services with multiple capabilities
let services = discovery
    .find_by_capabilities(vec![
        "storage",
        "distributed-storage",
    ])
    .await?;

// With preferences
let prefs = DiscoveryPreferences {
    prefer_local: true,
    require_healthy: true,
    max_results: 5,
    timeout: Duration::from_secs(10),
};

let services = discovery
    .find_by_capability_with_prefs("service-discovery", prefs)
    .await?;
```

---

## 🔧 MIGRATION PATTERNS

### Pattern 1: Simple Service Call

```rust
// ❌ BEFORE
async fn get_crypto_key(key_id: &str) -> Result<Key> {
    let url = "http://localhost:8081/keys";
    let response = reqwest::get(format!("{}/{}", url, key_id)).await?;
    Ok(response.json().await?)
}

// ✅ AFTER
async fn get_crypto_key(
    discovery: &DiscoveryEngine,
    key_id: &str
) -> Result<Key> {
    let services = discovery
        .find_by_capability("key-management")
        .await?;
    
    for service in services {
        let url = format!("{}/keys/{}", service.endpoint, key_id);
        match reqwest::get(&url).await {
            Ok(response) => return Ok(response.json().await?),
            Err(e) => warn!("Service {} failed: {}", service.endpoint, e),
        }
    }
    Err(anyhow!("No key management service available"))
}
```

### Pattern 2: Service with Fallback

```rust
// ❌ BEFORE
async fn store_data(data: &[u8]) -> Result<String> {
    // Hardcoded primary and fallback
    match store_to_primary(data).await {
        Ok(id) => Ok(id),
        Err(_) => store_to_fallback(data).await,
    }
}

// ✅ AFTER
async fn store_data(
    discovery: &DiscoveryEngine,
    data: &[u8]
) -> Result<String> {
    // Discovers all available storage services
    // Tries in priority order automatically
    let services = discovery
        .find_by_capability("storage")
        .await?;
    
    for service in services {
        match try_store(&service, data).await {
            Ok(id) => {
                info!("Stored to {} successfully", service.endpoint);
                return Ok(id);
            }
            Err(e) => {
                warn!("Storage {} failed: {}, trying next", service.endpoint, e);
                continue;
            }
        }
    }
    Err(anyhow!("All storage services failed"))
}
```

### Pattern 3: Load Balanced Requests

```rust
// ❌ BEFORE
static CURRENT_SERVER: AtomicUsize = AtomicUsize::new(0);
const SERVERS: &[&str] = &[
    "http://server1:8080",
    "http://server2:8080",
    "http://server3:8080",
];

async fn execute_workload(workload: Workload) -> Result<Output> {
    let idx = CURRENT_SERVER.fetch_add(1, Ordering::SeqCst) % SERVERS.len();
    let server = SERVERS[idx];
    send_to_server(server, workload).await
}

// ✅ AFTER
async fn execute_workload(
    discovery: &DiscoveryEngine,
    workload: Workload
) -> Result<Output> {
    // Discovery engine handles load balancing automatically
    let services = discovery
        .find_by_capability("universal-compute")
        .await?;
    
    // Services are already sorted by health and load
    // Pick first available (built-in round-robin)
    let service = services.first()
        .ok_or(anyhow!("No compute service available"))?;
    
    send_to_service(service, workload).await
}
```

### Pattern 4: Conditional Capabilities

```rust
// ❌ BEFORE
async fn optimize_with_gpu_if_available(task: Task) -> Result<Output> {
    if let Ok(gpu) = connect_to_gpu_service().await {
        gpu.execute(task).await
    } else {
        execute_on_cpu(task).await
    }
}

// ✅ AFTER
async fn optimize_with_gpu_if_available(
    discovery: &DiscoveryEngine,
    task: Task
) -> Result<Output> {
    // Try GPU first
    if let Ok(services) = discovery.find_by_capability("gpu-compute").await {
        if let Some(gpu) = services.first() {
            if let Ok(result) = execute_on_service(gpu, &task).await {
                return Ok(result);
            }
        }
    }
    
    // Fallback to CPU
    let cpu_services = discovery
        .find_by_capability("native-execution")
        .await?;
    execute_on_service(&cpu_services[0], &task).await
}
```

---

## 🏗️ INTEGRATION INTO EXISTING CODE

### Step 1: Add Discovery Engine to Struct

```rust
// BEFORE
pub struct MyService {
    config: Config,
    http_client: HttpClient,
}

// AFTER
pub struct MyService {
    config: Config,
    http_client: HttpClient,
    discovery: Arc<DiscoveryEngine>,  // ← Add this
}
```

### Step 2: Update Constructor

```rust
// BEFORE
impl MyService {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            http_client: HttpClient::new(),
        }
    }
}

// AFTER
impl MyService {
    pub async fn new(
        config: Config,
        discovery: Arc<DiscoveryEngine>
    ) -> Self {
        Self {
            config,
            http_client: HttpClient::new(),
            discovery,
        }
    }
}
```

### Step 3: Replace Hardcoded Calls

```rust
// BEFORE
async fn call_external_service(&self, data: Data) -> Result<Response> {
    let url = "http://localhost:8082/api";
    self.http_client.post(url, data).await
}

// AFTER
async fn call_external_service(&self, data: Data) -> Result<Response> {
    let services = self.discovery
        .find_by_capability("service-discovery")
        .await?;
    
    let service = services.first()
        .ok_or(anyhow!("No service found"))?;
    
    self.http_client.post(&service.endpoint, data).await
}
```

---

## 🧪 TESTING WITH DISCOVERY

### Mock Discovery for Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use toadstool_testing::mocks::MockDiscoveryEngine;
    
    #[tokio::test]
    async fn test_with_mock_discovery() {
        // Create mock discovery
        let mut mock_discovery = MockDiscoveryEngine::new();
        
        // Configure mock to return specific services
        mock_discovery
            .expect_find_by_capability("storage")
            .returning(|_| {
                Ok(vec![DiscoveredService {
                    capability: "storage".to_string(),
                    endpoint: "http://localhost:9999".to_string(),
                    protocols: vec!["http".to_string()],
                    metadata: ServiceMetadata::default(),
                    source: DiscoverySource::Environment,
                }])
            });
        
        // Test your service
        let service = MyService::new(config, Arc::new(mock_discovery)).await;
        let result = service.store_data(b"test").await;
        assert!(result.is_ok());
    }
}
```

---

## 📋 CAPABILITY REFERENCE

### From `primal-capabilities.toml`:

**Cryptographic Operations** (beardog):
- `cryptographic-operations`
- `key-management`
- `signing`, `verification`
- `encryption`, `decryption`

**Service Discovery** (songbird):
- `service-discovery`
- `load-balancing`
- `request-routing`
- `job-routing`

**Storage** (nestgate):
- `storage`
- `distributed-storage`
- `object-storage`
- `caching`

**AI/ML** (squirrel):
- `ai-agents`
- `mcp-protocol`
- `plugin-execution`
- `inference`

**Compute** (toadstool):
- `universal-compute`
- `native-execution`
- `wasm-execution`
- `container-execution`
- `gpu-compute`

---

## 🚀 DEPLOYMENT CONFIGURATION

### Environment Variables

```bash
# Highest priority - env vars
export TOADSTOOL_DISCOVERY_CRYPTO_ENDPOINT="http://beardog-prod:8081"
export TOADSTOOL_DISCOVERY_STORAGE_ENDPOINT="http://nestgate-prod:8083"

# Services advertise capabilities
export TOADSTOOL_CAPABILITIES="universal-compute,multi-runtime-execution"
```

### Kubernetes Service Discovery

```yaml
apiVersion: v1
kind: Service
metadata:
  name: beardog
  annotations:
    capabilities: "cryptographic-operations,key-management"
spec:
  selector:
    app: beardog
  ports:
  - port: 8081
```

### Docker Compose

```yaml
version: '3.8'
services:
  toadstool:
    image: toadstool:latest
    environment:
      - TOADSTOOL_DISCOVERY_METHOD=mdns
    
  beardog:
    image: beardog:latest
    labels:
      - "capabilities=cryptographic-operations,key-management"
```

---

## 🔍 DEBUGGING DISCOVERY

```rust
// Enable discovery logging
std::env::set_var("RUST_LOG", "toadstool_common::infant_discovery=debug");

// Check what was discovered
let services = discovery.find_by_capability("storage").await?;
for service in &services {
    println!("Found: {} at {} via {:?}",
        service.capability,
        service.endpoint,
        service.source
    );
}

// Check cache
let cache_stats = discovery.get_cache_stats().await;
println!("Cache: {} services, hit rate: {}%",
    cache_stats.size,
    cache_stats.hit_rate * 100.0
);
```

---

## 📚 NEXT STEPS

1. ✅ Review this guide
2. 🔄 Integrate discovery into your module
3. 🔄 Replace hardcoded references
4. 🧪 Test with mock discovery
5. 🚀 Deploy with real discovery

---

**Created**: December 3, 2025  
**Status**: Ready for Integration  
**Philosophy**: "Like living organisms - discover by need, not by name"


