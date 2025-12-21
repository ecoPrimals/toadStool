# 🏗️ Capability-Based Architecture

**Date**: November 18, 2025  
**Status**: ✅ Implemented (75% complete)  
**Version**: 1.0.0

---

## 📐 System Architecture

### Before: Hardcoded Point-to-Point (N² Problem)

```
┌──────────────────────────────────────────────────────────────┐
│  BEFORE: Each primal "knows" the others by name             │
│  Problem: N² connections, hardcoded names                    │
└──────────────────────────────────────────────────────────────┘

    ToadStool
        │
        ├─────────→ BearDog (hardcoded "beardog.local:8081")
        │
        ├─────────→ NestGate (hardcoded "nestgate.local:8082")
        │
        └─────────→ Songbird (hardcoded "songbird.local:8080")

Problems:
  ❌ ToadStool "knows" BearDog, NestGate, Songbird by name
  ❌ Can't work with AWS KMS, Ceph, Consul, etc.
  ❌ N² connections if we add more primals
  ❌ Code changes required to switch providers
  ❌ Violates infant discovery principle
```

### After: Universal Adapter Pattern (N Connections)

```
┌──────────────────────────────────────────────────────────────┐
│  AFTER: Discovery by capability, not by name                 │
│  Solution: Universal adapter, O(N) connections               │
└──────────────────────────────────────────────────────────────┘

                        ToadStool
                            │
                            ▼
                ┌───────────────────────┐
                │ UniversalServiceAdapter│
                │  (Capability Registry) │
                └───────────────────────┘
                            │
        ┌───────────────────┼───────────────────┐
        ▼                   ▼                   ▼
   CryptoAdapter      StorageAdapter    CoordinationAdapter
        │                   │                   │
        ▼                   ▼                   ▼
    
    "crypto"            "storage"         "coordination"
    capability          capability         capability
        │                   │                   │
        ▼                   ▼                   ▼
        
   ANY Provider        ANY Provider       ANY Provider
   - BearDog           - NestGate         - Songbird
   - AWS KMS           - Ceph             - Consul
   - Vault             - GlusterFS        - etcd
   - HSM               - S3/MinIO         - Kubernetes
   - Custom            - Custom           - Custom

Benefits:
  ✅ ToadStool knows ZERO primal names
  ✅ Works with any provider of capabilities
  ✅ O(N) connections via universal adapter
  ✅ Zero code changes to switch providers
  ✅ True infant discovery
  ✅ Automatic failover
```

---

## 🔄 Discovery Flow

### 1. Environment Variables (Priority 1, Highest)

```
┌─────────────────────────────────────────────────────────────┐
│ Step 1: Check Environment Variables                         │
└─────────────────────────────────────────────────────────────┘

$ export TOADSTOOL_CRYPTO_SERVICE_URL="http://beardog.local:9876"
$ export TOADSTOOL_STORAGE_SERVICE_URL="http://nestgate.local:8082"
$ export TOADSTOOL_COORDINATION_SERVICE_URL="http://songbird.local:8080"

ToadStool App
    │
    ▼
discover_from_environment("crypto")
    │
    ├─→ Check: $TOADSTOOL_CRYPTO_SERVICE_URL
    │   └─→ Found: "http://beardog.local:9876"
    │       └─→ Return endpoint ✅
    │
    └─→ Not found? Try next method...
```

### 2. Configuration Files (Priority 2)

```
┌─────────────────────────────────────────────────────────────┐
│ Step 2: Check Configuration Files                           │
└─────────────────────────────────────────────────────────────┘

Config Locations (in order):
  1. ~/.toadstool/services.toml
  2. ./.toadstool/config.toml
  3. /etc/toadstool/services.toml

~/.toadstool/services.toml:
┌────────────────────────────────────────────────┐
│ [services.crypto]                              │
│ url = "http://beardog.local:9876"              │
│ priority = 90                                  │
│                                                 │
│ [services.storage]                             │
│ url = "http://nestgate.local:8082"             │
│ priority = 80                                  │
└────────────────────────────────────────────────┘

ToadStool App
    │
    ▼
discover_from_config("crypto")
    │
    ├─→ Read: ~/.toadstool/services.toml
    │   └─→ Parse: [services.crypto]
    │       └─→ Return: "http://beardog.local:9876" ✅
    │
    └─→ Not found? Try next method...
```

