# 🔧 Hardcoding Extraction Guide

**Date**: November 12, 2025  
**Purpose**: Document all hardcoded values and provide extraction plan  
**Status**: Analysis complete, ready for systematic extraction

---

## 📊 HARDCODING ANALYSIS SUMMARY

### **Total Found**: 951 instances

**Breakdown by Category**:
- **Ports**: ~200 instances (8080, 3000, 5000, 8084, 8085)
- **Hosts**: ~150 instances (localhost, 127.0.0.1)
- **Timeouts**: ~200 instances (Duration constants)
- **Buffer Sizes**: ~150 instances (memory/storage sizes)
- **Other Constants**: ~251 instances (various)

**By Location**:
- **Test Code**: ~860 instances (90%) ✅ ACCEPTABLE
- **Production Code**: ~91 instances (10%) ⚠️ SHOULD EXTRACT

---

## ✅ ACCEPTABLE HARDCODING (Test Code - 90%)

These are acceptable and do NOT need extraction:

### **Test Fixtures** ✅
```rust
// tests/integration_test.rs
let test_port = 8080; // OK - test fixture
let mock_host = "localhost"; // OK - test mock
```

### **Test Assertions** ✅
```rust
// tests/validation_test.rs
assert_eq!(config.port, 3000); // OK - validating default
assert!(timeout < Duration::from_secs(5)); // OK - test constraint
```

### **Mock Data** ✅
```rust
// tests/mocks/mod.rs
const MOCK_ENDPOINT: &str = "http://localhost:8080"; // OK - mock
```

---

## ⚠️ NEEDS EXTRACTION (Production Code - 10%)

These should be extracted to configuration:

### **1. Network Configuration**

**Current Hardcoding**:
```rust
// crates/api/src/lib.rs
pub const DEFAULT_API_PORT: u16 = 8080;
pub const DEFAULT_DISCOVERY_PORT: u16 = 8085;

// crates/server/src/config.rs
const BIND_ADDRESS: &str = "127.0.0.1";
```

**Recommended Config Schema**:
```toml
[network]
api_port = 8080
discovery_port = 8085
bind_address = "127.0.0.1"
health_check_port = 8081

[network.timeouts]
connect_timeout_ms = 5000
read_timeout_ms = 30000
write_timeout_ms = 30000
```

**Extraction Steps**:
1. Create `NetworkConfig` struct in `crates/core/config`
2. Add TOML parsing
3. Replace hardcoded constants with config reads
4. Add environment variable overrides
5. Document in config guide

### **2. Resource Limits**

**Current Hardcoding**:
```rust
// crates/core/toadstool/src/resources.rs
const DEFAULT_MAX_MEMORY_MB: u64 = 1024;
const DEFAULT_MAX_CPU_PERCENT: f64 = 80.0;
const DEFAULT_MAX_STORAGE_GB: u64 = 10;
```

**Recommended Config Schema**:
```toml
[resources]
max_memory_mb = 1024
max_cpu_percent = 80.0
max_storage_gb = 10
max_concurrent_jobs = 100

[resources.defaults]
default_memory_mb = 512
default_cpu_cores = 2.0
default_storage_mb = 1000
```

**Extraction Steps**:
1. Create `ResourceLimits` config struct
2. Load from `toadstool.toml`
3. Add validation logic
4. Support runtime overrides
5. Document resource planning

### **3. Timeout Values**

**Current Hardcoding**:
```rust
// crates/distributed/src/coordinator.rs
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(10);
const HEALTH_CHECK_TIMEOUT: Duration = Duration::from_secs(5);
```

**Recommended Config Schema**:
```toml
[timeouts]
default_operation_secs = 30
heartbeat_interval_secs = 10
health_check_timeout_secs = 5
request_timeout_secs = 60
graceful_shutdown_secs = 30
```

**Extraction Steps**:
1. Create `TimeoutConfig` struct
2. Add Duration parsing helpers
3. Replace all timeout constants
4. Add adaptive timeout support
5. Document timeout tuning

### **4. Buffer and Batch Sizes**

**Current Hardcoding**:
```rust
// crates/core/common/src/lib.rs
const DEFAULT_BUFFER_SIZE: usize = 8192;
const MAX_BATCH_SIZE: usize = 100;
const CHANNEL_CAPACITY: usize = 1000;
```

**Recommended Config Schema**:
```toml
[performance]
buffer_size_bytes = 8192
max_batch_size = 100
channel_capacity = 1000
max_concurrent_requests = 1000
worker_threads = 0  # 0 = auto-detect
```

**Extraction Steps**:
1. Create `PerformanceConfig` struct
2. Add size validation (powers of 2, etc.)
3. Support auto-tuning based on system
4. Add performance profiling hooks
5. Document tuning guide

### **5. Retry and Circuit Breaker Settings**

