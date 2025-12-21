# 🔄 Primal Hardcoding Elimination & Universal Adapter Migration

**Date**: November 18, 2025  
**Goal**: Eliminate ALL primal and vendor hardcoding, move to pure infant discovery  
**Principle**: *"Each primal knows only itself. Everything else is discovered via capabilities."*

---

## 📋 CURRENT STATE ANALYSIS

### Hardcoded Primal References Found

#### 1. **Service-Specific Files** (HIGH PRIORITY)
```
crates/cli/src/ecosystem/services/
├── beardog.rs       ❌ Hardcoded "beardog", BearDog types, crypto specifics
├── nestgate.rs      ❌ Hardcoded "nestgate", ZFS specifics, storage assumptions
├── songbird.rs      ❌ Hardcoded "songbird", HTTP endpoint structure
└── mod.rs           ❌ Exposes all service-specific modules
```

#### 2. **Discovery Module** (HIGH PRIORITY)
```rust
// crates/cli/src/ecosystem/discovery.rs

pub fn get_standard_service_ports() -> HashMap<String, u16> {
    ports.insert("songbird".to_string(), 8080);  ❌
    ports.insert("beardog".to_string(), 8081);   ❌
    ports.insert("nestgate".to_string(), 8082);  ❌
    ports.insert("squirrel".to_string(), 8083);  ❌
}

fn parse_service_type(service_type: &str) -> EcosystemService {
    match service_type.to_lowercase().as_str() {
        "songbird" => EcosystemService::Songbird,  ❌
        "beardog" => EcosystemService::BearDog,    ❌
        "nestgate" => EcosystemService::NestGate,  ❌
        _ => EcosystemService::Unknown(service_type.to_string()),
    }
}
```

#### 3. **Type Definitions** (MEDIUM PRIORITY)
```rust
// Types with hardcoded primal names
enum EcosystemService {
    Songbird,   ❌
    BearDog,    ❌
    NestGate,   ❌
    Squirrel,   ❌
    Unknown(String),
}
```

#### 4. **Other Hardcoding**
- Port numbers (8080, 8081, 8082, 8083, 3000, 5000, 9090)
- External services (k8s, consul, etcd references)
- Endpoint structures (`/api/v1/register`)
- Protocol assumptions (HTTP, gRPC)

---

## 🎯 TARGET STATE

### Infant Discovery Pattern

**Core Principle**:
> Services start with **ZERO knowledge** and discover everything dynamically through **capability queries**, not service names.

### Example: Current vs Target

#### ❌ CURRENT (Hardcoded)
```rust
// Hardcoded service connection
let beardog_addr = "127.0.0.1:8081";
let response = connect_to_beardog(beardog_addr).await?;
```

#### ✅ TARGET (Capability-Based)
```rust
// Discover by capability, not name
let crypto_service = discovery_engine
    .discover_capability("crypto.signature.ed25519")
    .await?;
    
let response = crypto_service.invoke(request).await?;
```

### Capability Taxonomy

Instead of knowing "BearDog" or "Songbird", services know **capabilities**:

```
crypto.signature.ed25519          (was: BearDog)
crypto.encryption.aes256          (was: BearDog)
storage.distributed.zfs           (was: NestGate)
storage.object.s3-compatible      (was: NestGate or external)
coordination.service-mesh         (was: Songbird)
coordination.discovery.mdns       (was: Songbird)
compute.container.oci             (was: ToadStool)
compute.wasm.component-model      (was: ToadStool)
messaging.queue.amqp              (was: Squirrel)
```

---

## 🚀 MIGRATION PLAN

### Phase 1: Create Capability-Based Abstractions (Week 1)

#### Step 1.1: Define Capability Registry
```rust
// crates/cli/src/ecosystem/capabilities/registry.rs

pub struct CapabilityRegistry {
    engine: DiscoveryEngine,
    providers: Arc<RwLock<HashMap<CapabilityId, Vec<ServiceProvider>>>>,
}

pub struct CapabilityId(String);  // e.g., "crypto.signature.ed25519"

pub struct ServiceProvider {
    endpoint: String,
    protocols: Vec<Protocol>,
    health: ServiceHealth,
    metadata: ServiceMetadata,
}
```

#### Step 1.2: Create Universal Service Adapter
```rust
// crates/cli/src/ecosystem/adapters/universal.rs

pub struct UniversalServiceAdapter {
    discovery: Arc<DiscoveryEngine>,
    cache: Arc<RwLock<ServiceCache>>,
}

impl UniversalServiceAdapter {
    pub async fn invoke_capability(
        &self,
        capability: &str,
        request: Request,
    ) -> Result<Response> {
        // 1. Discover service providing capability
        let service = self.discovery.discover(capability).await?;
        
        // 2. Negotiate protocol
        let protocol = self.negotiate_protocol(&service).await?;
        
        // 3. Invoke via discovered protocol
        protocol.invoke(service.endpoint, request).await
    }
}
```

