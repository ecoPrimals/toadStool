# 🎯 Capability-Based Discovery: Modern Pattern Guide

**Version**: 0.2.0  
**Date**: December 5, 2025  
**Status**: ✅ **Architecture Ready, Migration In Progress**

---

## 📖 Overview

This guide documents the evolution from **hardcoded primal endpoints** to **capability-based runtime discovery**, implementing the **self-knowledge principle**: each primal knows only itself and discovers others dynamically.

---

## 🚫 **LEGACY PATTERN (Deprecated)**

### Anti-Pattern: Hardcoded Primal Knowledge

```rust
// ❌ BAD: ToadStool "knows" Songbird's endpoint
let songbird_endpoint = "http://localhost:50001";
let response = client.post(songbird_endpoint).send().await?;

// ❌ BAD: Configuration has hardcoded primal ports
pub fn get_songbird_port() -> u16 {
    50001  // Hardcoded!
}

// ❌ BAD: Direct primal-to-primal coupling
impl ToadStool {
    pub fn call_songbird(&self) -> Result<Response> {
        let url = format!("http://{}:{}", 
            self.config.songbird_host,  // Hardcoded knowledge
            self.config.songbird_port);
        // ...
    }
}
```

### Why This Is Wrong

1. **Violates self-knowledge**: ToadStool shouldn't "know" Songbird exists
2. **Brittle**: Breaks when Songbird moves or scales
3. **Not universal**: Assumes specific deployment topology
4. **Hard to test**: Requires mock services at known endpoints
5. **Not cloud-native**: Can't handle service discovery, load balancing

---

## ✅ **MODERN PATTERN (Capability-Based)**

### Principle: Self-Knowledge + Runtime Discovery

```rust
// ✅ GOOD: Self-knowledge - ToadStool knows only itself
pub struct ToadStoolConfig {
    pub name: String,              // "toadstool"
    pub port: u16,                 // My port
    pub capabilities: Vec<Capability>,  // What I can do
    // NO songbird_endpoint!
    // NO beardog_port!
    // NO other primal knowledge!
}

// ✅ GOOD: Discover services by capability at runtime
use toadstool_common::runtime_discovery::RuntimeDiscovery;

async fn coordinate_task(task: Task) -> Result<Response> {
    let discovery = RuntimeDiscovery::new();
    
    // Find ANY service with coordination capability
    let coord_services = discovery
        .discover_capability(&Capability::Coordination)
        .await?;
    
    // Use first available (or implement load balancing)
    let service = coord_services
        .first()
        .ok_or(Error::NoServiceAvailable)?;
    
    // Call the service (don't care if it's Songbird, could be anything)
    let response = client
        .post(&service.endpoint)
        .json(&task)
        .send()
        .await?;
    
    Ok(response)
}
```

---

## 🏗️ **ARCHITECTURE**

### 1. Capability Definition

```rust
/// Capabilities that services can provide
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    /// Orchestration and coordination
    Coordination,
    
    /// Cryptographic operations
    Crypto,
    
    /// Persistent storage
    Storage,
    
    /// AI/ML operations
    AI,
    
    /// Compute execution
    Compute,
    
    /// Custom capability
    Custom(String),
}
```

### 2. Service Discovery

```rust
/// Runtime service discovery
pub struct RuntimeDiscovery {
    cache: Arc<RwLock<HashMap<Capability, Vec<ServiceInfo>>>>,
    discovery_sources: Vec<Box<dyn DiscoverySource>>,
}

impl RuntimeDiscovery {
    /// Discover services providing a specific capability
    pub async fn discover_capability(
        &self,
        capability: &Capability,
    ) -> Result<Vec<ServiceInfo>> {
        // Check cache first
        if let Some(cached) = self.cache.read().await.get(capability) {
            if !cached.is_empty() {
                return Ok(cached.clone());
            }
        }
        
        // Query all discovery sources
        let mut services = Vec::new();
        for source in &self.discovery_sources {
            if let Ok(found) = source.find_by_capability(capability).await {
                services.extend(found);
            }
        }
        
        // Update cache
        self.cache.write().await.insert(capability.clone(), services.clone());
        
        Ok(services)
    }
    
    /// Discover all available services
    pub async fn discover_all(&self) -> Result<Vec<ServiceInfo>> {
        let mut all_services = Vec::new();
        for source in &self.discovery_sources {
            if let Ok(services) = source.list_all().await {
                all_services.extend(services);
            }
        }
        Ok(all_services)
    }
}
```