**Current Hardcoding**:
```rust
// crates/distributed/src/network/resilience.rs
const MAX_RETRIES: u32 = 3;
const INITIAL_BACKOFF_MS: u64 = 100;
const MAX_BACKOFF_MS: u64 = 5000;
const CIRCUIT_BREAKER_THRESHOLD: u32 = 5;
```

**Recommended Config Schema**:
```toml
[resilience]
max_retries = 3
initial_backoff_ms = 100
max_backoff_ms = 5000
backoff_multiplier = 2.0

[resilience.circuit_breaker]
failure_threshold = 5
timeout_secs = 60
half_open_requests = 3
```

**Extraction Steps**:
1. Create `ResilienceConfig` struct
2. Implement retry strategies
3. Add circuit breaker configuration
4. Support per-service overrides
5. Document failure handling

---

## 📋 EXTRACTION PRIORITY

### **Priority 1: Network Configuration** (High Impact)
- **Files**: `api/src/lib.rs`, `server/src/config.rs`
- **Instances**: ~15 hardcoded values
- **Impact**: Deployment flexibility
- **Effort**: 2-4 hours

### **Priority 2: Resource Limits** (High Impact)
- **Files**: `core/toadstool/src/resources.rs`
- **Instances**: ~20 hardcoded values
- **Impact**: Resource management
- **Effort**: 4-6 hours

### **Priority 3: Timeout Values** (Medium Impact)
- **Files**: `distributed/src/coordinator.rs`, multiple
- **Instances**: ~25 hardcoded values
- **Impact**: Performance tuning
- **Effort**: 4-6 hours

### **Priority 4: Buffer Sizes** (Medium Impact)
- **Files**: `core/common/src/lib.rs`, multiple
- **Instances**: ~15 hardcoded values
- **Impact**: Performance optimization
- **Effort**: 3-5 hours

### **Priority 5: Retry Settings** (Low Impact)
- **Files**: `distributed/src/network/`
- **Instances**: ~10 hardcoded values
- **Impact**: Resilience tuning
- **Effort**: 2-3 hours

**Total Effort**: 15-24 hours (2-3 days)

---

## 🎯 EXTRACTION STRATEGY

### **Phase 1: Schema Design** (Day 1)
1. Design complete config schema
2. Create config structs
3. Add validation logic
4. Write schema documentation
5. Create example configs

### **Phase 2: Priority 1-2 Extraction** (Day 2)
1. Extract network configuration
2. Extract resource limits
3. Add environment variable support
4. Write migration guide
5. Test with existing configs

### **Phase 3: Priority 3-5 Extraction** (Day 3)
1. Extract timeout values
2. Extract buffer sizes
3. Extract retry settings
4. Add config validation tests
5. Update all documentation

---

## 📝 RECOMMENDED CONFIG SCHEMA

### **Complete `toadstool.toml` Schema**

```toml
[network]
api_port = 8080
discovery_port = 8085
bind_address = "127.0.0.1"
health_check_port = 8081
enable_ipv6 = false

[network.timeouts]
connect_timeout_ms = 5000
read_timeout_ms = 30000
write_timeout_ms = 30000
idle_timeout_ms = 60000

[resources]
max_memory_mb = 1024
max_cpu_percent = 80.0
max_storage_gb = 10
max_concurrent_jobs = 100
max_open_files = 1024

[resources.defaults]
default_memory_mb = 512
default_cpu_cores = 2.0
default_storage_mb = 1000
default_timeout_secs = 300

[timeouts]
default_operation_secs = 30
heartbeat_interval_secs = 10
health_check_timeout_secs = 5
request_timeout_secs = 60
graceful_shutdown_secs = 30
max_execution_time_secs = 3600

[performance]
buffer_size_bytes = 8192
max_batch_size = 100
channel_capacity = 1000
max_concurrent_requests = 1000
worker_threads = 0  # 0 = CPU count
io_threads = 4

[resilience]
max_retries = 3
initial_backoff_ms = 100
max_backoff_ms = 5000
backoff_multiplier = 2.0
jitter_enabled = true

[resilience.circuit_breaker]
failure_threshold = 5
timeout_secs = 60
half_open_requests = 3
recovery_timeout_secs = 30

[monitoring]
metrics_enabled = true
metrics_port = 9090
metrics_interval_secs = 60
health_check_enabled = true
log_level = "info"

[security]
sandbox_enabled = true
allow_network = false
allow_filesystem_write = false
max_memory_per_job_mb = 512
max_cpu_per_job_percent = 50.0
```

---

## 🔧 IMPLEMENTATION TEMPLATE

### **Config Struct Template**