### 3. mDNS Discovery (Priority 3) [Coming Soon]

```
┌─────────────────────────────────────────────────────────────┐
│ Step 3: mDNS/Bonjour Discovery                              │
└─────────────────────────────────────────────────────────────┘

Network:
┌────────────────────────────────────────────────┐
│ 10.0.0.5  BearDog                              │
│   Advertises: _crypto-service._tcp.local       │
│                                                 │
│ 10.0.0.6  NestGate                             │
│   Advertises: _storage-service._tcp.local      │
│                                                 │
│ 10.0.0.7  Songbird                             │
│   Advertises: _coord-service._tcp.local        │
└────────────────────────────────────────────────┘

ToadStool App
    │
    ▼
discover_via_mdns("crypto")
    │
    ├─→ Query: _crypto-service._tcp.local
    │   └─→ Response: 10.0.0.5:9876 (BearDog)
    │       └─→ Return endpoint ✅
    │
    └─→ Not found? Try next method...
```

### 4. Service Mesh (Priority 4) [Coming Soon]

```
┌─────────────────────────────────────────────────────────────┐
│ Step 4: Service Mesh Query                                  │
└─────────────────────────────────────────────────────────────┘

Service Mesh Options:
  - Consul
  - etcd
  - Kubernetes Service Discovery

ToadStool App
    │
    ▼
discover_via_service_mesh("crypto")
    │
    ├─→ Query: Consul for "crypto" capability
    │   └─→ Response: beardog.service.consul:9876
    │       └─→ Return endpoint ✅
    │
    └─→ Not found? Return error ❌
```

### 5. Complete Discovery Flow

```
┌─────────────────────────────────────────────────────────────┐
│ Complete Discovery Chain                                     │
└─────────────────────────────────────────────────────────────┘

discover_service_by_capability("crypto")
    │
    ├─→ (1) Check Environment Variables
    │   ├─→ Found? Return ✅
    │   └─→ Not found? Continue...
    │
    ├─→ (2) Check Config Files
    │   ├─→ Found? Return ✅
    │   └─→ Not found? Continue...
    │
    ├─→ (3) mDNS Discovery
    │   ├─→ Found? Return ✅
    │   └─→ Not found? Continue...
    │
    └─→ (4) Service Mesh Query
        ├─→ Found? Return ✅
        └─→ Not found? Return error ❌

Result: ServiceEndpoint {
    service_type: Unknown("crypto"),
    address: 10.0.0.5:9876,
    capabilities: ["crypto"],
    trust_level: Configured,
}
```

---

## 🎯 Adapter Call Flow

### Example: Crypto Signature Verification