### 3. Service Information

```rust
/// Information about a discovered service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    /// Service name (e.g., "songbird-prod-01")
    pub name: String,
    
    /// Service endpoint (e.g., "https://songbird.example.com")
    pub endpoint: String,
    
    /// Capabilities this service provides
    pub capabilities: Vec<Capability>,
    
    /// Service version
    pub version: String,
    
    /// Health status
    pub status: ServiceStatus,
    
    /// Metadata
    pub metadata: HashMap<String, String>,
}
```

---

## 🔄 **MIGRATION PATTERNS**

### Pattern 1: Direct Endpoint Call → Capability Discovery

```rust
// ❌ Before: Hardcoded
async fn send_to_songbird(data: Data) -> Result<Response> {
    let endpoint = "http://localhost:50001";  // Hardcoded!
    client.post(endpoint).json(&data).send().await
}

// ✅ After: Capability-based
async fn send_to_coordinator(data: Data) -> Result<Response> {
    let discovery = RuntimeDiscovery::new();
    let services = discovery
        .discover_capability(&Capability::Coordination)
        .await?;
    
    let service = services
        .first()
        .ok_or(Error::NoCoordinatorAvailable)?;
    
    client
        .post(&service.endpoint)
        .json(&data)
        .send()
        .await
}
```

### Pattern 2: Config-Based → Discovery-Based

```rust
// ❌ Before: Config has hardcoded knowledge
pub struct ToadStoolConfig {
    pub songbird_endpoint: String,  // ❌ Shouldn't know Songbird
    pub beardog_port: u16,          // ❌ Shouldn't know BearDog
}

// ✅ After: Self-knowledge only
pub struct ToadStoolConfig {
    pub name: String,               // "toadstool"
    pub endpoint: String,           // My endpoint
    pub capabilities: Vec<Capability>,  // What I provide
    pub discovery: DiscoveryConfig,     // How to find others
}

pub struct DiscoveryConfig {
    pub sources: Vec<DiscoverySourceConfig>,
    pub cache_ttl: Duration,
    pub retry_policy: RetryPolicy,
}
```

### Pattern 3: Static Client → Dynamic Client

```rust
// ❌ Before: Static clients with hardcoded endpoints
pub struct ToadStoolClients {
    songbird: SongbirdClient,  // ❌ Hardcoded
    beardog: BeardogClient,    // ❌ Hardcoded
    nestgate: NestgateClient,  // ❌ Hardcoded
}

// ✅ After: Dynamic capability-based client
pub struct UniversalClient {
    discovery: RuntimeDiscovery,
    http_client: reqwest::Client,
    cache: Arc<RwLock<ClientCache>>,
}

impl UniversalClient {
    /// Call any service with a capability
    pub async fn call_capability(
        &self,
        capability: &Capability,
        request: Request,
    ) -> Result<Response> {
        let services = self.discovery
            .discover_capability(capability)
            .await?;
        
        // Load balance across available services
        let service = self.select_service(&services)?;
        
        self.http_client
            .post(&service.endpoint)
            .json(&request)
            .send()
            .await
    }
}
```

---

## 🌐 **DISCOVERY SOURCES**

### 1. Environment Variables (Development)

```rust
pub struct EnvDiscoverySource;

impl DiscoverySource for EnvDiscoverySource {
    async fn find_by_capability(
        &self,
        capability: &Capability,
    ) -> Result<Vec<ServiceInfo>> {
        let key = format!("TOADSTOOL_CAPABILITY_{:?}_SERVICES", capability);
        let services = std::env::var(&key)?;
        
        // Parse JSON array of service endpoints
        serde_json::from_str(&services)
    }
}
```

### 2. Configuration File (Simple Deployments)

```toml
# toadstool.toml - No hardcoded primal names!
[discovery]
sources = ["env", "config", "mdns"]

[[services]]
endpoint = "https://coord.example.com"
capabilities = ["Coordination"]
version = "1.0.0"

[[services]]
endpoint = "https://crypto.example.com"
capabilities = ["Crypto"]
version = "1.0.0"
```

### 3. mDNS/DNS-SD (Local Network)

