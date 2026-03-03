# 🔐 Quick Start: Encryption in Toadstool

## 5-Minute Guide to Encrypted Workloads

### Basic Encryption

```rust
use toadstool::{ExecutionRequest, EncryptionConfig, SecurityLevel};

// Create a request with encryption
let request = ExecutionRequest {
    encryption_config: Some(EncryptionConfig {
        required: true,                          // Fail if no crypto provider
        encrypt_results: true,                   // Encrypt output too
        min_security_level: SecurityLevel::Enhanced,
        ..Default::default()
    }),
    ..Default::default()
};

// Execute - encryption handled automatically
let response = orchestrator.execute(request).await?;
```

### Discovering Crypto Providers

```rust
use toadstool::{CryptoProviderRegistry, CryptoCapability, SecurityLevel};

// Create registry
let registry = CryptoProviderRegistry::new();

// Register BearDog client (when available)
let beardog = BearDogClient::new(BearDogConfig::default());
registry.register(Arc::new(beardog)).await?;

// Find provider for capability
let capability = CryptoCapability {
    algorithms: vec!["chacha20poly1305".to_string()],
    security_level: SecurityLevel::Enhanced,
    hardware_backed: false,
};

let provider = registry.find_provider(&capability).await?;
```

### Encryption Context

```rust
use toadstool::{EncryptionContext, EncryptionContextBuilder};

// Build context with fluent API
let mut context = EncryptionContextBuilder::new(execution_id)
    .required(true)
    .encrypt_results(true)
    .security_level(SecurityLevel::Enhanced)
    .algorithms(vec!["chacha20poly1305".to_string()])
    .build();

// Discover provider
context.discover_provider(&registry).await?;

// Decrypt input
let decrypted = context.decrypt_input(&encrypted_input).await?;

// Encrypt output
let encrypted_output = context.encrypt_output(&result_data).await?;
```

### BearDog Integration (When Available)

```rust
use toadstool_distributed::beardog_integration::{BearDogClient, BearDogConfig};

// Create BearDog client with auto-discovery
let beardog = BearDogClient::new(BearDogConfig {
    auto_discover: true,
    discovery_timeout_ms: 5000,
    preferred_location: ServiceLocation::Local,
    fallback_enabled: true,
});

// Discover BearDog services
let endpoints = beardog.discover().await?;
println!("Found {} BearDog endpoints", endpoints.len());

// Register as crypto provider
let registry = CryptoProviderRegistry::new();
registry.register(Arc::new(beardog)).await?;

// Now all encrypted executions can use BearDog
```

### Graceful Fallback

```rust
use toadstool::{ExecutionRequest, EncryptionConfig};

// Optional encryption (fallback to unencrypted)
let request = ExecutionRequest {
    encryption_config: Some(EncryptionConfig {
        required: false,  // Don't fail if unavailable
        encrypt_results: true,
        ..Default::default()
    }),
    ..Default::default()
};

// Executes with encryption if available, without if not
let response = orchestrator.execute(request).await?;
```

### Key Management

```rust
use toadstool::{EncryptionKey, KeyRotationPolicy};

// Create rotation policy
let policy = KeyRotationPolicy {
    max_uses: Some(100_000),
    max_age_seconds: Some(86400 * 30),  // 30 days
    max_data_bytes: Some(100 * 1024 * 1024 * 1024),  // 100 GB
    auto_retire: true,
};

// Check if key should rotate
if policy.should_rotate(key_uses, key_age, data_encrypted) {
    let new_key = provider.generate_key(SecurityLevel::Enhanced).await?;
    // Update active key
}
```

### Security Levels

```rust
use toadstool::SecurityLevel;

// Standard - Software-based encryption
SecurityLevel::Standard

// Enhanced - Genetic keys, entropy mixing (BearDog)
SecurityLevel::Enhanced

// Hardware-Secured - HSM required
SecurityLevel::HardwareSecured
```

### Error Handling

```rust
use toadstool::{ToadStoolError, ExecutionRequest};

match orchestrator.execute(request).await {
    Ok(response) => {
        println!("Execution successful!");
    }
    Err(ToadStoolError::Configuration(msg)) => {
        eprintln!("Encryption config error: {}", msg);
        // Maybe retry without encryption
    }
    Err(ToadStoolError::Network(msg)) => {
        eprintln!("BearDog unreachable: {}", msg);
        // Use fallback crypto provider
    }
    Err(e) => {
        eprintln!("Execution failed: {}", e);
    }
}
```

### Complete Example

```rust
use toadstool::{
    ExecutionRequest, EncryptionConfig, SecurityLevel,
    CryptoProviderRegistry, WorkloadSpec, ExecutableSource,
};
use toadstool_distributed::beardog_integration::BearDogClient;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Setup crypto provider registry
    let registry = Arc::new(CryptoProviderRegistry::new());
    
    // 2. Register BearDog (when available)
    let beardog = BearDogClient::new(Default::default());
    registry.register(Arc::new(beardog)).await?;
    
    // 3. Create workload
    let workload = WorkloadSpec::Native {
        executable: ExecutableSource::File {
            path: "/usr/bin/my-sensitive-app".into(),
        },
        args: Some(vec!["--process-data".to_string()]),
        working_dir: None,
        env_vars: Default::default(),
        user: None,
    };
    
    // 4. Create encrypted execution request
    let request = ExecutionRequest {
        workload,
        encryption_config: Some(EncryptionConfig {
            required: true,
            encrypt_results: true,
            min_security_level: SecurityLevel::Enhanced,
            preferred_algorithms: vec![
                "chacha20poly1305".to_string(),
                "aes-256-gcm".to_string(),
            ],
            ..Default::default()
        }),
        ..Default::default()
    };
    
    // 5. Execute with automatic encryption
    let response = orchestrator.execute(request).await?;
    
    println!("Encrypted execution completed!");
    println!("Duration: {:?}", response.duration);
    println!("Status: {:?}", response.status);
    
    Ok(())
}
```

## More Information

- **Architecture**: `docs/planning/THREE_PRIMAL_INTEGRATION_ROADMAP.md`
- **API Docs**: `crates/core/toadstool/src/encryption/mod.rs`

## 🚀 Status

- ✅ Encryption API: **Production Ready**
- 🟡 BearDog Integration: **Awaiting HTTP API**
- ✅ Fallback Support: **Fully Implemented**
- ✅ Documentation: **Complete**

**Start using encryption in Toadstool today with graceful fallback!**

