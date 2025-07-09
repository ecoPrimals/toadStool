# **toadStool + biomeOS Unification Specification**
**Version:** 1.0  
**Date:** January 2025  
**Author:** ecoPrimals Architecture Team  
**Status:** Implementation Ready  
**Target Team:** toadStool Development Team

---

## **Executive Summary**

This specification outlines the implementation of `toadStool` as the universal runtime engine for the ecoPrimals ecosystem, designed to work seamlessly with `biomeOS` manifest system. toadStool provides Docker-free, sovereignty-focused container orchestration with WASM-first execution.

**Key Principles:**
- **Sovereignty-first:** No external runtime dependencies
- **WASM-native:** WebAssembly as primary execution environment
- **Platform agnostic:** Single binary works on Windows/Linux/macOS
- **Security-focused:** Capability-based isolation with bearDog integration
- **Federation-ready:** Built-in peer-to-peer networking

---

## **1. toadStool Architecture Overview**

### **1.1 Core Components**

```
toadStool Runtime:
├── Core Engine (Rust)
│   ├── Manifest Parser (biome.yaml)
│   ├── Workload Scheduler
│   ├── Resource Manager
│   └── Health Monitor
├── Execution Engines
│   ├── WASM Runtime (Wasmtime)
│   ├── Native Container Runtime
│   └── Process Isolation
├── Security Layer
│   ├── Capability System
│   ├── bearDog Integration
│   └── Cryptographic Verification
└── Federation Layer
    ├── songbird Integration
    ├── Peer Discovery
    └── Service Routing
```

### **1.2 CLI Interface**

```bash
# Single binary installation
curl -sSf https://install.ecoprimals.io | sh

# Usage commands
toadstool run biome.yaml          # Run from manifest
toadstool up biome.yaml -d        # Detached mode
toadstool ps                      # List running biomes
toadstool logs biome-name         # View logs
toadstool stop biome-name         # Stop biome
toadstool federation status       # Federation health
```

---

## **2. Implementation Tasks**

### **Phase 1: Core Runtime Foundation (Weeks 1-2)**

#### **2.1 Main CLI Application (`src/main.rs`)**

```rust
use clap::{App, Arg, SubCommand};
use tokio;

mod manifest;
mod scheduler;
mod runtimes;
mod resources;
mod federation;
mod cli;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = App::new("toadStool")
        .version("1.0.0")
        .about("Universal runtime for ecoPrimals biomes")
        .subcommand(SubCommand::with_name("run")
            .about("Run a biome from manifest")
            .arg(Arg::with_name("manifest")
                .required(true)
                .help("Path to biome.yaml manifest")))
        .subcommand(SubCommand::with_name("up")
            .about("Start biome in background")
            .arg(Arg::with_name("manifest")
                .required(true)
                .help("Path to biome.yaml manifest"))
            .arg(Arg::with_name("detached")
                .short("d")
                .long("detached")
                .help("Run in detached mode")))
        .get_matches();

    match matches.subcommand() {
        ("run", Some(sub_m)) => {
            let manifest_path = sub_m.value_of("manifest").unwrap();
            run_biome(manifest_path).await?;
        }
        ("up", Some(sub_m)) => {
            let manifest_path = sub_m.value_of("manifest").unwrap();
            let detached = sub_m.is_present("detached");
            start_biome(manifest_path, detached).await?;
        }
        _ => println!("Use --help for usage information"),
    }

    Ok(())
}
```