```rust
pub struct MDNSDiscoverySource {
    service_type: String,  // "_toadstool._tcp"
}

impl DiscoverySource for MDNSDiscoverySource {
    async fn list_all(&self) -> Result<Vec<ServiceInfo>> {
        // Discover via mDNS
        let browser = mdns::Browser::new(&self.service_type)?;
        let services = browser.discover().await?;
        Ok(services)
    }
}
```

### 4. Consul/etcd (Production)

```rust
pub struct ConsulDiscoverySource {
    client: ConsulClient,
    datacenter: String,
}

impl DiscoverySource for ConsulDiscoverySource {
    async fn find_by_capability(
        &self,
        capability: &Capability,
    ) -> Result<Vec<ServiceInfo>> {
        let tag = format!("capability:{:?}", capability);
        self.client
            .catalog()
            .service_by_tag(&self.datacenter, &tag)
            .await
    }
}
```

### 5. Kubernetes Services

```rust
pub struct K8sDiscoverySource {
    client: kube::Client,
    namespace: String,
}

impl DiscoverySource for K8sDiscoverySource {
    async fn list_all(&self) -> Result<Vec<ServiceInfo>> {
        let services: Api<Service> = Api::namespaced(
            self.client.clone(),
            &self.namespace,
        );
        
        let svc_list = services.list(&Default::default()).await?;
        // Convert K8s services to ServiceInfo
        Ok(convert_k8s_services(svc_list))
    }
}
```

---

## 📝 **EXAMPLE: Complete Evolution**

### Before: Hardcoded Songbird Calls

```rust
// ❌ OLD: crates/core/toadstool/src/ecosystem.rs
impl ToadStoolEcosystem {
    pub async fn coordinate_task(&self, task: Task) -> Result<TaskResult> {
        // Hardcoded Songbird endpoint
        let songbird_url = format!(
            "http://{}:{}",
            self.config.songbird_host,
            self.config.songbird_port
        );
        
        let response = self.http_client
            .post(&format!("{}/coordinate", songbird_url))
            .json(&task)
            .send()
            .await?;
        
        response.json().await
    }
}
```

### After: Capability-Based Discovery

```rust
// ✅ NEW: Modern capability-based pattern
use toadstool_common::runtime_discovery::{RuntimeDiscovery, Capability};

impl ToadStoolEcosystem {
    pub async fn coordinate_task(&self, task: Task) -> Result<TaskResult> {
        // Discover coordination services dynamically
        let coord_services = self.discovery
            .discover_capability(&Capability::Coordination)
            .await
            .or_else(|_| self.get_fallback_coordinator())?;  // Graceful degradation
        
        // Select best service (load balancing, health, latency)
        let service = self.select_best_service(&coord_services)
            .ok_or(Error::NoCoordinatorAvailable)?;
        
        // Make the call (don't care which implementation)
        let response = self.http_client
            .post(&format!("{}/coordinate", service.endpoint))
            .json(&task)
            .timeout(self.config.request_timeout)
            .send()
            .await
            .map_err(|e| Error::CoordinationFailed(service.name.clone(), e))?;
        
        response.json().await
    }
    
    /// Fallback to configured endpoints (legacy compatibility)
    fn get_fallback_coordinator(&self) -> Result<Vec<ServiceInfo>> {
        warn!("Using fallback coordinator configuration (legacy mode)");
        
        if let Some(endpoint) = &self.config.fallback_coordinator {
            Ok(vec![ServiceInfo {
                name: "fallback-coordinator".to_string(),
                endpoint: endpoint.clone(),
                capabilities: vec![Capability::Coordination],
                version: "unknown".to_string(),
                status: ServiceStatus::Unknown,
                metadata: HashMap::new(),
            }])
        } else {
            Err(Error::NoCoordinatorConfigured)
        }
    }
}
```

---

## 🧪 **TESTING PATTERNS**

