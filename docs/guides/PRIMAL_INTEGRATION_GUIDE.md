# 🔗 Primal Integration Guide

**Date**: January 14, 2026  
**Status**: ✅ **ACTIVE**  
**Purpose**: Runtime integration with ecoPrimals and external systems

---

## 🎯 Deep Debt Principles

### 1. **Self-Knowledge Only**
ToadStool knows only itself. Other services discovered at runtime.

### 2. **Runtime Discovery**
No compile-time coupling. All integration points resolved at deployment.

### 3. **No Hardcoding**
Zero hardcoded addresses, ports, or service names.

### 4. **Capability-Based**
Discover services by what they do, not by what they're called.

### 5. **Graceful Degradation**
Works optimally with available services, degrades gracefully without them.

---

## 🦈 bearDog Integration (Encryption)

### Purpose
- Encryption/decryption operations
- Key management
- Cryptographic operations
- Security attestation

### Discovery

```rust
use toadstool_common::primal_integration::*;

// Automatic runtime discovery (capability-based)
let beardog = discover_encryption_service()?;

// Or discover by capability (development: set TOADSTOOL_ENCRYPTION_ENDPOINT or TOADSTOOL_CRYPTO_SERVICE_SUBDIR)
let beardog = discover_service_by_capability("encryption")?;
```

### Environment Configuration

```bash
# Production
export TOADSTOOL_ENCRYPTION_ENDPOINT=http://beardog.prod:6060
export BEARDOG_ENDPOINT=http://beardog.prod:6060

# Development (Docker Compose)
export TOADSTOOL_ENCRYPTION_ENDPOINT=http://beardog:6060

# Local development (filesystem)
# Discovers ../beardog/ automatically
```

### Usage Pattern

```rust
// Example: Encrypt sensitive data before storage
async fn store_sensitive_data(data: &[u8]) -> Result<()> {
    // Discover bearDog at runtime
    let encryption_service = match discover_encryption_service().await {
        Ok(endpoints) => endpoints.into_iter().next()
            .ok_or_else(|| Error::msg("No endpoints found"))?,
        Err(DiscoveryError::NoServiceFound { .. }) => {
            // Graceful degradation: Store unencrypted with warning
            tracing::warn!("bearDog not available, storing without encryption");
            return store_unencrypted(data).await;
        }
        Err(e) => return Err(e.into()),
    };
    
    // Use encryption
    let encrypted = call_encryption_service(&encryption_service.url, data).await?;
    store_encrypted(encrypted).await
}
```

---

## 🗄️ nestGate Integration (Compression/Persistence)

### Purpose
- Data compression (zstd, lz4, gzip)
- Persistent storage
- Blob storage
- Object storage interface

### Discovery

```rust
use toadstool_common::primal_integration::*;

// Automatic runtime discovery (capability-based)
let nestgate = discover_storage_service()?;

// Or discover by capability (development: set TOADSTOOL_STORAGE_ENDPOINT or TOADSTOOL_STORAGE_SERVICE_SUBDIR)
let nestgate = discover_service_by_capability("storage")?;
```

### Environment Configuration

```bash
# Production
export TOADSTOOL_STORAGE_ENDPOINT=http://nestgate.prod:8080
export NESTGATE_ENDPOINT=http://nestgate.prod:8080

# Development (Docker Compose)
export TOADSTOOL_STORAGE_ENDPOINT=http://nestgate:8080

# Local development (filesystem)
# Discovers ../nestgate/ automatically
```

### Usage Pattern - Compression

```rust
// Example: Compress data before storage
async fn compress_and_store(data: &[u8]) -> Result<()> {
    // Discover nestGate at runtime
    let storage_service = match discover_storage_service().await {
        Ok(endpoints) => endpoints.into_iter().next()
            .ok_or_else(|| Error::msg("No endpoints found"))?,
        Err(DiscoveryError::NoServiceFound { .. }) => {
            // Graceful degradation: Store uncompressed
            tracing::warn!("nestGate not available, storing without compression");
            return store_uncompressed(data).await;
        }
        Err(e) => return Err(e.into()),
    };
    
    // Use compression capability
    let compressed = call_compression_service(&storage_service.url, data).await?;
    store_compressed(compressed).await
}
```