#### Step 1.3: Protocol Negotiation
```rust
// crates/cli/src/ecosystem/protocols/negotiation.rs

pub enum Protocol {
    Http { version: HttpVersion },
    Grpc { service_name: String },
    MessageQueue { protocol: MqProtocol },
    Custom { name: String, version: String },
}

pub trait ProtocolAdapter: Send + Sync {
    fn supports(&self, protocol: &Protocol) -> bool;
    async fn invoke(&self, endpoint: String, request: Request) -> Result<Response>;
}
```

### Phase 2: Migrate Existing Integrations (Week 2)

#### Step 2.1: Replace BearDog Hardcoding
```rust
// ❌ OLD: crates/cli/src/ecosystem/services/beardog.rs
pub async fn verify_ed25519_signature(...) -> Result<bool> {
    // Hardcoded BearDog logic
}

// ✅ NEW: crates/cli/src/ecosystem/adapters/crypto.rs
pub struct CryptoAdapter {
    universal: Arc<UniversalServiceAdapter>,
}

impl CryptoAdapter {
    pub async fn verify_signature(
        &self,
        algorithm: &str,  // "ed25519", "ecdsa", etc.
        public_key: &[u8],
        message: &[u8],
        signature: &[u8],
    ) -> Result<bool> {
        let capability = format!("crypto.signature.{}", algorithm);
        
        let request = CryptoRequest::Verify {
            public_key: public_key.to_vec(),
            message: message.to_vec(),
            signature: signature.to_vec(),
        };
        
        let response = self.universal
            .invoke_capability(&capability, request.into())
            .await?;
            
        Ok(response.verified)
    }
}
```

#### Step 2.2: Replace NestGate Hardcoding
```rust
// ❌ OLD: crates/cli/src/ecosystem/services/nestgate.rs
pub async fn connect_storage(addr: &SocketAddr, ...) -> Result<NestGateMount> {
    // Hardcoded NestGate ZFS logic
}

// ✅ NEW: crates/cli/src/ecosystem/adapters/storage.rs
pub struct StorageAdapter {
    universal: Arc<UniversalServiceAdapter>,
}

impl StorageAdapter {
    pub async fn mount_distributed_storage(
        &self,
        requirements: StorageRequirements,
    ) -> Result<MountPoint> {
        // Discover storage capability (could be NestGate, Ceph, S3, etc.)
        let capability = match requirements.backend_type {
            StorageType::Distributed => "storage.distributed",
            StorageType::Object => "storage.object",
            StorageType::Block => "storage.block",
        };
        
        let request = StorageRequest::Mount { requirements };
        
        let response = self.universal
            .invoke_capability(capability, request.into())
            .await?;
            
        Ok(response.mount_point)
    }
}
```

#### Step 2.3: Replace Songbird Hardcoding
```rust
// ❌ OLD: crates/cli/src/ecosystem/services/songbird.rs
pub async fn send_registration(addr: &SocketAddr, ...) -> Result<SongbirdResponse> {
    reqwest::Client::new()
        .post(format!("http://{addr}/api/v1/register"))  // Hardcoded endpoint
        .json(registration)
        .send()
        .await
}

// ✅ NEW: crates/cli/src/ecosystem/adapters/coordination.rs
pub struct CoordinationAdapter {
    universal: Arc<UniversalServiceAdapter>,
}

impl CoordinationAdapter {
    pub async fn register_service(
        &self,
        service_info: ServiceInfo,
    ) -> Result<RegistrationToken> {
        // Discover coordination capability (could be Songbird, Consul, etcd, etc.)
        let capability = "coordination.service-registry";
        
        let request = CoordinationRequest::Register { service_info };
        
        let response = self.universal
            .invoke_capability(capability, request.into())
            .await?;
            
        Ok(response.token)
    }
    
    pub async fn discover_peers(
        &self,
        capability_filter: Option<&str>,
    ) -> Result<Vec<PeerInfo>> {
        let capability = "coordination.peer-discovery";
        
        let request = CoordinationRequest::DiscoverPeers { 
            capability: capability_filter.map(String::from),
        };
        
        let response = self.universal
            .invoke_capability(capability, request.into())
            .await?;
            
        Ok(response.peers)
    }
}
```

### Phase 3: Eliminate Port Hardcoding (Week 2)

#### Step 3.1: Remove Standard Ports
```rust
// ❌ DELETE: crates/cli/src/ecosystem/discovery.rs
pub fn get_standard_service_ports() -> HashMap<String, u16> {
    // Delete this entire function
}
```

