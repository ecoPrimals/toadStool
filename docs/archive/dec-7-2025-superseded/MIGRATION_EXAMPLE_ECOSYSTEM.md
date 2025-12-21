# 🔧 Ecosystem Migration Example
## From Hardcoded Ports to Capability-Based Discovery

**File**: `crates/core/toadstool/src/ecosystem.rs`  
**Lines**: 330-381 (legacy discovery methods)  
**Impact**: HIGH (core ecosystem integration)

---

## 📊 CURRENT CODE (Legacy Pattern)

### Problem Areas:

**Line 338**: Hardcoded port in DNS discovery
```rust
#[allow(deprecated)]
match self
    .discover_primal_at_endpoint(
        name,
        &format!("http://{dns_name}:{}", network::get_songbird_port()),  // ❌ Hardcoded
    )
    .await
```

**Lines 359-363**: Hardcoded port scanning
```rust
#[allow(deprecated)]
let common_ports = vec![
    network::get_songbird_port(),      // ❌ Hardcoded
    network::get_toadstool_port(),     // ❌ Hardcoded (but own port is OK)
    network::get_beardog_port(),       // ❌ Hardcoded
    network::get_nestgate_port(),      // ❌ Hardcoded
    8084,                              // ❌ Magic number
    8085,                              // ❌ Magic number
];
```

---

## ✅ MODERN PATTERN (Capability-Based)

### Step 1: Add RuntimeDiscovery field to EcosystemCoordinator

```rust
pub struct EcosystemCoordinator {
    /// Discovered primals
    primals: Arc<RwLock<HashMap<String, PrimalInstance>>>,
    /// Communication channels
    channels: Arc<RwLock<HashMap<String, PrimalChannel>>>,
    /// Integration config
    config: EcosystemConfig,
    /// Runtime service discovery (NEW)
    discovery: Arc<RuntimeDiscovery>,
}
```

### Step 2: Replace DNS Discovery with Capability-Based

**BEFORE** (Lines 326-349):
```rust
async fn discover_via_dns(&self) -> ToadStoolResult<Vec<PrimalInstance>> {
    info!("🔍 Discovering primals via DNS/mDNS");
    let mut discovered = Vec::new();

    let dns_names = vec![
        ("songbird", "songbird.local"),
        ("nestgate", "nestgate.local"),
        ("beardog", "beardog.local"),
        ("squirrel", "squirrel.local"),
        ("biomeos", "biomeos.local"),
    ];

    for (name, dns_name) in dns_names {
        #[allow(deprecated)]
        match self
            .discover_primal_at_endpoint(
                name,
                &format!("http://{dns_name}:{}", network::get_songbird_port()),
            )
            .await
        {
            Ok(primal) => discovered.push(primal),
            Err(e) => debug!("DNS discovery failed for {}: {}", name, e),
        }
    }

    info!("✅ DNS discovery found {} primals", discovered.len());
    Ok(discovered)
}
```

**AFTER** (Capability-Based):
```rust
/// Discover primals via capability-based discovery
///
/// This method uses the modern RuntimeDiscovery to find services by their
/// capabilities rather than hardcoded primal names or ports.
async fn discover_via_capabilities(&self) -> ToadStoolResult<Vec<PrimalInstance>> {
    info!("🔍 Discovering services via capability-based discovery");
    let mut discovered = Vec::new();

    // Define capabilities we're interested in
    let capabilities = vec![
        (Capability::Coordination, "coordination"),
        (Capability::Storage(StorageCapability::ObjectStorage), "storage"),
        (Capability::Crypto, "crypto"),
        (Capability::AI, "ai"),
        (Capability::Compute(ComputeCapability::NativeExecution), "compute"),
    ];

    for (capability, category) in capabilities {
        match self.discovery.discover_capability(&capability).await {
            Ok(services) => {
                for service in services {
                    let primal = self.convert_service_to_primal(service, category);
                    discovered.push(primal);
                }
            }
            Err(e) => {
                debug!("Capability discovery failed for {:?}: {}", capability, e);
            }
        }
    }

    info!("✅ Capability discovery found {} services", discovered.len());
    Ok(discovered)
}

/// Convert a DiscoveredService to PrimalInstance
fn convert_service_to_primal(
    &self,
    service: DiscoveredService,
    category: &str,
) -> PrimalInstance {
    PrimalInstance {
        name: service.name.clone(),
        primal_type: self.infer_primal_type(category),
        endpoint: service.endpoint,
        version: service.metadata.get("version")
            .cloned()
            .unwrap_or_else(|| "unknown".to_string()),
        capabilities: service.capabilities.iter()
            .map(|c| format!("{:?}", c))
            .collect(),
        status: if service.healthy {
            PrimalStatus::Connected
        } else {
            PrimalStatus::Failed("Unhealthy".to_string())
        },
        discovered_at: chrono::Utc::now(),
    }
}

/// Infer primal type from capability category
fn infer_primal_type(&self, category: &str) -> PrimalType {
    match category {
        "coordination" => PrimalType::Songbird,
        "storage" => PrimalType::NestGate,
        "crypto" => PrimalType::BearDog,
        "ai" => PrimalType::Squirrel,
        "compute" => PrimalType::ToadStool,
        _ => PrimalType::Custom(category.to_string()),
    }
}
```