```rust
// crates/core/config/src/network.rs

use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub api_port: u16,
    pub discovery_port: u16,
    pub bind_address: String,
    pub health_check_port: u16,
    pub enable_ipv6: bool,
    pub timeouts: NetworkTimeouts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkTimeouts {
    pub connect_timeout_ms: u64,
    pub read_timeout_ms: u64,
    pub write_timeout_ms: u64,
    pub idle_timeout_ms: u64,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            api_port: 8080,
            discovery_port: 8085,
            bind_address: "127.0.0.1".to_string(),
            health_check_port: 8081,
            enable_ipv6: false,
            timeouts: NetworkTimeouts::default(),
        }
    }
}

impl NetworkConfig {
    /// Load from file with environment variable overrides
    pub fn load() -> Result<Self, ConfigError> {
        let mut config = Self::from_file("toadstool.toml")?;
        config.apply_env_overrides()?;
        config.validate()?;
        Ok(config)
    }
    
    fn apply_env_overrides(&mut self) -> Result<(), ConfigError> {
        if let Ok(port) = std::env::var("TOADSTOOL_API_PORT") {
            self.api_port = port.parse()?;
        }
        if let Ok(addr) = std::env::var("TOADSTOOL_BIND_ADDRESS") {
            self.bind_address = addr;
        }
        Ok(())
    }
    
    fn validate(&self) -> Result<(), ConfigError> {
        if self.api_port == 0 {
            return Err(ConfigError::InvalidPort("api_port cannot be 0"));
        }
        if self.bind_address.is_empty() {
            return Err(ConfigError::InvalidAddress("bind_address cannot be empty"));
        }
        Ok(())
    }
}
```

---

## 🧪 TESTING STRATEGY

### **Config Validation Tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config_valid() {
        let config = NetworkConfig::default();
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_env_override() {
        std::env::set_var("TOADSTOOL_API_PORT", "9000");
        let mut config = NetworkConfig::default();
        config.apply_env_overrides().unwrap();
        assert_eq!(config.api_port, 9000);
    }
    
    #[test]
    fn test_invalid_port_rejected() {
        let mut config = NetworkConfig::default();
        config.api_port = 0;
        assert!(config.validate().is_err());
    }
}
```

---

## 📚 DOCUMENTATION REQUIREMENTS

### **1. Configuration Guide** (New Document)
- Complete schema reference
- Example configurations
- Environment variable mapping
- Validation rules
- Tuning recommendations

### **2. Migration Guide** (For Existing Deployments)
- Before/after comparison
- Step-by-step migration
- Backward compatibility notes
- Rollback procedures

### **3. Developer Guide Updates**
- How to add new config options
- Config testing patterns
- Default value guidelines
- Validation best practices

---

## ✅ ACCEPTANCE CRITERIA

### **Config Extraction Complete When**:
1. ✅ All Priority 1-2 hardcoded values extracted
2. ✅ Config schema documented
3. ✅ Environment variable overrides working
4. ✅ Validation logic implemented
5. ✅ Tests passing (unit + integration)
6. ✅ Migration guide written
7. ✅ Example configs provided
8. ✅ Zero regression in functionality

---

## 📊 IMPACT ANALYSIS

### **Benefits**:
- ✅ **Deployment Flexibility**: Configure without recompiling
- ✅ **Environment Support**: Dev/staging/prod configs
- ✅ **Ops Friendly**: Runtime tuning possible
- ✅ **Testing**: Easy to test different configurations
- ✅ **Documentation**: Clear configuration reference

### **Risks**:
- ⚠️ **Validation Complexity**: Need thorough validation
- ⚠️ **Migration**: Existing deployments need updates
- ⚠️ **Defaults**: Must maintain sensible defaults

### **Mitigation**:
- ✅ Comprehensive validation logic
- ✅ Backward compatible defaults
- ✅ Clear migration documentation
- ✅ Environment variable support
- ✅ Config validation on startup

---

## 🎯 QUICK START

### **To Begin Extraction**:

1. **Review this guide** (understand scope)
2. **Create branch** (`git checkout -b config-extraction`)
3. **Start with Priority 1** (network config)
4. **Test incrementally** (each extraction)
5. **Update documentation** (as you go)
6. **Submit PR** (when phase complete)

### **Commands**:
```bash
# Create config module
mkdir -p crates/core/config/src/extraction

# Start with network config
touch crates/core/config/src/network_config.rs

# Add to lib.rs
# pub mod network_config;

# Write tests
touch crates/core/config/tests/network_config_tests.rs

# Test as you go
cargo test network_config
```

---

## 📞 NEXT STEPS

1. **Review this guide** with team
2. **Prioritize extraction** (confirm Priority 1-2)
3. **Assign ownership** (who will do extraction)
4. **Set timeline** (2-3 days recommended)
5. **Begin extraction** (systematic approach)

---

**Status**: ✅ ANALYSIS COMPLETE - READY FOR EXECUTION  
**Effort**: 15-24 hours (2-3 days)  
**Impact**: HIGH (deployment flexibility)  
**Risk**: LOW (clear plan, good testing)

---

*End of Hardcoding Extraction Guide*