#### Step 3.2: Environment-Based Discovery
```rust
// ✅ NEW: Use environment variables or discovery protocols
impl DiscoveryEngine {
    async fn discover_endpoint(&self, capability: &str) -> Result<String> {
        // Try discovery sources in order:
        // 1. Environment variables (TOADSTOOL_CRYPTO_SERVICE_URL)
        // 2. mDNS discovery (_crypto-service._tcp.local)
        // 3. Service mesh (Consul, etcd)
        // 4. Config file (~/.toadstool/services.toml)
        // 5. Network scan (last resort)
    }
}
```

### Phase 4: Remove Vendor Hardcoding (Week 3)

#### Step 4.1: Abstract External Services
```rust
// Instead of hardcoding "kubernetes", "consul", etc.

pub trait InfrastructureProvider: Send + Sync {
    async fn deploy_workload(&self, spec: WorkloadSpec) -> Result<DeploymentId>;
    async fn scale(&self, deployment: &DeploymentId, replicas: u32) -> Result<()>;
    async fn health_check(&self, deployment: &DeploymentId) -> Result<HealthStatus>;
}

// Implementations discovered dynamically
pub struct KubernetesProvider { ... }  // Discovered if K8s API available
pub struct ConsulProvider { ... }      // Discovered if Consul available
pub struct DockerProvider { ... }      // Discovered if Docker socket available
```

#### Step 4.2: Protocol-Agnostic Service Discovery
```rust
pub struct ServiceMeshDetector;

impl ServiceMeshDetector {
    pub async fn detect() -> Vec<DetectedMesh> {
        let mut meshes = Vec::new();
        
        // Try Consul
        if Self::check_consul().await {
            meshes.push(DetectedMesh::Consul { endpoint });
        }
        
        // Try Kubernetes service discovery
        if Self::check_k8s().await {
            meshes.push(DetectedMesh::Kubernetes { config });
        }
        
        // Try etcd
        if Self::check_etcd().await {
            meshes.push(DetectedMesh::Etcd { endpoints });
        }
        
        meshes
    }
}
```

---

## 📦 NEW MODULE STRUCTURE

```
crates/cli/src/ecosystem/
├── mod.rs                          # Re-exports
├── capabilities/
│   ├── mod.rs                      # Capability system
│   ├── registry.rs                 # Capability registry
│   ├── taxonomy.rs                 # Standard capability names
│   └── resolver.rs                 # Capability → service resolution
├── adapters/
│   ├── mod.rs                      # Universal adapter
│   ├── universal.rs                # UniversalServiceAdapter
│   ├── crypto.rs                   # CryptoAdapter (replaces beardog.rs)
│   ├── storage.rs                  # StorageAdapter (replaces nestgate.rs)
│   ├── coordination.rs             # CoordinationAdapter (replaces songbird.rs)
│   └── messaging.rs                # MessagingAdapter (replaces squirrel hardcoding)
├── protocols/
│   ├── mod.rs                      # Protocol negotiation
│   ├── http.rs                     # HTTP/REST protocol adapter
│   ├── grpc.rs                     # gRPC protocol adapter
│   ├── mq.rs                       # Message queue protocol adapter
│   └── negotiation.rs              # Auto-protocol negotiation
├── discovery/
│   ├── mod.rs                      # Discovery coordination
│   ├── engine.rs                   # Uses infant_discovery::DiscoveryEngine
│   ├── mdns.rs                     # mDNS discovery source
│   ├── env.rs                      # Environment variable source
│   └── mesh.rs                     # Service mesh source
└── services/                       # DEPRECATED - to be removed
    ├── beardog.rs                  # ⚠️ DEPRECATED: Use adapters/crypto.rs
    ├── nestgate.rs                 # ⚠️ DEPRECATED: Use adapters/storage.rs
    └── songbird.rs                 # ⚠️ DEPRECATED: Use adapters/coordination.rs
```

---

## 🔬 EXAMPLE: COMPLETE FLOW

### Old Hardcoded Way ❌
```rust
// ToadStool needs crypto signature verification
let beardog_client = BeardogClient::new("127.0.0.1:8081");
let verified = beardog_client.verify_ed25519(key, msg, sig).await?;
```