```
┌─────────────────────────────────────────────────────────────┐
│ User Code                                                    │
└─────────────────────────────────────────────────────────────┘

let universal = UniversalServiceAdapter::new();
let crypto = CryptoAdapter::new(&universal);

let verified = crypto.verify_signature(
    StandardCapability::CryptoSignatureEd25519,
    data, signature, public_key
).await?;

    │
    ▼

┌─────────────────────────────────────────────────────────────┐
│ CryptoAdapter                                                │
│ (crates/cli/src/ecosystem/adapters/crypto.rs)               │
└─────────────────────────────────────────────────────────────┘

pub async fn verify_signature(&self, ...) -> Result<bool> {
    // 1. Discover service by capability
    let endpoint = self.universal.discover(capability).await?;
    
    // 2. Build request parameters
    let params = json!({
        "data": base64::encode(data),
        "signature": base64::encode(signature),
        "public_key": base64::encode(public_key),
    });
    
    // 3. Invoke via universal adapter
    let response = self.universal.invoke(capability, params).await?;
    
    // 4. Parse response
    Ok(response["verified"].as_bool().unwrap_or(false))
}

    │
    ▼

┌─────────────────────────────────────────────────────────────┐
│ UniversalServiceAdapter                                      │
│ (crates/cli/src/ecosystem/adapters/universal.rs)            │
└─────────────────────────────────────────────────────────────┘

pub async fn discover(&self, capability) -> Result<ServiceEndpoint> {
    // 1. Check registry cache
    if let Some(cached) = self.registry.get_provider(capability) {
        return Ok(cached);
    }
    
    // 2. Discover from environment, config, mDNS, service mesh
    let endpoint = discover_service_by_capability(
        &capability.to_string()
    ).await?;
    
    // 3. Cache result
    self.registry.register_provider(capability, endpoint.clone())?;
    
    // 4. Return endpoint
    Ok(endpoint)
}

pub async fn invoke(&self, capability, params) -> Result<Value> {
    // 1. Get endpoint
    let endpoint = self.discover(capability).await?;
    
    // 2. Build HTTP request
    let url = format!("http://{}/api/v1/{}", 
        endpoint.address,
        capability.endpoint_path()
    );
    
    // 3. Send request
    let response = reqwest::Client::new()
        .post(&url)
        .json(&params)
        .send()
        .await?;
    
    // 4. Parse response
    Ok(response.json().await?)
}

    │
    ▼

┌─────────────────────────────────────────────────────────────┐
│ CapabilityResolver                                           │
│ (crates/cli/src/ecosystem/capabilities/resolver.rs)         │
└─────────────────────────────────────────────────────────────┘

pub async fn resolve(&self, capability) -> Result<ServiceEndpoint> {
    // Discovery chain (priority order):
    // 1. Environment variables
    // 2. Configuration files
    // 3. mDNS discovery
    // 4. Service mesh
    
    discover_service_by_capability(&capability).await
}

    │
    ▼

┌─────────────────────────────────────────────────────────────┐
│ Discovery Module                                             │
│ (crates/cli/src/ecosystem/discovery.rs)                     │
└─────────────────────────────────────────────────────────────┘

pub async fn discover_service_by_capability(
    capability: &str
) -> Result<Vec<ServiceEndpoint>> {
    // Try each discovery method in priority order
    
    if let Some(url) = discover_from_environment(capability) {
        return Ok(vec![parse_endpoint(url)?]);
    }
    
    if let Some(url) = discover_from_config(capability) {
        return Ok(vec![parse_endpoint(url)?]);
    }
    
    if let Ok(endpoints) = discover_via_mdns(capability).await {
        if !endpoints.is_empty() {
            return Ok(endpoints);
        }
    }
    
    // No service found
    Err(anyhow!("No service found for capability: {}", capability))
}

    │
    ▼

┌─────────────────────────────────────────────────────────────┐
│ Network Request to Service Provider                         │
│ (e.g., BearDog, AWS KMS, Vault, HSM)                        │
└─────────────────────────────────────────────────────────────┘

POST http://beardog.local:9876/api/v1/crypto/signature/verify
Content-Type: application/json

{
  "data": "SGVsbG8gV29ybGQ=",
  "signature": "c2lnbmF0dXJl...",
  "public_key": "cHVibGljS2V5...",
  "algorithm": "ed25519"
}

    │
    ▼

Response:
{
  "verified": true,
  "algorithm": "ed25519",
  "timestamp": "2025-11-18T22:30:00Z"
}

    │
    ▼

Return to User Code: verified = true ✅
```

---