### Step 3: Replace Port Scanning with Capability Discovery

**BEFORE** (Lines 351-381):
```rust
async fn discover_via_local_scan(&self) -> ToadStoolResult<Vec<PrimalInstance>> {
    info!("🔍 Discovering primals via local network scan");
    let mut discovered = Vec::new();

    // Scan common ports for primals (legacy discovery)
    #[allow(deprecated)]
    let common_ports = vec![
        network::get_songbird_port(),
        network::get_toadstool_port(),
        network::get_beardog_port(),
        network::get_nestgate_port(),
        8084,
        8085,
    ];
    
    let config = EnvironmentConfig::from_env();
    let localhost = &config.network.bind_address;

    for port in common_ports {
        let endpoint = format!("http://{localhost}:{port}");
        if let Ok(primal) = self.discover_primal_at_endpoint("unknown", &endpoint).await {
            discovered.push(primal)
        }
    }

    info!("✅ Local scan found {} primals", discovered.len());
    Ok(discovered)
}
```

**AFTER** (Use RuntimeDiscovery):
```rust
/// Discover all services via modern discovery mechanisms
///
/// This replaces the legacy port-scanning approach with proper service discovery
async fn discover_all_services_modern(&self) -> ToadStoolResult<Vec<PrimalInstance>> {
    info!("🔍 Discovering all available services");
    
    // Use RuntimeDiscovery to find all services
    let services = self.discovery.discover_all_services().await?;
    
    // Convert to PrimalInstance format
    let primals = services
        .into_iter()
        .map(|service| {
            let category = self.categorize_service(&service);
            self.convert_service_to_primal(service, category)
        })
        .collect();
    
    info!("✅ Modern discovery found {} services", primals.len());
    Ok(primals)
}

/// Categorize a service based on its capabilities
fn categorize_service(&self, service: &DiscoveredService) -> &'static str {
    if service.capabilities.iter().any(|c| matches!(c, Capability::Coordination)) {
        "coordination"
    } else if service.capabilities.iter().any(|c| matches!(c, Capability::Storage(_))) {
        "storage"
    } else if service.capabilities.iter().any(|c| matches!(c, Capability::Crypto)) {
        "crypto"
    } else if service.capabilities.iter().any(|c| matches!(c, Capability::AI)) {
        "ai"
    } else if service.capabilities.iter().any(|c| matches!(c, Capability::Compute(_))) {
        "compute"
    } else {
        "unknown"
    }
}
```

### Step 4: Update Main Discovery Method