### New Capability-Based Way ✅
```rust
// ToadStool doesn't know BearDog exists
let crypto = CryptoAdapter::new(universal_adapter);
let verified = crypto.verify_signature("ed25519", key, msg, sig).await?;

// Under the hood:
// 1. CryptoAdapter asks: "Who provides crypto.signature.ed25519?"
// 2. DiscoveryEngine searches:
//    - Environment: TOADSTOOL_CRYPTO_SERVICE_URL
//    - mDNS: _crypto-service._tcp.local
//    - Service mesh: Query Consul/K8s
//    - Config: ~/.toadstool/services.toml
// 3. Finds: "Some service at 10.0.0.5:9876 provides this"
// 4. Negotiates protocol: "Service speaks gRPC"
// 5. Invokes: gRPC call to 10.0.0.5:9876
// 6. Returns result
//
// ToadStool never knew it was BearDog! Could be any crypto service.
```

---

## 🎯 SUCCESS CRITERIA

### Phase 1 Complete When:
- [x] Infant discovery engine exists (DONE - already in codebase)
- [ ] CapabilityRegistry created
- [ ] UniversalServiceAdapter created
- [ ] Protocol negotiation framework created

### Phase 2 Complete When:
- [ ] CryptoAdapter replaces beardog.rs
- [ ] StorageAdapter replaces nestgate.rs
- [ ] CoordinationAdapter replaces songbird.rs
- [ ] All service-specific files deprecated

### Phase 3 Complete When:
- [ ] get_standard_service_ports() removed
- [ ] All hardcoded ports eliminated
- [ ] Environment-based discovery working
- [ ] mDNS discovery working

### Phase 4 Complete When:
- [ ] No vendor names in code (k8s, consul, etc.)
- [ ] InfrastructureProvider abstraction
- [ ] Dynamic vendor detection
- [ ] Zero hardcoded service names

### Final Validation:
- [ ] Can deploy ToadStool with zero config
- [ ] Discovers all services automatically
- [ ] Works with BearDog, alternative crypto service, or both
- [ ] Works with NestGate, Ceph, S3, or any storage
- [ ] Works with Songbird, Consul, etcd, or any coordinator
- [ ] No code changes needed to swap services

---

## 📊 IMPACT ANALYSIS

### Files to Modify (Phase 1-2)
1. Create: `crates/cli/src/ecosystem/capabilities/` (new)
2. Create: `crates/cli/src/ecosystem/adapters/` (new)
3. Create: `crates/cli/src/ecosystem/protocols/` (new)
4. Deprecate: `crates/cli/src/ecosystem/services/*.rs`
5. Refactor: `crates/cli/src/ecosystem/discovery.rs`
6. Refactor: `crates/cli/src/ecosystem/types.rs`

### Breaking Changes
- ❌ Direct service imports will break:
  ```rust
  use toadstool::ecosystem::services::beardog;  // Will be deprecated
  ```

- ✅ New capability-based imports:
  ```rust
  use toadstool::ecosystem::adapters::CryptoAdapter;
  ```

### Migration Path for Users
```rust
// OLD CODE (still works with deprecation warnings)
let beardog = connect_beardog("127.0.0.1:8081").await?;

// NEW CODE (recommended)
let crypto = ecosystem.adapter::<CryptoAdapter>().await?;
let verified = crypto.verify_signature(...).await?;
```

---

## 🚦 ROLLOUT PLAN

### Week 1: Foundation
- Day 1-2: Create capability registry and taxonomy
- Day 3-4: Implement UniversalServiceAdapter
- Day 5: Protocol negotiation framework

### Week 2: Migration
- Day 1-2: Create CryptoAdapter, deprecate beardog.rs
- Day 3: Create StorageAdapter, deprecate nestgate.rs
- Day 4: Create CoordinationAdapter, deprecate songbird.rs
- Day 5: Remove port hardcoding

### Week 3: Vendor Abstraction
- Day 1-2: Create InfrastructureProvider abstraction
- Day 3-4: Implement dynamic vendor detection
- Day 5: Final cleanup and testing

### Week 4: Validation & Documentation
- Day 1-2: E2E testing with multiple service combinations
- Day 3-4: Update documentation
- Day 5: Migration guide for users

---

## 💡 PHILOSOPHICAL ALIGNMENT

This migration embodies the core ecoPrimals principles:

1. **Human Dignity**: Services don't dictate; they offer capabilities
2. **Sovereignty**: Users can swap any service without code changes
3. **Infant Discovery**: Start with zero knowledge, learn everything
4. **Universal Compatibility**: Works with any provider of a capability
5. **Zero Hardcoding**: No assumptions about what exists

**Result**: A truly universal, vendor-agnostic, future-proof system where ToadStool can work with services that don't even exist yet, as long as they provide the right capabilities.

---

**Status**: 🟡 **PLANNING COMPLETE - READY FOR IMPLEMENTATION**  
**Next Step**: Begin Phase 1, Step 1.1 (Create CapabilityRegistry)  
**Owner**: Development Team  
**Timeline**: 3-4 weeks to completion