## 📊 Component Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                      USER APPLICATION                            │
│  (e.g., ToadStool CLI, ToadStool Server, Custom App)            │
└─────────────────────────────────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│                    ADAPTER LAYER                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │CryptoAdapter │  │StorageAdapter│  │CoordinationA.│          │
│  │              │  │              │  │              │          │
│  │ - sign       │  │ - mount      │  │ - register   │          │
│  │ - verify     │  │ - store      │  │ - discover   │          │
│  │ - encrypt    │  │ - retrieve   │  │ - publish    │          │
│  │ - decrypt    │  │ - snapshot   │  │ - health     │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
└─────────────────────────────────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│               UNIVERSAL SERVICE ADAPTER                          │
│  ┌───────────────────────────────────────────────┐              │
│  │  - discover(capability) -> ServiceEndpoint    │              │
│  │  - invoke(capability, params) -> Response     │              │
│  │  - has_capability(capability) -> bool         │              │
│  └───────────────────────────────────────────────┘              │
└─────────────────────────────────────────────────────────────────┘
                               │
                ┌──────────────┼──────────────┐
                ▼              ▼              ▼
┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐
│CapabilityRegistry│  │CapabilityResolver│  │CapabilityTaxonomy│
│                  │  │                  │  │                  │
│ - register       │  │ - resolve        │  │ - 70+ standard   │
│ - get_provider   │  │ - prioritize     │  │   capabilities   │
│ - list_all       │  │ - failover       │  │ - categorized    │
└──────────────────┘  └──────────────────┘  └──────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│                   DISCOVERY LAYER                                │
│  ┌──────────────────────────────────────────────┐               │
│  │  discover_service_by_capability(capability)  │               │
│  │    ├─→ (1) Environment Variables             │               │
│  │    ├─→ (2) Configuration Files               │               │
│  │    ├─→ (3) mDNS Discovery                    │               │
│  │    └─→ (4) Service Mesh                      │               │
│  └──────────────────────────────────────────────┘               │
└─────────────────────────────────────────────────────────────────┘
                               │
                ┌──────────────┼──────────────┐
                ▼              ▼              ▼
┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐
│   Config Files   │  │   Environment    │  │   Network        │
│                  │  │   Variables      │  │   Discovery      │
│ ~/.toadstool/    │  │                  │  │                  │
│ services.toml    │  │ TOADSTOOL_*_URL  │  │ - mDNS           │
│                  │  │                  │  │ - Service Mesh   │
└──────────────────┘  └──────────────────┘  └──────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│                  SERVICE PROVIDERS                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │   BearDog    │  │   NestGate   │  │   Songbird   │          │
│  │   AWS KMS    │  │   Ceph       │  │   Consul     │          │
│  │   Vault      │  │   GlusterFS  │  │   etcd       │          │
│  │   HSM        │  │   S3/MinIO   │  │   Kubernetes │          │
│  │   Custom     │  │   Custom     │  │   Custom     │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
└─────────────────────────────────────────────────────────────────┘
```

---

## 🔐 Security Model

### Trust Levels

```
┌─────────────────────────────────────────────────────────────────┐
│  Trust Hierarchy (lowest to highest)                            │
└─────────────────────────────────────────────────────────────────┘

TrustLevel::Unknown (0)
  └─→ Service discovered but not verified
      - No credentials exchanged
      - Use with extreme caution

TrustLevel::Discovered (1)
  └─→ Service found via mDNS or service mesh
      - Identity not verified
      - May be malicious

TrustLevel::Advertised (2)
  └─→ Service actively advertising capabilities
      - Claims checked but not verified
      - Moderate risk

TrustLevel::Configured (3)
  └─→ Service in configuration file or environment
      - Administrator trust
      - Low risk

TrustLevel::Verified (4)
  └─→ Service cryptographically verified
      - Signature checked
      - Very low risk

TrustLevel::Sovereign (5) [Coming Soon]
  └─→ Service verified via sovereign identity
      - Full trust chain
      - Zero risk
```

### Capability-Based Security

```
┌─────────────────────────────────────────────────────────────────┐
│  Principle: Grant Capabilities, Not Service Access              │
└─────────────────────────────────────────────────────────────────┘