### Usage Pattern - Persistence

```rust
// Example: Store results persistently
async fn persist_computation_result(key: &str, result: &[u8]) -> Result<()> {
    // Discover nestGate storage
    let storage_service = discover_storage_service().await?;
    
    // Use persistence capability
    call_storage_service(
        &storage_service.first().ok_or("No storage service")?.url,
        key,
        result
    ).await
}
```

---

## 🕊️ songBird Integration (Coordination)

### Purpose
- Service discovery
- Capability registration
- Health monitoring
- Load balancing

### Discovery

```rust
use toadstool_common::primal_integration::*;

// Automatic runtime discovery
let songbird = discover_coordination_service().await?;
```

### Environment Configuration

```bash
# Production
export TOADSTOOL_COORDINATION_ENDPOINT=http://songbird.prod:9090
export SONGBIRD_ENDPOINT=http://songbird.prod:9090

# Development (Docker Compose)
export TOADSTOOL_COORDINATION_ENDPOINT=http://songbird:9090
```

### Usage Pattern

```rust
// Example: Register ToadStool capabilities with songBird
async fn register_with_coordinator() -> Result<()> {
    // Discover songBird at runtime
    let coordinator = match discover_coordination_service().await {
        Ok(endpoints) => endpoints.into_iter().next()
            .ok_or_else(|| Error::msg("No coordinator found"))?,
        Err(DiscoveryError::NoServiceFound { .. }) => {
            // Graceful degradation: Run standalone
            tracing::warn!("songBird not available, running in standalone mode");
            return Ok(());
        }
        Err(e) => return Err(e.into()),
    };
    
    // Register our capabilities
    register_capabilities(&coordinator.url, &get_local_capabilities()).await
}
```

---

## 🐿️ squirrel Integration (MCP/Agents)

### Purpose
- MCP server hosting
- Agent orchestration
- Plugin execution
- Tool discovery

### Discovery

```rust
use toadstool_common::primal_integration::*;

// Automatic runtime discovery
let squirrel = discover_mcp_service().await?;
```

### Usage Pattern

```rust
// Example: Execute MCP tool via squirrel
async fn execute_mcp_tool(tool_name: &str, params: serde_json::Value) -> Result<()> {
    // Discover squirrel MCP platform
    let mcp = match discover_mcp_service().await {
        Ok(endpoints) => endpoints.into_iter().next()
            .ok_or_else(|| Error::msg("No MCP service found"))?,
        Err(DiscoveryError::NoServiceFound { .. }) => {
            tracing::warn!("squirrel not available, tool execution unavailable");
            return Err(Error::msg("MCP not available"));
        }
        Err(e) => return Err(e.into()),
    };
    
    // Execute tool
    call_mcp_tool(&mcp.url, tool_name, params).await
}
```

---

## 🌐 External System Integration

### Redis Integration

```rust
// Discover Redis at runtime
let cache = discover_cache_service().await?;

// Use standard Redis client with discovered endpoint
let client = redis::Client::open(cache.first().unwrap().url.as_str())?;
```

Environment:
```bash
export TOADSTOOL_CACHE_ENDPOINT=redis://redis.prod:6379
```

### PostgreSQL Integration

```rust
// Discover PostgreSQL at runtime
let db = discover_database_service().await?;

// Use sqlx with discovered endpoint
let pool = sqlx::postgres::PgPool::connect(&db.first().unwrap().url).await?;
```

Environment:
```bash
export TOADSTOOL_DATABASE_ENDPOINT=postgresql://user:pass@postgres.prod:5432/dbname
```

### S3-Compatible Storage

```rust
// Discover S3 at runtime
let storage = discover_object_storage().await?;

// Use s3 client with discovered endpoint
let config = s3::Config::builder()
    .endpoint(&storage.first().unwrap().url)
    .build();
```

Environment:
```bash
export TOADSTOOL_OBJECT_STORAGE_ENDPOINT=https://s3.amazonaws.com
```