#### **2.2 Manifest Parser (`src/manifest/mod.rs`)**

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize)]
pub struct BiomeManifest {
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    pub kind: String,
    pub metadata: BiomeMetadata,
    pub primals: HashMap<String, PrimalConfig>,
    pub services: Vec<ServiceConfig>,
    pub federation: Option<FederationConfig>,
    pub resources: Option<ResourceLimits>,
    pub health_checks: Option<Vec<HealthCheck>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ServiceConfig {
    pub name: String,
    pub source: String,
    pub runtime: RuntimeType,
    pub resources: ResourceRequirements,
    pub network: Option<Vec<NetworkConfig>>,
    pub volumes: Option<Vec<VolumeMount>>,
    pub environment: Option<Vec<EnvVar>>,
    pub capabilities: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum RuntimeType {
    #[serde(rename = "wasm")]
    Wasm,
    #[serde(rename = "container")]
    Container,
    #[serde(rename = "process")]
    Process,
}

impl BiomeManifest {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ManifestError> {
        let content = fs::read_to_string(path)?;
        let manifest: BiomeManifest = serde_yaml::from_str(&content)?;
        manifest.validate()?;
        Ok(manifest)
    }
    
    pub fn validate(&self) -> Result<(), ManifestError> {
        // Validate API version
        if self.api_version != "biomeOS/v1" {
            return Err(ManifestError::UnsupportedApiVersion(self.api_version.clone()));
        }
        
        // Validate service names are unique
        let mut service_names = std::collections::HashSet::new();
        for service in &self.services {
            if !service_names.insert(&service.name) {
                return Err(ManifestError::DuplicateServiceName(service.name.clone()));
            }
        }
        
        Ok(())
    }
}
```

#### **2.3 Workload Scheduler (`src/scheduler/mod.rs`)**

```rust
use std::collections::HashMap;
use tokio::sync::mpsc;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct WorkloadScheduler {
    runtime_engines: HashMap<RuntimeType, Box<dyn RuntimeEngine>>,
    running_services: Arc<RwLock<HashMap<String, ServiceHandle>>>,
    resource_manager: ResourceManager,
}

impl WorkloadScheduler {
    pub async fn new() -> Result<Self, SchedulerError> {
        let mut engines: HashMap<RuntimeType, Box<dyn RuntimeEngine>> = HashMap::new();
        engines.insert(RuntimeType::Wasm, Box::new(WasmEngine::new().await?));
        engines.insert(RuntimeType::Container, Box::new(ContainerEngine::new().await?));
        engines.insert(RuntimeType::Process, Box::new(ProcessEngine::new().await?));
        
        Ok(Self {
            runtime_engines: engines,
            running_services: Arc::new(RwLock::new(HashMap::new())),
            resource_manager: ResourceManager::new().await?,
        })
    }
    
    pub async fn start_biome(&mut self, manifest: BiomeManifest) -> Result<(), SchedulerError> {
        println!("Validating biome manifest...");
        
        // 1. Validate resource requirements
        self.resource_manager.validate_requirements(&manifest).await?;
        
        // 2. Start primals in dependency order
        println!("Starting primals...");
        self.start_primals(&manifest.primals).await?;
        
        // 3. Start services
        println!("Starting services...");
        for service in &manifest.services {
            self.start_service(service).await?;
        }
        
        println!("Biome '{}' started successfully!", manifest.metadata.name);
        Ok(())
    }
    
    async fn start_primals(&mut self, primals: &HashMap<String, PrimalConfig>) -> Result<(), SchedulerError> {
        // Start primals in dependency order: beardog -> nestgate -> songbird
        let start_order = vec!["beardog", "nestgate", "songbird"];
        
        for primal_name in start_order {
            if let Some(config) = primals.get(primal_name) {
                if config.enabled {
                    println!("Starting primal: {}", primal_name);
                    self.start_primal(primal_name, config).await?;
                }
            }
        }
        
        Ok(())
    }
}
```

### **Phase 2: Runtime Engines (Weeks 3-4)**

#### **2.4 WASM Runtime Engine (`src/runtimes/wasm.rs`)**

```rust
use wasmtime::{Engine, Store, Module, Instance, Linker, Config};
use wasmtime_wasi::{WasiCtxBuilder, WasiCtx};

pub struct WasmEngine {
    engine: Engine,
}

impl WasmEngine {
    pub async fn new() -> Result<Self, RuntimeError> {
        let mut config = Config::new();
        config.wasm_simd(true);
        config.wasm_bulk_memory(true);
        config.wasm_multi_value(true);
        
        let engine = Engine::new(&config)?;
        Ok(Self { engine })
    }
}

#[async_trait::async_trait]
impl RuntimeEngine for WasmEngine {
    async fn start_service(&mut self, config: &ServiceConfig, allocation: ResourceAllocation) -> Result<ServiceHandle, RuntimeError> {
        // 1. Download and verify WASM module
        let wasm_bytes = self.download_module(&config.source).await?;
        let module = Module::new(&self.engine, &wasm_bytes)?;
        
        // 2. Create WASI context with capabilities
        let mut wasi_ctx = WasiCtxBuilder::new()
            .inherit_stdio()
            .inherit_args()?;
        
        // Apply capability restrictions
        wasi_ctx = self.apply_capabilities(wasi_ctx, &config.capabilities)?;
        
        // 3. Create store and instance
        let mut store = Store::new(&self.engine, wasi_ctx.build());
        let mut linker = Linker::new(&self.engine);
        wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;
        
        // Add custom ecoPrimals functions
        self.add_ecoprimals_functions(&mut linker)?;
        
        let instance = linker.instantiate(&mut store, &module)?;
        
        // 4. Start the WASM module
        let service_name = config.name.clone();
        let handle = self.spawn_service_task(instance, store, service_name).await?;
        
        Ok(handle)
    }
    
    fn apply_capabilities(&self, mut ctx: WasiCtxBuilder, capabilities: &Option<Vec<String>>) -> Result<WasiCtxBuilder, RuntimeError> {
        if let Some(caps) = capabilities {
            for cap in caps {
                match cap.as_str() {
                    "network.client" => {
                        ctx = ctx.inherit_network();
                    }
                    cap if cap.starts_with("fs.read:") => {
                        let path = &cap[8..];
                        ctx = ctx.preopened_dir(path, path)?;
                    }
                    cap if cap.starts_with("fs.write:") => {
                        let path = &cap[9..];
                        ctx = ctx.preopened_dir(path, path)?;
                    }
                    _ => return Err(RuntimeError::UnsupportedCapability(cap.clone())),
                }
            }
        }
        Ok(ctx)
    }
    
    fn add_ecoprimals_functions(&self, linker: &mut Linker<WasiCtx>) -> Result<(), RuntimeError> {
        // bearDog crypto functions
        linker.func_wrap("ecoprimals", "crypto_sign", |data: i32, len: i32| -> i32 {
            // TODO: Implement crypto signing via bearDog
            0
        })?;
        
        // nestgate storage functions
        linker.func_wrap("ecoprimals", "storage_get", |key: i32, key_len: i32| -> i32 {
            // TODO: Implement storage access via nestgate
            0
        })?;
        
        Ok(())
    }
}
```

#### **2.5 Resource Manager (`src/resources/mod.rs`)**

```rust
use sysinfo::{System, SystemExt, ProcessorExt};
use std::collections::HashMap;

pub struct ResourceManager {
    system: System,
    allocated_resources: HashMap<String, ResourceAllocation>,
}

impl ResourceManager {
    pub async fn new() -> Result<Self, ResourceError> {
        let mut system = System::new_all();
        system.refresh_all();
        
        Ok(Self {
            system,
            allocated_resources: HashMap::new(),
        })
    }
    
    pub async fn validate_requirements(&self, manifest: &BiomeManifest) -> Result<(), ResourceError> {
        let total_cpu: f64 = manifest.services.iter()
            .map(|s| s.resources.cpu.parse::<f64>().unwrap_or(0.0))
            .sum();
        
        let total_memory: u64 = manifest.services.iter()
            .map(|s| self.parse_memory(&s.resources.memory))
            .sum();
        
        // Check against system limits
        let available_cpu = self.system.processors().len() as f64;
        if total_cpu > available_cpu {
            return Err(ResourceError::InsufficientCPU {
                required: total_cpu,
                available: available_cpu,
            });
        }
        
        let available_memory = self.system.total_memory() * 1024;
        if total_memory > available_memory {
            return Err(ResourceError::InsufficientMemory {
                required: total_memory,
                available: available_memory,
            });
        }
        
        Ok(())
    }
    
    fn parse_memory(&self, memory_str: &str) -> u64 {
        let memory_str = memory_str.to_uppercase();
        
        if memory_str.ends_with("GB") {
            let num = memory_str[..memory_str.len()-2].parse::<u64>().unwrap_or(0);
            num * 1024 * 1024 * 1024
        } else if memory_str.ends_with("MB") {
            let num = memory_str[..memory_str.len()-2].parse::<u64>().unwrap_or(0);
            num * 1024 * 1024
        } else {
            memory_str.parse::<u64>().unwrap_or(0)
        }
    }
}
```

### **Phase 3: Integration & Security (Weeks 5-6)**

#### **2.6 Security Layer (`src/security/mod.rs`)**

```rust
pub struct SecurityManager {
    capability_validator: CapabilityValidator,
    crypto_verifier: CryptoVerifier,
    sandbox_enforcer: SandboxEnforcer,
}

impl SecurityManager {
    pub fn new() -> Self {
        Self {
            capability_validator: CapabilityValidator::new(),
            crypto_verifier: CryptoVerifier::new(),
            sandbox_enforcer: SandboxEnforcer::new(),
        }
    }
    
    pub fn validate_service_capabilities(&self, service: &ServiceConfig) -> Result<(), SecurityError> {
        if let Some(capabilities) = &service.capabilities {
            for cap in capabilities {
                self.capability_validator.validate_capability(cap)?;
            }
        }
        Ok(())
    }
    
    pub fn verify_module_signature(&self, module_bytes: &[u8], signature: &str) -> Result<(), SecurityError> {
        self.crypto_verifier.verify_signature(module_bytes, signature)
    }
    
    pub fn enforce_sandbox(&self, service: &ServiceConfig) -> Result<SandboxConfig, SecurityError> {
        self.sandbox_enforcer.create_sandbox(service)
    }
}
```

#### **2.7 Federation Layer (`src/federation/mod.rs`)**

```rust
pub struct FederationManager {
    songbird_client: SongbirdClient,
    peer_discovery: PeerDiscovery,
    service_router: ServiceRouter,
}

impl FederationManager {
    pub async fn new() -> Result<Self, FederationError> {
        Ok(Self {
            songbird_client: SongbirdClient::new().await?,
            peer_discovery: PeerDiscovery::new(),
            service_router: ServiceRouter::new(),
        })
    }
    
    pub async fn configure_federation(&self, config: &FederationConfig) -> Result<(), FederationError> {
        if config.enabled {
            self.songbird_client.configure_trust_policy(&config.trust_policy).await?;
            
            for peer in &config.allowed_peers {
                self.peer_discovery.add_allowed_peer(peer).await?;
            }
            
            for service in &config.shared_services {
                self.service_router.register_shared_service(service).await?;
            }
        }
        
        Ok(())
    }
}
```

---

## **3. Dependencies and Cargo.toml**

```toml
[package]
name = "toadstool"
version = "1.0.0"
edition = "2021"

[dependencies]
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_yaml = "0.9"
clap = "2.33"
thiserror = "1.0"
wasmtime = "18.0"
wasmtime-wasi = "18.0"
async-trait = "0.1"
uuid = { version = "1.0", features = ["v4"] }
sysinfo = "0.30"
reqwest = { version = "0.11", features = ["json"] }
sha2 = "0.10"
hex = "0.4"
tracing = "0.1"
tracing-subscriber = "0.3"

[dev-dependencies]
tempfile = "3.0"
tokio-test = "0.4"
```

---

## **4. Implementation Timeline**

### **Week 1-2: Foundation**
- [ ] Main CLI application structure
- [ ] Manifest parser with validation
- [ ] Basic workload scheduler
- [ ] Resource manager implementation
- [ ] Error handling framework

### **Week 3-4: Runtime Engines**
- [ ] WASM runtime engine with Wasmtime
- [ ] Container runtime engine (fallback)
- [ ] Process runtime engine (legacy)
- [ ] Capability system implementation
- [ ] Service lifecycle management

### **Week 5-6: Advanced Features**
- [ ] Security layer with bearDog integration
- [ ] Federation layer with songBird
- [ ] Health monitoring system
- [ ] Custom WASM functions
- [ ] Performance optimization

### **Week 7-8: Production Ready**
- [ ] Comprehensive error handling
- [ ] Logging and telemetry
- [ ] Testing suite
- [ ] Documentation
- [ ] Cross-platform installer

---

## **5. Success Criteria**

### **Technical Goals**
- [ ] Single binary deployment with no external dependencies
- [ ] WASM-first execution with container fallback
- [ ] Capability-based security system
- [ ] Resource management with system limits
- [ ] Federation integration with songBird

### **Performance Goals**
- [ ] Sub-second startup time for WASM services
- [ ] Memory usage under 100MB base footprint
- [ ] CPU overhead under 5% for runtime
- [ ] Network latency under 10ms for local services

### **Security Goals**
- [ ] No privileged containers
- [ ] Cryptographic verification of all modules
- [ ] Sandbox isolation for all services
- [ ] Audit logging for all operations

---

This specification provides the complete technical foundation for implementing toadStool as the universal runtime for the ecoPrimals ecosystem. The implementation follows a phased approach with clear milestones and success criteria.

**Next Steps:** Begin with Phase 1 implementation focusing on CLI structure and manifest parsing. 