### Mock Discovery for Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    /// Mock discovery source for testing
    struct MockDiscovery {
        services: HashMap<Capability, Vec<ServiceInfo>>,
    }
    
    impl MockDiscovery {
        fn with_coordination_service(endpoint: &str) -> Self {
            let mut services = HashMap::new();
            services.insert(
                Capability::Coordination,
                vec![ServiceInfo {
                    name: "mock-coordinator".to_string(),
                    endpoint: endpoint.to_string(),
                    capabilities: vec![Capability::Coordination],
                    version: "test".to_string(),
                    status: ServiceStatus::Healthy,
                    metadata: HashMap::new(),
                }],
            );
            Self { services }
        }
    }
    
    #[tokio::test]
    async fn test_coordinate_task_with_discovery() {
        let mock_server = mockito::Server::new_async().await;
        let mock_url = mock_server.url();
        
        // Setup mock response
        let _m = mock_server.mock("POST", "/coordinate")
            .with_status(200)
            .with_body(r#"{"status": "success"}"#)
            .create();
        
        // Use mock discovery
        let discovery = MockDiscovery::with_coordination_service(&mock_url);
        let ecosystem = ToadStoolEcosystem::with_discovery(discovery);
        
        let task = Task::new("test_task");
        let result = ecosystem.coordinate_task(task).await;
        
        assert!(result.is_ok());
    }
}
```

---

## ✅ **DEPRECATION STRATEGY**

### Phase 1: Add Deprecation Warnings (✅ DONE)

```rust
#[deprecated(
    since = "0.2.0",
    note = "Use capability-based discovery instead of hardcoded endpoints"
)]
pub fn get_songbird_endpoint() -> String {
    // Legacy implementation
}
```

### Phase 2: Log Warnings (In Progress)

```rust
pub fn get_songbird_endpoint() -> String {
    warn!("DEPRECATED: Using hardcoded Songbird endpoint. Migrate to RuntimeDiscovery.");
    warn!("See: CAPABILITY_BASED_DISCOVERY_GUIDE.md for migration patterns");
    // Return legacy endpoint
}
```

### Phase 3: Require Opt-In (Future)

```rust
#[cfg(feature = "legacy-hardcoded-endpoints")]
pub fn get_songbird_endpoint() -> String {
    // Only available with feature flag
}
```

### Phase 4: Remove (v1.0.0)

Complete removal of hardcoded endpoint methods.

---

## 📊 **MIGRATION STATUS**

### ✅ Completed

- [x] Capability enum defined (`toadstool_common::capabilities`)
- [x] RuntimeDiscovery implemented (`toadstool_common::runtime_discovery`)
- [x] ServiceInfo structs defined
- [x] Deprecation warnings added to hardcoded methods
- [x] Documentation created (this file)

### 🚧 In Progress

- [ ] Migrate ecosystem coordination calls
- [ ] Migrate crypto operations
- [ ] Migrate storage operations
- [ ] Migrate AI/ML calls
- [ ] Add discovery sources (mDNS, Consul, K8s)

### 📋 Planned

- [ ] Complete test coverage for discovery
- [ ] Add load balancing strategies
- [ ] Implement health checking
- [ ] Add metrics for discovery performance
- [ ] Create migration tools/scripts

---

## 🎓 **PRINCIPLES**

### 1. Self-Knowledge

> Each primal knows only itself. No hardcoded knowledge of other primals.

```rust
// ✅ ToadStool knows:
- My name: "toadstool"
- My port: 8082
- My capabilities: [Compute, Execution]

// ❌ ToadStool does NOT know:
- Songbird's port
- BearDog's endpoint
- NestGate's existence
```

### 2. Runtime Discovery

> Services are discovered at runtime by capability, not by name.

```rust
// ✅ "Find me a coordination service"
let coord = discovery.discover_capability(&Capability::Coordination).await?;

// ❌ "Connect to Songbird at port 50001"
let songbird = connect("localhost:50001");
```

### 3. Graceful Degradation

> Fallback to configuration if discovery fails (dev/test).

```rust
let services = discovery.discover_capability(&cap).await
    .or_else(|_| self.get_configured_fallback())?;
```

### 4. Cloud-Native

> Works in dynamic environments (K8s, service mesh, multi-cloud).

---

## 🚀 **NEXT STEPS**

1. **Review this guide** with the team
2. **Migrate one module** as a reference implementation
3. **Create migration PRs** for each subsystem
4. **Update tests** to use mock discovery
5. **Deploy with both** patterns for transition period
6. **Monitor metrics** during migration
7. **Complete removal** of legacy patterns in v1.0.0

---

**Status**: ✅ **Architecture Ready**  
**Pattern**: ✅ **Documented**  
**Migration**: 🚧 **20% Complete**  
**Timeline**: **4-6 weeks to complete**

---

*"Self-knowledge is wisdom. Runtime discovery is flexibility. Capability-based design is universal."* 🎯