---

## 📋 Discovery Priority

Discovery attempts methods in this order:

1. **Environment Variables** (highest priority)
   - `TOADSTOOL_{CAPABILITY}_ENDPOINT`
   - `{PRIMAL}_ENDPOINT`
   - `TOADSTOOL_SERVICE_{NAME}_URL`

2. **mDNS/DNS-SD** (local network)
   - `_encryption._tcp.local.` (bearDog)
   - `_storage._tcp.local.` (nestGate)
   - `_coordination._tcp.local.` (songBird)

3. **Kubernetes Service Discovery**
   - DNS: `beardog.default.svc.cluster.local`
   - DNS: `nestgate.default.svc.cluster.local`
   - DNS: `songbird.default.svc.cluster.local`

4. **Docker Compose Service Names**
   - `http://beardog:6060`
   - `http://nestgate:8080`
   - `http://songbird:9090`

5. **Runtime Registry** (consul, etcd)
   - Query by capability tag
   - Load balance across instances

6. **Filesystem Discovery** (development)
   - Set `TOADSTOOL_CRYPTO_SERVICE_SUBDIR` / `TOADSTOOL_STORAGE_SERVICE_SUBDIR` for custom paths
   - Or use `TOADSTOOL_ENCRYPTION_ENDPOINT` / `TOADSTOOL_STORAGE_ENDPOINT` for direct URLs

---

## ✅ Best Practices

### 1. Always Use Discovery

```rust
// ❌ BAD: Hardcoded
let url = "http://localhost:8080";

// ✅ GOOD: Runtime discovery
let service = discover_storage_service().await?;
let url = &service.first().unwrap().url;
```

### 2. Handle Missing Services Gracefully

```rust
// ✅ GOOD: Graceful degradation
let encrypted = match discover_encryption_service().await {
    Ok(service) => encrypt_with_service(data, &service).await?,
    Err(_) => {
        tracing::warn!("Encryption unavailable, storing plaintext");
        data.to_vec()
    }
};
```

### 3. Cache Discovery Results

```rust
// ✅ GOOD: Cache and reuse
lazy_static! {
    static ref STORAGE_SERVICE: Mutex<Option<PrimalEndpoint>> = Mutex::new(None);
}

async fn get_storage() -> Result<PrimalEndpoint> {
    let mut cache = STORAGE_SERVICE.lock().unwrap();
    if cache.is_none() {
        *cache = Some(discover_storage_service().await?.into_iter().next().unwrap());
    }
    Ok(cache.as_ref().unwrap().clone())
}
```

### 4. Health Check Discovered Services

```rust
// ✅ GOOD: Verify health before use
let service = discover_storage_service().await?;
if !check_health(&service.first().unwrap().url).await? {
    return Err(Error::msg("Service unhealthy"));
}
```

---

## 🧪 Testing with Mocks

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_with_mock_encryption() {
        // Set test environment
        std::env::set_var(
            "TOADSTOOL_ENCRYPTION_ENDPOINT",
            "http://mock-beardog:6060"
        );
        
        // Discovery will find mock
        let service = discover_encryption_service().await.unwrap();
        assert_eq!(service[0].url, "http://mock-beardog:6060");
        
        // Cleanup
        std::env::remove_var("TOADSTOOL_ENCRYPTION_ENDPOINT");
    }
}
```

---

## 📊 Status

| Integration | Discovery | Usage | Status |
|-------------|-----------|-------|--------|
| bearDog (encryption) | ✅ Implemented | 🚧 In progress | Active |
| nestGate (storage) | ✅ Implemented | 🚧 In progress | Active |
| songBird (coordination) | ✅ Implemented | ✅ Complete | Active |
| squirrel (MCP) | ✅ Implemented | 📋 Planned | Active |
| Redis (cache) | ✅ Implemented | 📋 Planned | Active |
| PostgreSQL (database) | ✅ Implemented | 📋 Planned | Active |
| S3 (object storage) | ✅ Implemented | 📋 Planned | Active |

---

**"Discover at runtime, not compile time."** 🔍✨