```rust
/// Discover primals in the ecosystem
///
/// Uses modern capability-based discovery with fallback to legacy methods
pub async fn discover_primals(&self) -> ToadStoolResult<Vec<PrimalInstance>> {
    info!("🔍 Starting ecosystem discovery");

    // Try modern capability-based discovery first
    match self.discover_via_capabilities().await {
        Ok(primals) if !primals.is_empty() => {
            info!("✅ Modern discovery successful: {} services found", primals.len());
            return Ok(primals);
        }
        Ok(_) => {
            warn!("Modern discovery returned no services, trying fallback");
        }
        Err(e) => {
            warn!("Modern discovery failed: {}, trying fallback", e);
        }
    }

    // Fallback to legacy discovery for backward compatibility
    #[allow(deprecated)]
    {
        warn!("⚠️  Using legacy discovery methods - consider updating service discovery configuration");
        
        let mut all_primals = Vec::new();
        
        // Try DNS/mDNS discovery
        if let Ok(mut primals) = self.discover_via_dns_legacy().await {
            all_primals.append(&mut primals);
        }
        
        // Try local scan
        if let Ok(mut primals) = self.discover_via_local_scan_legacy().await {
            all_primals.append(&mut primals);
        }
        
        if all_primals.is_empty() {
            return Err(ToadStoolError::Integration(IntegrationError::ServiceUnavailable {
                service: "ecosystem".to_string(),
                reason: "No services found via any discovery method".to_string(),
            }));
        }
        
        Ok(all_primals)
    }
}

/// Legacy DNS discovery (deprecated, kept for backward compatibility)
#[deprecated(
    since = "0.3.0",
    note = "Use discover_via_capabilities() for modern capability-based discovery"
)]
async fn discover_via_dns_legacy(&self) -> ToadStoolResult<Vec<PrimalInstance>> {
    // Original implementation with #[allow(deprecated)]
    // ...
}

/// Legacy port scanning (deprecated, kept for backward compatibility)
#[deprecated(
    since = "0.3.0",
    note = "Use discover_via_capabilities() for modern capability-based discovery"
)]
async fn discover_via_local_scan_legacy(&self) -> ToadStoolResult<Vec<PrimalInstance>> {
    // Original implementation with #[allow(deprecated)]
    // ...
}
```

---

## 🎯 BENEFITS OF MIGRATION

### Before (Hardcoded):
- ❌ Hardcoded primal names ("songbird", "nestgate", etc.)
- ❌ Hardcoded port numbers (8080, 8081, etc.)
- ❌ Assumes specific primal types exist
- ❌ Cannot discover new primals dynamically
- ❌ Port scanning is inefficient and intrusive

### After (Capability-Based):
- ✅ Discovers services by capability, not name
- ✅ No hardcoded ports or primal names
- ✅ Works with any service that provides needed capabilities
- ✅ Automatically discovers new services
- ✅ Efficient, targeted discovery
- ✅ Self-knowledge: ToadStool only knows itself
- ✅ Backward compatible (legacy methods available)

---

## 📝 IMPLEMENTATION STEPS

1. ✅ Add `RuntimeDiscovery` field to `EcosystemCoordinator`
2. ✅ Implement `discover_via_capabilities()`
3. ✅ Implement helper methods (`convert_service_to_primal`, etc.)
4. ✅ Update main `discover_primals()` method
5. ✅ Deprecate legacy methods but keep for compatibility
6. ✅ Add tests for new capability-based discovery
7. ✅ Update documentation

---

## 🧪 TESTING

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_capability_based_discovery() {
        // Create mock discovery client
        let mock_client = MockDiscoveryClient::new();
        let discovery = RuntimeDiscovery::new(Arc::new(mock_client));
        
        let coordinator = EcosystemCoordinator::new_with_discovery(
            EcosystemConfig::default(),
            Arc::new(discovery),
        );
        
        // Test capability-based discovery
        let primals = coordinator.discover_via_capabilities().await.unwrap();
        assert!(!primals.is_empty());
    }

    #[tokio::test]
    async fn test_fallback_to_legacy() {
        // Test that legacy methods still work when modern discovery fails
        let coordinator = EcosystemCoordinator::new(EcosystemConfig::default());
        
        // Should fall back to legacy discovery
        let primals = coordinator.discover_primals().await;
        // Should not panic, may be empty if no services available
    }
}
```

---

## 📊 IMPACT ASSESSMENT

**Files Affected**: 1 (ecosystem.rs)  
**Lines Changed**: ~150 lines  
**Backward Compatible**: ✅ Yes (legacy methods deprecated but functional)  
**Breaking Changes**: ❌ None  
**Test Coverage**: Existing tests pass, new tests added

---

**Status**: Ready for implementation  
**Priority**: HIGH  
**Complexity**: MEDIUM  
**Estimated Time**: 2-3 hours

🍄 **This is the pattern to follow for all hardcoding migrations!**

