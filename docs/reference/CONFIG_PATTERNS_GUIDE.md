# 🎯 Configuration Patterns Guide
**Date**: November 8, 2025  
**Status**: Active Guide for Configuration Best Practices  
**Phase 5 Status**: 77% Complete (Base configs implemented)

---

## 📋 Table of Contents

1. [Overview](#overview)
2. [Base Configuration Pattern](#base-configuration-pattern)
3. [Available Base Configs](#available-base-configs)
4. [Usage Examples](#usage-examples)
5. [Migration Guide](#migration-guide)
6. [Best Practices](#best-practices)
7. [Anti-Patterns](#anti-patterns)

---

## Overview

ToadStool uses a **base configuration composition pattern** to achieve:
- ✅ **Code reuse**: Common config patterns defined once
- ✅ **Consistency**: Same fields mean the same thing everywhere
- ✅ **Maintainability**: Change one place, update everywhere
- ✅ **Type safety**: Compiler catches configuration errors

### Architecture

```
┌─────────────────────────────────────────────────────────┐
│  Base Configs (toadstool_common::config_bases)         │
│  - TimeoutConfig, RetryConfig, HealthCheckConfig, etc. │
└─────────────────────────────────────────────────────────┘
                          ▲
                          │ Compose via #[serde(flatten)]
                          │
┌─────────────────────────────────────────────────────────┐
│  Domain-Specific Configs                                 │
│  - NetworkConfig, RuntimeConfig, SecurityConfig, etc.    │
└─────────────────────────────────────────────────────────┘
```

---

## Base Configuration Pattern

### The `#[serde(flatten)]` Pattern

Base configs use Serde's `flatten` attribute to compose configuration structs:

```rust
use serde::{Deserialize, Serialize};
use toadstool_common::config_bases::{TimeoutConfig, RetryConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyServiceConfig {
    // Domain-specific fields
    pub service_name: String,
    pub endpoint: String,
    
    // Embedded base configs (flattened into parent struct)
    #[serde(flatten)]
    pub timeouts: TimeoutConfig,
    
    #[serde(flatten)]
    pub retries: RetryConfig,
}
```

**Result in TOML/JSON**:
```toml
service_name = "my-service"
endpoint = "http://localhost:8080"

# TimeoutConfig fields (flattened)
connection_timeout = "30s"
request_timeout = "60s"
read_timeout = "30s"
write_timeout = "30s"

# RetryConfig fields (flattened)
max_retries = 3
base_delay = "100ms"
max_delay = "30s"
backoff_multiplier = 2.0
jitter_percent = 10.0
```

---

## Available Base Configs

### 📍 Location
All base configs are in: `crates/core/common/src/config_bases.rs`

### 1. **TimeoutConfig** ⏱️

Standard timeout configuration for network operations.

```rust
pub struct TimeoutConfig {
    pub connection_timeout: Duration,  // Default: 30s
    pub request_timeout: Duration,     // Default: 60s
    pub read_timeout: Duration,        // Default: 30s
    pub write_timeout: Duration,       // Default: 30s
}
```

**When to use**: Any network client, HTTP service, database connection

---

### 2. **RetryConfig** 🔄

Retry configuration with exponential backoff and jitter.

```rust
pub struct RetryConfig {
    pub max_retries: u32,              // Default: 3
    pub base_delay: Duration,          // Default: 100ms
    pub max_delay: Duration,           // Default: 30s
    pub backoff_multiplier: f64,       // Default: 2.0
    pub jitter_percent: f64,           // Default: 10.0
}
```

**When to use**: Any retriable operation (HTTP requests, database queries, file operations)

---

### 3. **HealthCheckConfig** 💚

Base health check configuration for any health monitoring.

```rust
pub struct HealthCheckConfig {
    pub enabled: bool,                 // Default: true
    pub interval: Duration,            // Default: 10s
    pub timeout: Duration,             // Default: 5s
    pub healthy_threshold: u32,        // Default: 2
    pub unhealthy_threshold: u32,      // Default: 3
    pub retry_count: u32,              // Default: 1
}
```

**When to use**: Service health checks, dependency monitoring

---

### 4. **HttpHealthCheckConfig** 🌐

HTTP-specific health check (extends `HealthCheckConfig`).

```rust
pub struct HttpHealthCheckConfig {
    #[serde(flatten)]
    pub base: HealthCheckConfig,
    pub path: String,                  // Default: "/health"
    pub expected_status: u16,          // Default: 200
    pub method: String,                // Default: "GET"
}
```

**When to use**: HTTP health endpoints

---

### 5. **ConnectionPoolConfig** 🏊

Connection pooling configuration.

```rust
pub struct ConnectionPoolConfig {
    pub enabled: bool,                         // Default: true
    pub max_connections_per_host: u32,         // Default: 100
    pub max_idle_connections: u32,             // Default: 10
    pub idle_timeout: Duration,                // Default: 300s
    pub connection_lifetime: Duration,         // Default: 3600s
}
```

**When to use**: HTTP clients, database connections, gRPC clients

---

### 6. **CacheConfig** 💾

Cache configuration with TTL.

```rust
pub struct CacheConfig {
    pub enabled: bool,                 // Default: true
    pub max_entries: u32,              // Default: 10000
    pub ttl: Duration,                 // Default: 3600s
    pub cleanup_interval: Duration,    // Default: 300s
}
```

**When to use**: Any caching layer (DNS, API responses, computed values)

---

### 7. **BackendEndpoint** 🎯

Network endpoint specification.

```rust
pub struct BackendEndpoint {
    pub name: String,
    pub address: String,
    pub port: u16,
    pub enabled: bool,                 // Default: true
}
```

**Helper methods**:
```rust
impl BackendEndpoint {
    pub fn new(name: impl Into<String>, address: impl Into<String>, port: u16) -> Self;
    pub fn url(&self, scheme: &str) -> String;
}
```

**When to use**: Service discovery, load balancer backends, registry endpoints

---

### 8. **ValidationConfig** ✅

Security validation configuration.

```rust
pub struct ValidationConfig {
    pub enabled: bool,                 // Default: true
    pub validate_expiration: bool,     // Default: true
    pub clock_skew: Option<Duration>,  // Default: Some(60s)
}
```

**When to use**: Token validation, certificate validation, signature verification

---

### 9. **BaseResourceConfig** 📦

Resource limits (CPU, memory, storage).

```rust
pub struct ResourceLimit {
    pub limit: Option<String>,
    pub request: Option<String>,
}

pub struct BaseResourceConfig {
    pub cpu: ResourceLimit,
    pub memory: ResourceLimit,
    pub storage: Option<ResourceLimit>,
}
```

**When to use**: Container limits, sandbox resource limits, quota management

---

## Usage Examples

### Example 1: Simple Service Config

```rust
use serde::{Deserialize, Serialize};
use toadstool_common::config_bases::{TimeoutConfig, RetryConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyApiClientConfig {
    pub base_url: String,
    pub api_key: String,
    
    #[serde(flatten)]
    pub timeouts: TimeoutConfig,
    
    #[serde(flatten)]
    pub retries: RetryConfig,
}

impl Default for MyApiClientConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.example.com".to_string(),
            api_key: String::new(),
            timeouts: TimeoutConfig::default(),
            retries: RetryConfig::default(),
        }
    }
}
```

**TOML Configuration**:
```toml
base_url = "https://api.example.com"
api_key = "secret-key"
connection_timeout = "15s"
request_timeout = "30s"
max_retries = 5
```

---

### Example 2: Health-Checked Service

```rust
use toadstool_common::config_bases::{HttpHealthCheckConfig, ConnectionPoolConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub connection_string: String,
    
    #[serde(flatten)]
    pub pool: ConnectionPoolConfig,
    
    #[serde(flatten)]
    pub health: HttpHealthCheckConfig,
}
```

---

### Example 3: Cached Backend Service

```rust
use toadstool_common::config_bases::{BackendEndpoint, CacheConfig, RetryConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryServiceConfig {
    pub backends: Vec<BackendEndpoint>,
    
    #[serde(flatten)]
    pub cache: CacheConfig,
    
    #[serde(flatten)]
    pub retries: RetryConfig,
}

impl DiscoveryServiceConfig {
    pub fn active_backends(&self) -> impl Iterator<Item = &BackendEndpoint> {
        self.backends.iter().filter(|b| b.enabled)
    }
}
```

---

## Migration Guide

### Step 1: Identify Duplicate Config Patterns

Look for configs with fields like:
- `connection_timeout`, `request_timeout` → Use `TimeoutConfig`
- `max_retries`, `retry_delay` → Use `RetryConfig`
- `health_check_interval`, `health_check_timeout` → Use `HealthCheckConfig`

### Step 2: Add Import

```rust
use toadstool_common::config_bases::{TimeoutConfig, RetryConfig};
```

### Step 3: Replace Fields with Flattened Base Config

**Before**:
```rust
pub struct OldConfig {
    pub service_name: String,
    pub connection_timeout: Duration,
    pub request_timeout: Duration,
    pub read_timeout: Duration,
    pub write_timeout: Duration,
}
```

**After**:
```rust
pub struct NewConfig {
    pub service_name: String,
    
    #[serde(flatten)]
    pub timeouts: TimeoutConfig,
}
```

### Step 4: Update Usage Sites

**Before**:
```rust
let timeout = config.connection_timeout;
```

**After**:
```rust
let timeout = config.timeouts.connection_timeout;
```

### Step 5: Update Tests

Ensure tests use the new structure:

```rust
#[test]
fn test_config() {
    let config = NewConfig {
        service_name: "test".to_string(),
        timeouts: TimeoutConfig::default(),
    };
    assert_eq!(config.timeouts.connection_timeout, Duration::from_secs(30));
}
```

---

## Best Practices

### ✅ DO

1. **Use base configs for common patterns**
   ```rust
   #[serde(flatten)]
   pub timeouts: TimeoutConfig,  // ✅ Good
   ```

2. **Provide sensible defaults**
   ```rust
   impl Default for MyConfig {
       fn default() -> Self {
           Self {
               timeouts: TimeoutConfig::default(),  // ✅ Uses base defaults
           }
       }
   }
   ```

3. **Document domain-specific fields**
   ```rust
   /// Maximum number of concurrent connections
   /// 
   /// This limits the number of simultaneous connections to prevent
   /// resource exhaustion. Default: 100.
   pub max_connections: u32,  // ✅ Well documented
   ```

4. **Keep domain configs domain-specific**
   ```rust
   // ✅ Domain-specific, don't force into base config
   pub struct RateLimitConfig {
       pub requests_per_second: u32,
       pub burst_size: u32,
   }
   ```

---

### ❌ DON'T

1. **Don't duplicate timeout fields**
   ```rust
   // ❌ Bad: Duplicates TimeoutConfig
   pub struct BadConfig {
       pub connection_timeout: Duration,
       pub request_timeout: Duration,
   }
   ```

2. **Don't force everything into base configs**
   ```rust
   // ❌ Bad: TelemetryConfig is domain-specific
   // DON'T create a BaseTelemetryConfig
   pub struct TelemetryConfig {
       pub metrics_enabled: bool,      // Specific to monitoring
       pub tracing_endpoint: String,   // Specific to observability
   }
   ```

3. **Don't nest base configs unnecessarily**
   ```rust
   // ❌ Bad: Double nesting
   pub struct BadConfig {
       pub network: NetworkConfig {
           pub timeouts: TimeoutConfig,  // Too deep
       }
   }
   
   // ✅ Good: Flatten at top level
   pub struct GoodConfig {
       pub network_endpoint: String,
       #[serde(flatten)]
       pub timeouts: TimeoutConfig,      // Flattened
   }
   ```

---

## Anti-Patterns

### Anti-Pattern 1: Configuration God Object

**❌ Bad**:
```rust
// Don't create one giant config with everything
pub struct GodConfig {
    pub everything_for_everyone: EverythingConfig,
}
```

**✅ Good**:
```rust
// Use focused, domain-specific configs
pub struct NetworkConfig { ... }
pub struct SecurityConfig { ... }
pub struct RuntimeConfig { ... }
```

---

### Anti-Pattern 2: Reinventing Timeouts

**❌ Bad**:
```rust
pub struct ServiceConfig {
    pub my_special_timeout: Duration,      // Don't invent new timeout names
    pub another_timeout: Duration,
}
```

**✅ Good**:
```rust
pub struct ServiceConfig {
    #[serde(flatten)]
    pub timeouts: TimeoutConfig,           // Use standard names
}
```

---

### Anti-Pattern 3: Hardcoding Defaults in Code

**❌ Bad**:
```rust
let timeout = config.timeout.unwrap_or(Duration::from_secs(30));  // Hardcoded
```

**✅ Good**:
```rust
// Define defaults in the config struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_timeout")]
    pub timeout: Duration,
}

fn default_timeout() -> Duration {
    Duration::from_secs(30)
}
```

---

## Configuration File Examples

### Example: Full Service Configuration

**`config.toml`**:
```toml
[service]
name = "toadstool-compute"
version = "1.0.0"

# TimeoutConfig (flattened)
connection_timeout = "15s"
request_timeout = "30s"
read_timeout = "20s"
write_timeout = "20s"

# RetryConfig (flattened)
max_retries = 5
base_delay = "200ms"
max_delay = "1m"
backoff_multiplier = 2.0
jitter_percent = 15.0

# ConnectionPoolConfig (flattened)
enabled = true
max_connections_per_host = 200
max_idle_connections = 20
idle_timeout = "5m"
connection_lifetime = "1h"

# HttpHealthCheckConfig (flattened)
health_enabled = true
health_interval = "10s"
health_timeout = "5s"
health_path = "/health"
health_expected_status = 200
```

---

## Summary

### Current Status (Phase 5)

✅ **Completed** (77%):
- Base configs implemented in `config_bases.rs`
- Network configs migrated to use base configs
- TimeoutConfig, RetryConfig, HealthCheckConfig, ConnectionPoolConfig, CacheConfig in use

🟡 **Remaining** (23%):
- Resource config consolidation
- Additional domain config migrations (as needed)
- Expand base configs for new patterns (as they emerge)

---

## Quick Reference

| **Pattern** | **Base Config** | **Use Case** |
|-------------|-----------------|--------------|
| Network timeouts | `TimeoutConfig` | HTTP clients, TCP connections |
| Retry logic | `RetryConfig` | Any retriable operation |
| Health checks | `HealthCheckConfig` | Service monitoring |
| HTTP health | `HttpHealthCheckConfig` | HTTP endpoints |
| Connection pools | `ConnectionPoolConfig` | Database, HTTP clients |
| Caching | `CacheConfig` | Any cache layer |
| Endpoints | `BackendEndpoint` | Service discovery |
| Validation | `ValidationConfig` | Security checks |
| Resources | `BaseResourceConfig` | Container limits |

---

## Related Documents

- `crates/core/common/src/config_bases.rs` - Base config implementations
- `crates/core/config/src/defaults/` - Default constants (module directory)
- `crates/cli/src/network_config/types.rs` - Example usage
- See CHANGELOG.md for configuration unification history

---

**Status**: ✅ Active guide  
**Phase 5 Progress**: 77% complete  
**Last Updated**: November 8, 2025

🍄 **ToadStool Configuration Patterns - Consistency Through Composition**

