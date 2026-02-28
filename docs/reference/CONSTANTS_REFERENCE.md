# 📚 ToadStool Constants Reference Guide
**Last Updated**: November 9, 2025  
**Purpose**: Quick reference for all default configuration constants  
**Module**: `toadstool_config::defaults`

---

## 🎯 Quick Start

```rust
use toadstool_config::defaults;

// Access any constant by category
let api_port = defaults::network::API_PORT;
let timeout = defaults::timeouts::REQUEST_MS;
let max_retries = defaults::retries::MAX_ATTEMPTS;
```

---

## 📋 Table of Contents

1. [Network Constants](#network-constants)
2. [Port Ranges](#port-ranges)
3. [Timeouts](#timeouts)
4. [Retries & Resilience](#retries--resilience)
5. [Storage & Databases](#storage--databases)
6. [Resource Limits](#resource-limits)
7. [Endpoints](#endpoints)
8. [Logging](#logging)
9. [Validation Thresholds](#validation-thresholds) ✨ **NEW!**
10. [Helper Functions](#helper-functions)
11. [Environment Override Pattern](#environment-override-pattern)

---

## Network Constants

**Module**: `defaults::network`

| Constant | Value | Purpose |
|----------|-------|---------|
| `LOCALHOST` | `"127.0.0.1"` | Default localhost address |
| `SONGBIRD_PORT` | `8080` | Service mesh/discovery service |
| `BEARDOG_PORT` | `8081` | Authentication service |
| `NESTGATE_PORT` | `8082` | Orchestration service |
| `SQUIRREL_PORT` | `8083` | MCP service |
| `API_PORT` | `8084` | ToadStool API |
| `METRICS_PORT` | `9090` | Metrics/telemetry endpoint |
| `DISCOVERY_PORT` | `8084` | Service discovery |
| `FEDERATION_PORT` | `7777` | Cross-primal communication |

### Example

```rust
use toadstool_config::defaults::network;

// Build service URLs
let songbird_url = format!("http://{}:{}", network::LOCALHOST, network::SONGBIRD_PORT);
// → "http://127.0.0.1:8080"

let config = ServiceConfig {
    api_port: network::API_PORT,
    metrics_port: network::METRICS_PORT,
    bind_address: network::LOCALHOST.to_string(),
};
```

---

## Port Ranges

**Module**: `defaults::ports`

| Constant | Value | Purpose |
|----------|-------|---------|
| `CONTAINER_START` | `3000` | Container port range start |
| `CONTAINER_END` | `3999` | Container port range end |
| `RANGE_START` | `8080` | General port range start |
| `RANGE_END` | `8999` | General port range end |
| `SIDECAR_LISTEN` | `15001` | Service mesh sidecar listen port |
| `SIDECAR_ADMIN` | `15000` | Service mesh sidecar admin port |

### Example

```rust
use toadstool_config::defaults::ports;

// Allocate ports for containers
for port in ports::CONTAINER_START..=ports::CONTAINER_END {
    if is_port_available(port) {
        allocate_container_port(port);
        break;
    }
}

// Service mesh configuration
let proxy_config = ProxyConfig {
    listen_port: ports::SIDECAR_LISTEN,
    admin_port: ports::SIDECAR_ADMIN,
};
```

---

## Timeouts

**Module**: `defaults::timeouts`

All values are in **milliseconds** unless noted.

| Constant | Value (ms) | Purpose |
|----------|-----------|---------|
| `EXECUTION_MS` | `30,000` | Task execution timeout |
| `HEALTH_CHECK_MS` | `5,000` | Health check interval |
| `CONNECTION_MS` | `5,000` | Connection establishment timeout |
| `REQUEST_MS` | `30,000` | Request/response timeout |
| `IDLE_MS` | `60,000` | Idle connection timeout |
| `DISCOVERY_MS` | `5,000` | Service discovery timeout |
| `DISCOVERY_INTERVAL_MS` | `30,000` | Service discovery refresh interval |
| `KEEPALIVE_SEC` | `60` | Keepalive timeout (in seconds) |

### Example

```rust
use toadstool_config::defaults::timeouts;
use std::time::Duration;

// Create timeout durations
let conn_timeout = Duration::from_millis(timeouts::CONNECTION_MS);
let req_timeout = Duration::from_millis(timeouts::REQUEST_MS);

// Use in HTTP client
let client = reqwest::Client::builder()
    .connect_timeout(conn_timeout)
    .timeout(req_timeout)
    .build()?;

// Or use helper functions (returns Duration directly)
use toadstool_config::defaults::durations;
let exec_timeout = durations::execution();  // No conversion needed!
```

---

## Retries & Resilience

**Module**: `defaults::retries`

| Constant | Value | Purpose |
|----------|-------|---------|
| `MAX_ATTEMPTS` | `3` | Maximum retry attempts |
| `BACKOFF_MS` | `1,000` | Initial backoff duration (ms) |
| `BACKOFF_MULTIPLIER` | `2.0` | Exponential backoff multiplier |
| `MAX_BACKOFF_MS` | `30,000` | Maximum backoff duration (ms) |

### Example: Exponential Backoff

```rust
use toadstool_config::defaults::retries;
use std::time::Duration;

async fn retry_with_backoff<F, T, E>(mut operation: F) -> Result<T, E>
where
    F: FnMut() -> Result<T, E>,
{
    let max_attempts = retries::MAX_ATTEMPTS;
    let initial_backoff = Duration::from_millis(retries::BACKOFF_MS);
    let max_backoff = Duration::from_millis(retries::MAX_BACKOFF_MS);
    let multiplier = retries::BACKOFF_MULTIPLIER;
    
    for attempt in 0..max_attempts {
        match operation() {
            Ok(result) => return Ok(result),
            Err(e) if attempt < max_attempts - 1 => {
                // Calculate exponential backoff
                let backoff = (initial_backoff.as_millis() as f64
                    * multiplier.powi(attempt as i32)) as u64;
                let backoff = backoff.min(max_backoff.as_millis() as u64);
                
                tokio::time::sleep(Duration::from_millis(backoff)).await;
            }
            Err(e) => return Err(e),
        }
    }
    
    unreachable!()
}
```

---

## Storage & Databases

**Module**: `defaults::storage`

| Constant | Value | Purpose |
|----------|-------|---------|
| `DISTRIBUTED_URL` | `"s3://localhost:9000"` | MinIO/S3 compatible storage |
| `MINIO_PORT` | `9000` | MinIO/S3 port |
| `REDIS_PORT` | `6379` | Redis cache port |
| `POSTGRES_PORT` | `5432` | PostgreSQL database port |

### Example

```rust
use toadstool_config::defaults::storage;

// Configure storage backends
let minio_url = format!("localhost:{}", storage::MINIO_PORT);
let redis_url = format!("redis://localhost:{}", storage::REDIS_PORT);
let postgres_url = format!(
    "postgres://user:pass@localhost:{}/toadstool",
    storage::POSTGRES_PORT
);

// Use S3-compatible storage
let s3_client = S3Client::new(storage::DISTRIBUTED_URL);
```

---

## Resource Limits

**Module**: `defaults::resources`

| Constant | Value | Purpose |
|----------|-------|---------|
| `WORKER_THREADS` | `4` | Default worker thread count |
| `MAX_CONNECTIONS` | `1000` | Maximum concurrent connections |
| `RETRY_COUNT` | `3` | Default retry count |
| `SIDECAR_CPU_LIMIT` | `"200m"` | CPU limit (200 millicores) |
| `SIDECAR_MEMORY_LIMIT` | `"256Mi"` | Memory limit (256 mebibytes) |
| `SIDECAR_CPU_REQUEST` | `"100m"` | CPU request (100 millicores) |
| `SIDECAR_MEMORY_REQUEST` | `"128Mi"` | Memory request (128 mebibytes) |

### Example

```rust
use toadstool_config::defaults::resources;

// Configure Tokio runtime
let rt = tokio::runtime::Builder::new_multi_thread()
    .worker_threads(resources::WORKER_THREADS)
    .build()?;

// Configure connection pool
let pool = ConnectionPool::builder()
    .max_connections(resources::MAX_CONNECTIONS)
    .build();

// Kubernetes-style resource specifications
let pod_spec = PodSpec {
    containers: vec![Container {
        resources: Resources {
            limits: ResourceList {
                cpu: resources::SIDECAR_CPU_LIMIT.to_string(),
                memory: resources::SIDECAR_MEMORY_LIMIT.to_string(),
            },
            requests: ResourceList {
                cpu: resources::SIDECAR_CPU_REQUEST.to_string(),
                memory: resources::SIDECAR_MEMORY_REQUEST.to_string(),
            },
        },
    }],
};
```

**Note**: Kubernetes resource format:
- CPU: `"200m"` = 200 millicores = 0.2 CPU cores
- Memory: `"256Mi"` = 256 mebibytes = 268,435,456 bytes

---

## Endpoints

**Module**: `defaults::endpoints`

Helper functions that construct endpoint URLs from network constants.

| Function | Returns | Purpose |
|----------|---------|---------|
| `songbird()` | `String` | Songbird service URL |
| `beardog()` | `String` | BearDog service URL |
| `nestgate()` | `String` | NestGate service URL |
| `api()` | `String` | API endpoint URL |
| `cloud()` | `String` | Cloud endpoint URL |

### Example

```rust
use toadstool_config::defaults::endpoints;

// Get service URLs
let songbird_url = endpoints::songbird();  // "http://localhost:8080"
let beardog_url = endpoints::beardog();    // "http://localhost:8081"

// Use in client configuration
let client = ServiceClient::new(&songbird_url)?;
```

---

## Logging

**Module**: `defaults::logging`

| Constant | Value | Purpose |
|----------|-------|---------|
| `LEVEL` | `"info"` | Default log level |
| `FORMAT` | `"json"` | Default log format |

### Example

```rust
use toadstool_config::defaults::logging;

// Configure logger
tracing_subscriber::fmt()
    .with_max_level(logging::LEVEL)
    .json()  // if logging::FORMAT == "json"
    .init();
```

---

## Validation Thresholds

**Module**: `defaults::validation`

✨ **NEW!** - Added November 9, 2025

These constants define minimum and maximum values for configuration parameters to ensure safe and reasonable operation.

|| Constant | Value | Purpose |
||----------|-------|---------|
|| `MIN_CACHE_SIZE` | `100` | Minimum cache size (entries) |
|| `MAX_CACHE_SIZE` | `100,000` | Maximum cache size (entries) |
|| `MIN_CACHE_TTL_SECS` | `60` | Minimum cache TTL (1 minute) |
|| `MAX_CACHE_TTL_SECS` | `86,400` | Maximum cache TTL (24 hours) |
|| `MIN_FLUSH_INTERVAL_SECS` | `10` | Minimum flush interval |
|| `MAX_FLUSH_INTERVAL_SECS` | `3600` | Maximum flush interval (1 hour) |
|| `MIN_WORKER_THREADS` | `1` | Minimum worker thread count |
|| `MAX_WORKER_THREADS` | `128` | Maximum worker thread count |
|| `MIN_POOL_SIZE` | `1` | Minimum connection pool size |
|| `MAX_POOL_SIZE` | `10,000` | Maximum connection pool size |
|| `MIN_TIMEOUT_MS` | `100` | Minimum timeout (100ms) |
|| `MAX_TIMEOUT_MS` | `3,600,000` | Maximum timeout (1 hour) |
|| `MIN_RETRY_ATTEMPTS` | `0` | Minimum retry attempts |
|| `MAX_RETRY_ATTEMPTS` | `10` | Maximum retry attempts |
|| `MIN_PORT` | `1024` | Minimum port (avoid privileged) |
|| `MAX_PORT` | `65535` | Maximum port number |

### Example

```rust
use toadstool_config::defaults::validation;

// Validate cache configuration
fn validate_cache_size(size: usize) -> Result<(), String> {
    if size < validation::MIN_CACHE_SIZE {
        return Err(format!(
            "Cache size {} below minimum {}",
            size, validation::MIN_CACHE_SIZE
        ));
    }
    if size > validation::MAX_CACHE_SIZE {
        return Err(format!(
            "Cache size {} exceeds maximum {}",
            size, validation::MAX_CACHE_SIZE
        ));
    }
    Ok(())
}

// Validate worker thread count
fn validate_workers(count: usize) -> Result<(), String> {
    if count < validation::MIN_WORKER_THREADS {
        return Err(format!(
            "Worker count {} below minimum {}",
            count, validation::MIN_WORKER_THREADS
        ));
    }
    if count > validation::MAX_WORKER_THREADS {
        return Err(format!(
            "Worker count {} exceeds maximum {}",
            count, validation::MAX_WORKER_THREADS
        ));
    }
    Ok(())
}

// Validate timeout value
fn validate_timeout(timeout_ms: u64) -> Result<(), String> {
    if timeout_ms < validation::MIN_TIMEOUT_MS {
        return Err(format!(
            "Timeout {} ms below minimum {}",
            timeout_ms, validation::MIN_TIMEOUT_MS
        ));
    }
    if timeout_ms > validation::MAX_TIMEOUT_MS {
        return Err(format!(
            "Timeout {} ms exceeds maximum {}",
            timeout_ms, validation::MAX_TIMEOUT_MS
        ));
    }
    Ok(())
}
```

### Benefits

The validation constants provide:
- **Consistent validation** - Same limits applied everywhere
- **Safe operation** - Prevent unreasonable configuration values
- **Clear boundaries** - Explicit min/max values
- **Better errors** - Meaningful validation error messages

---

## Helper Functions

**Module**: `defaults::durations`

Convenience functions that return `Duration` values directly (no conversion needed).

| Function | Returns | Equivalent To |
|----------|---------|---------------|
| `execution()` | `Duration` | `Duration::from_millis(timeouts::EXECUTION_MS)` |
| `health_check()` | `Duration` | `Duration::from_millis(timeouts::HEALTH_CHECK_MS)` |
| `connection()` | `Duration` | `Duration::from_millis(timeouts::CONNECTION_MS)` |
| `request()` | `Duration` | `Duration::from_millis(timeouts::REQUEST_MS)` |
| `idle()` | `Duration` | `Duration::from_millis(timeouts::IDLE_MS)` |
| `discovery()` | `Duration` | `Duration::from_millis(timeouts::DISCOVERY_MS)` |
| `discovery_interval()` | `Duration` | `Duration::from_millis(timeouts::DISCOVERY_INTERVAL_MS)` |
| `keepalive()` | `Duration` | `Duration::from_secs(timeouts::KEEPALIVE_SEC)` |

### Example

```rust
use toadstool_config::defaults::durations;

// Use helper functions for cleaner code
let config = TimeoutConfig {
    connection_timeout: durations::connection(),    // Clean!
    request_timeout: durations::request(),          // No conversion!
    read_timeout: durations::request(),             // Easy to read!
    write_timeout: durations::request(),
};

// vs. the alternative:
use toadstool_config::defaults::timeouts;
use std::time::Duration;
let config = TimeoutConfig {
    connection_timeout: Duration::from_millis(timeouts::CONNECTION_MS),  // Verbose
    request_timeout: Duration::from_millis(timeouts::REQUEST_MS),
    // ...
};
```

---

## Environment Override Pattern

All constants can be overridden via environment variables.

### Manual Override

```rust
use std::env;
use toadstool_config::defaults;

// Override individual constants
let api_port = env::var("TOADSTOOL_API_PORT")
    .ok()
    .and_then(|s| s.parse().ok())
    .unwrap_or(defaults::network::API_PORT);

let max_retries = env::var("TOADSTOOL_MAX_RETRIES")
    .ok()
    .and_then(|s| s.parse().ok())
    .unwrap_or(defaults::retries::MAX_ATTEMPTS);
```

### Automatic Override (Recommended)

```rust
use toadstool_config::env_config::EnvironmentConfig;

// Load all config from environment (with fallback to defaults)
let config = EnvironmentConfig::from_env();

// All values automatically use env vars if set, defaults otherwise
let api_port = config.network.api_port;
let metrics_port = config.network.metrics_port;
let max_retries = config.retries.max_attempts;
```

### Environment Variable Names

| Constant | Environment Variable | Example |
|----------|---------------------|---------|
| `network::API_PORT` | `TOADSTOOL_API_PORT` | `TOADSTOOL_API_PORT=8080` |
| `network::METRICS_PORT` | `TOADSTOOL_METRICS_PORT` | `TOADSTOOL_METRICS_PORT=9090` |
| `timeouts::REQUEST_MS` | `TOADSTOOL_REQUEST_TIMEOUT_MS` | `TOADSTOOL_REQUEST_TIMEOUT_MS=60000` |
| `retries::MAX_ATTEMPTS` | `TOADSTOOL_MAX_RETRIES` | `TOADSTOOL_MAX_RETRIES=5` |
| `resources::WORKER_THREADS` | `TOADSTOOL_WORKER_THREADS` | `TOADSTOOL_WORKER_THREADS=8` |

---

## 📊 Constants Summary

| Category | Count | Module |
|----------|-------|--------|
| Network ports | 9 | `defaults::network` |
| Port ranges | 6 | `defaults::ports` |
| Timeouts | 8 | `defaults::timeouts` |
| Retry settings | 4 | `defaults::retries` |
| Storage backends | 4 | `defaults::storage` |
| Resource limits | 7 | `defaults::resources` |
| Endpoints | 6 | `defaults::endpoints` |
| Logging | 2 | `defaults::logging` |
| **Validation thresholds** ✨ | **16** | `defaults::validation` **NEW!** |
| Helper functions | 8 | `defaults::durations` |
| **Total** | **70** | **10 modules** |

---

## 🎯 Best Practices

### ✅ DO

```rust
// Use defaults with environment override pattern
use toadstool_config::defaults;
use toadstool_config::env_config::EnvironmentConfig;

let config = EnvironmentConfig::from_env();  // Automatic override
let port = config.network.api_port;

// Or use helper functions
use toadstool_config::defaults::durations;
let timeout = durations::request();  // Clean and readable
```

### ❌ DON'T

```rust
// Don't hardcode values directly
let port = 8084;  // Bad: not configurable

// Don't create duplicate constants
pub const MY_API_PORT: u16 = 8084;  // Bad: use defaults::network::API_PORT

// Don't skip environment override mechanism
let port = defaults::network::API_PORT;  // Bad: ignores env vars
// Use EnvironmentConfig::from_env() instead
```

---

## 🔍 Finding Constants

### By Category

1. **Network/Ports**: `defaults::network`, `defaults::ports`
2. **Timeouts**: `defaults::timeouts`, `defaults::durations`
3. **Reliability**: `defaults::retries`
4. **Infrastructure**: `defaults::storage`, `defaults::resources`
5. **Observability**: `defaults::logging`
6. **Validation** ✨: `defaults::validation` **NEW!**

### By Use Case

| Use Case | Relevant Modules |
|----------|------------------|
| HTTP client config | `network`, `timeouts`, `retries` |
| Service mesh setup | `ports`, `resources`, `endpoints` |
| Database connections | `storage`, `timeouts`, `resources` |
| Worker pools | `resources`, `timeouts` |
| Retry logic | `retries`, `timeouts` |
| Service discovery | `network`, `timeouts`, `endpoints` |

---

## 📚 Related Documentation

- **Module Documentation**: See `crates/core/config/src/defaults/` for inline docs
- **Environment Config**: See `crates/core/config/src/env_config.rs` for environment variable system
- **Config Patterns**: See `CONFIG_PATTERNS_GUIDE.md` for configuration best practices
- **Base Configs**: See `crates/core/common/src/config_bases.rs` for reusable config types

---

## 🎊 Quick Reference Card

**Copy-paste ready snippets:**

```rust
// Network
use toadstool_config::defaults::network;
let url = format!("http://{}:{}", network::LOCALHOST, network::API_PORT);

// Timeouts
use toadstool_config::defaults::durations;
let timeout = durations::request();

// Retries
use toadstool_config::defaults::retries;
let max_attempts = retries::MAX_ATTEMPTS;

// Resources
use toadstool_config::defaults::resources;
let workers = resources::WORKER_THREADS;

// Environment Override
use toadstool_config::env_config::EnvironmentConfig;
let config = EnvironmentConfig::from_env();
```

---

*🍄 ToadStool - Universal Compute Platform*  
*Constants Reference - November 8, 2025*  
*For questions or updates, see `/crates/core/config/src/defaults/`*

---