❌ OLD WAY: Service-Based Permissions
  User has access to: "BearDog"
  → Grants ALL BearDog capabilities (too broad!)

✅ NEW WAY: Capability-Based Permissions
  User has access to:
    - crypto.signature.ed25519
    - crypto.encryption.aes256
  → Grants ONLY specific capabilities (principle of least privilege!)

┌────────────────────────────────────────────────────────┐
│ Example: Workload Permissions                          │
├────────────────────────────────────────────────────────┤
│                                                         │
│  Workload "analytics-job" requests:                    │
│    ✅ storage.distributed (read-only)                  │
│    ✅ compute.cpu (8 cores)                            │
│    ❌ crypto.signature.ed25519 (denied)                │
│                                                         │
│  → Analytics job CAN'T sign transactions               │
│  → Principle of least privilege enforced               │
│                                                         │
└────────────────────────────────────────────────────────┘
```

---

## 📈 Scalability

### Before: N² Problem

```
With 5 primals, each hardcoding the others:
  ToadStool → BearDog, NestGate, Songbird, Squirrel
  BearDog   → ToadStool, NestGate, Songbird, Squirrel
  NestGate  → ToadStool, BearDog, Songbird, Squirrel
  Songbird  → ToadStool, BearDog, NestGate, Squirrel
  Squirrel  → ToadStool, BearDog, NestGate, Songbird

Total connections: 5 * 4 = 20 (N²)
Total code: 20 hardcoded modules

With 10 primals: 10 * 9 = 90 connections!
With 20 primals: 20 * 19 = 380 connections!
```

### After: O(N) with Universal Adapter

```
With 5 primals, each using universal adapter:
  ToadStool → Universal Adapter → ANY crypto/storage/coord
  BearDog   → Universal Adapter → ANY compute/storage/coord
  NestGate  → Universal Adapter → ANY compute/crypto/coord
  Songbird  → Universal Adapter → ANY compute/crypto/storage
  Squirrel  → Universal Adapter → ANY compute/crypto/storage

Total connections: 5 (N)
Total code: 1 universal adapter + N capability adapters

With 10 primals: 10 connections
With 20 primals: 20 connections

Reduction: O(N²) → O(N)
```

---

## 🎓 Design Patterns

### 1. Adapter Pattern
- **Adapters** wrap UniversalServiceAdapter
- **Purpose**: Provide domain-specific APIs

### 2. Registry Pattern
- **CapabilityRegistry** caches discovered services
- **Purpose**: Avoid repeated discovery

### 3. Strategy Pattern
- **Discovery methods** are strategies
- **Purpose**: Flexible discovery

### 4. Chain of Responsibility
- **Discovery chain** tries methods in order
- **Purpose**: Resilient discovery

### 5. Dependency Injection
- **Adapters** receive UniversalServiceAdapter
- **Purpose**: Testability, flexibility

---

## ✅ Completion Status

### Phase 1: Capability Infrastructure (100% ✅)
- [x] Capability taxonomy (70+ capabilities)
- [x] Capability registry
- [x] Capability resolver

### Phase 2: Service Migration (100% ✅)
- [x] Universal service adapter
- [x] Crypto adapter (replaces beardog.rs)
- [x] Storage adapter (replaces nestgate.rs)
- [x] Coordination adapter (replaces songbird.rs)

### Phase 3: Discovery Enhancement (75% ✅)
- [x] Environment variable discovery
- [x] Configuration file discovery
- [ ] mDNS discovery (25% remaining)
- [ ] Service mesh integration

### Phase 4: Vendor Abstraction (0%)
- [ ] K8s abstraction
- [ ] Consul abstraction
- [ ] Cloud provider abstraction

---

**Last Updated**: November 18, 2025  
**Architecture Version**: 1.0.0  
**Status**: ✅ Production Ready (75%)

🍄 **ToadStool: Truly Universal Compute, Zero Hardcoding.** 🚀

