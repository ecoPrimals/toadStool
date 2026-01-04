# 🍄 ToadStool Daemon Mode - Evolution Plan

**Date**: January 4, 2026  
**Status**: Architecture Proposal → Ready for Implementation  
**Grade Impact**: A+ (97/100) → A+ (98/100) (+1 point for ecosystem completion)  
**Estimated Effort**: 20-26 hours (can parallelize)

---

## 🎯 Executive Summary

**Proposal**: Add daemon mode to ToadStool for ecosystem-wide workload execution.

**Current State**: CLI-only (project-specific execution)  
**Proposed State**: Dual-mode (CLI + Daemon)

**Alignment**: ✅ Perfect fit with infant discovery architecture  
**Philosophy**: ✅ "Like the fungus, adapt form to environment"

---

## 🌟 Architecture Vision

### Dual-Mode Operation

```
┌─────────────────────────────────────────────────────────────┐
│                      ToadStool Binary                       │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  CLI Mode                          Daemon Mode              │
│  ┌─────────────────┐              ┌──────────────────┐     │
│  │ Direct Invoke   │              │ HTTP API Server  │     │
│  │ biome.yaml      │              │ /api/v1/workload │     │
│  │ Exit on complete│              │ Long-running     │     │
│  └─────────────────┘              └──────────────────┘     │
│         │                                   │               │
│         └──────────┬────────────────────────┘               │
│                    │                                        │
│              ┌─────▼──────┐                                 │
│              │  Workload  │                                 │
│              │  Executor  │                                 │
│              │  (Shared)  │                                 │
│              └────────────┘                                 │
└─────────────────────────────────────────────────────────────┘
```

### Infant Discovery Integration

```
ToadStool Daemon Startup:
  1. Load self-knowledge (ports, resources)
  2. Connect to biomeOS registry
  3. Register capabilities:
     - Compute (wasm, container, python, native, gpu)
     - Storage (local, distributed, encrypted)
     - Orchestration (multi-service coordination)
  4. Discover required services:
     - Security (BearDog) for workload auth
     - Discovery (Songbird) for service location
  5. Start API server
  6. Begin heartbeat reporting
```

**Perfect alignment with infant discovery!** ✅

---

## 🚀 Implementation Phases

### Phase 1: Dual-Mode CLI (2-3 hours)

**Goal**: Add daemon subcommand to CLI

**Changes**:
```rust
// crates/cli/src/main.rs

#[derive(Parser)]
enum ToadStoolCommand {
    /// Run a biome.yaml file (traditional CLI mode)
    Run {
        #[arg(value_name = "BIOME_FILE")]
        biome_file: PathBuf,
        
        #[arg(long)]
        watch: bool,
    },
    
    /// Start ToadStool as a daemon service (NEW!)
    Daemon {
        /// Register with biomeOS capability registry
        #[arg(long)]
        register: bool,
        
        /// HTTP API port (default: 8084)
        #[arg(long, default_value = "8084")]
        port: u16,
        
        /// Unix socket path for IPC
        #[arg(long)]
        socket: Option<PathBuf>,
        
        /// Config file
        #[arg(long)]
        config: Option<PathBuf>,
    },
    
    // ... existing subcommands
}
```

**Deliverable**: `toadstool daemon --register` command works ✅

---

### Phase 2: Daemon Server Core (4-6 hours)

**Goal**: HTTP API server for workload submission

**New Crate**: `crates/daemon/` (or extend `crates/server/`)

```rust
// crates/daemon/src/server.rs

pub struct DaemonServer {
    /// HTTP server for API
    http_server: Arc<HttpServer>,
    
    /// Workload manager
    workload_manager: Arc<WorkloadManager>,
    
    /// biomeOS registry client
    biomeos_client: Option<Arc<BiomeOSClient>>,
    
    /// Resource monitor
    resource_monitor: Arc<ResourceMonitor>,
    
    /// Configuration
    config: DaemonConfig,
}

impl DaemonServer {
    pub async fn start(config: DaemonConfig) -> Result<Self> {
        info!("🍄 Starting ToadStool daemon mode...");
        
        // 1. Connect to biomeOS (infant discovery!)
        let biomeos_client = if config.register_with_biomeos {
            match BiomeOSClient::connect().await {
                Ok(client) => {
                    info!("✅ Connected to biomeOS registry");
                    
                    // Register our capabilities
                    client.register_primal(PrimalRegistration {
                        id: PrimalId::generate(),
                        name: "toadstool".to_string(),
                        capabilities: vec![
                            Capability::Compute {
                                types: vec!["wasm", "container", "python", "native", "gpu"],
                            },
                            Capability::Storage {
                                types: vec!["local", "distributed", "encrypted"],
                            },
                            Capability::Orchestration,
                        ],
                        endpoints: vec![
                            Endpoint::Http(format!("http://localhost:{}", config.port)),
                            Endpoint::Unix(config.socket_path.clone()),
                        ],
                        resources: Self::report_resources().await?,
                    }).await?;
                    
                    info!("✅ Registered with biomeOS capability registry");
                    Some(Arc::new(client))
                }
                Err(e) => {
                    warn!("⚠️  Failed to connect to biomeOS: {e}");
                    warn!("📍 Running in standalone mode");
                    None
                }
            }
        } else {
            None
        };
        
        // 2. Start HTTP API server
        let http_server = HttpServer::new(config.port, Self::routes());
        
        // 3. Start workload manager
        let workload_manager = WorkloadManager::new(WorkloadConfig {
            max_concurrent: config.max_concurrent_workloads,
            default_timeout: config.default_workload_timeout,
        });
        
        // 4. Start resource monitor
        let resource_monitor = ResourceMonitor::start(Duration::from_secs(30));
        
        // 5. Start heartbeat task (if registered with biomeOS)
        if let Some(ref client) = biomeos_client {
            tokio::spawn(Self::heartbeat_loop(
                client.clone(),
                resource_monitor.clone(),
                workload_manager.clone(),
            ));
        }
        
        Ok(Self {
            http_server: Arc::new(http_server),
            workload_manager: Arc::new(workload_manager),
            biomeos_client,
            resource_monitor: Arc::new(resource_monitor),
            config,
        })
    }
    
    async fn report_resources() -> Result<ResourceReport> {
        Ok(ResourceReport {
            cpu: CpuInfo {
                total_cores: num_cpus::get(),
                available_cores: num_cpus::get(),
            },
            memory: MemoryInfo {
                total_bytes: sys_info::mem_info()?.total * 1024,
                available_bytes: sys_info::mem_info()?.avail * 1024,
            },
            gpu: GpuInfo::detect().await?,
            storage: StorageInfo::detect().await?,
        })
    }
    
    fn routes() -> Router {
        Router::new()
            // Workload management
            .route("/api/v1/workload/submit", post(submit_workload))
            .route("/api/v1/workload/:id", get(get_workload_status))
            .route("/api/v1/workload/:id", delete(stop_workload))
            .route("/api/v1/workloads", get(list_workloads))
            
            // Health & status
            .route("/health", get(health_check))
            .route("/api/v1/status", get(daemon_status))
            .route("/api/v1/resources", get(resource_status))
            
            // Metrics
            .route("/metrics", get(prometheus_metrics))
    }
    
    async fn heartbeat_loop(
        client: Arc<BiomeOSClient>,
        resource_monitor: Arc<ResourceMonitor>,
        workload_manager: Arc<WorkloadManager>,
    ) {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        
        loop {
            interval.tick().await;
            
            let resources = resource_monitor.current_snapshot().await;
            let workloads = workload_manager.get_summary().await;
            
            if let Err(e) = client.send_heartbeat(Heartbeat {
                timestamp: Utc::now(),
                resources,
                workloads,
                health: HealthStatus::Healthy,
            }).await {
                error!("Failed to send heartbeat: {e}");
            }
        }
    }
    
    pub async fn run(self) -> Result<()> {
        info!("🚀 ToadStool daemon running on port {}", self.config.port);
        info!("📊 API: http://localhost:{}/api/v1/workloads", self.config.port);
        
        tokio::select! {
            result = self.http_server.serve() => {
                error!("HTTP server stopped: {result:?}");
            }
            _ = signal::ctrl_c() => {
                info!("🛑 Shutdown signal received");
            }
        }
        
        // Graceful shutdown
        info!("🧹 Cleaning up workloads...");
        self.workload_manager.stop_all(Duration::from_secs(30)).await?;
        
        if let Some(ref client) = self.biomeos_client {
            info!("📤 Unregistering from biomeOS...");
            client.unregister().await?;
        }
        
        info!("👋 ToadStool daemon stopped");
        Ok(())
    }
}
```

**API Endpoints**:
```
POST   /api/v1/workload/submit    - Submit workload
GET    /api/v1/workload/:id       - Query status
DELETE /api/v1/workload/:id       - Stop workload
GET    /api/v1/workloads          - List all workloads
GET    /health                    - Health check
GET    /api/v1/status             - Daemon status
GET    /api/v1/resources          - Resource availability
GET    /metrics                   - Prometheus metrics
```

---

### Phase 3: Workload Manager (4-6 hours)

**Goal**: Queue, execute, and monitor workloads

```rust
// crates/daemon/src/workload_manager.rs

pub struct WorkloadManager {
    /// Active workloads
    active: Arc<RwLock<HashMap<WorkloadId, ActiveWorkload>>>,
    
    /// Queued workloads
    queue: Arc<RwLock<VecDeque<QueuedWorkload>>>,
    
    /// Executor (reuse existing BiomeExecutor!)
    executor: Arc<BiomeExecutor>,
    
    /// Configuration
    config: WorkloadConfig,
}

impl WorkloadManager {
    pub async fn submit(
        &self,
        request: WorkloadRequest,
    ) -> Result<WorkloadId> {
        let workload_id = WorkloadId::generate();
        
        info!("📥 Received workload: {workload_id} from {}", request.requester);
        
        // Validate request
        self.validate_request(&request).await?;
        
        // Check capacity
        if self.has_capacity().await {
            // Execute immediately
            self.execute_workload(workload_id, request).await?;
        } else {
            // Queue for later
            self.queue_workload(workload_id, request).await?;
        }
        
        Ok(workload_id)
    }
    
    async fn execute_workload(
        &self,
        id: WorkloadId,
        request: WorkloadRequest,
    ) -> Result<()> {
        info!("🚀 Executing workload: {id}");
        
        let workload = ActiveWorkload {
            id,
            request: request.clone(),
            status: WorkloadStatus::Running,
            started_at: Utc::now(),
            progress: 0.0,
        };
        
        self.active.write().await.insert(id, workload);
        
        // Execute using existing BiomeExecutor
        let executor = self.executor.clone();
        let active = self.active.clone();
        
        tokio::spawn(async move {
            match executor.execute_biome(&request.biome_yaml).await {
                Ok(result) => {
                    info!("✅ Workload {id} completed successfully");
                    
                    if let Some(mut w) = active.write().await.get_mut(&id) {
                        w.status = WorkloadStatus::Completed;
                        w.result = Some(result);
                        w.completed_at = Some(Utc::now());
                    }
                }
                Err(e) => {
                    error!("❌ Workload {id} failed: {e}");
                    
                    if let Some(mut w) = active.write().await.get_mut(&id) {
                        w.status = WorkloadStatus::Failed;
                        w.error = Some(e.to_string());
                        w.completed_at = Some(Utc::now());
                    }
                }
            }
        });
        
        Ok(())
    }
    
    pub async fn get_status(&self, id: &WorkloadId) -> Result<WorkloadStatus> {
        self.active
            .read()
            .await
            .get(id)
            .map(|w| w.status.clone())
            .ok_or_else(|| ToadStoolError::runtime("Workload not found"))
    }
    
    pub async fn stop(&self, id: &WorkloadId) -> Result<()> {
        info!("🛑 Stopping workload: {id}");
        
        // Implementation: Send stop signal to workload
        // Reuse existing BiomeExecutor stop mechanisms
        
        Ok(())
    }
}
```

---

### Phase 4: Resource Monitor (2-3 hours)

**Goal**: Track system resources and report to biomeOS

```rust
// crates/daemon/src/resource_monitor.rs

pub struct ResourceMonitor {
    /// Current resource snapshot
    current: Arc<RwLock<ResourceSnapshot>>,
    
    /// Monitoring interval
    interval: Duration,
}

impl ResourceMonitor {
    pub fn start(interval: Duration) -> Self {
        let monitor = Self {
            current: Arc::new(RwLock::new(ResourceSnapshot::default())),
            interval,
        };
        
        let current = monitor.current.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval);
            
            loop {
                interval.tick().await;
                
                let snapshot = Self::capture_snapshot().await;
                *current.write().await = snapshot;
            }
        });
        
        monitor
    }
    
    async fn capture_snapshot() -> ResourceSnapshot {
        ResourceSnapshot {
            timestamp: Utc::now(),
            cpu: CpuSnapshot {
                total_cores: num_cpus::get(),
                usage_percent: Self::get_cpu_usage(),
            },
            memory: MemorySnapshot {
                total_bytes: sys_info::mem_info().unwrap().total * 1024,
                used_bytes: sys_info::mem_info().unwrap().total * 1024 
                    - sys_info::mem_info().unwrap().avail * 1024,
            },
            gpu: GpuSnapshot::capture().await,
            storage: StorageSnapshot::capture().await,
        }
    }
    
    pub async fn current_snapshot(&self) -> ResourceSnapshot {
        self.current.read().await.clone()
    }
}
```

---

### Phase 5: API Handlers (3-4 hours)

**Goal**: Implement HTTP API endpoints

```rust
// crates/daemon/src/handlers.rs

async fn submit_workload(
    State(manager): State<Arc<WorkloadManager>>,
    Json(request): Json<WorkloadRequest>,
) -> Result<Json<WorkloadResponse>, ApiError> {
    let workload_id = manager.submit(request).await?;
    
    Ok(Json(WorkloadResponse {
        workload_id,
        status: WorkloadStatus::Queued,
        message: "Workload submitted successfully".to_string(),
    }))
}

async fn get_workload_status(
    State(manager): State<Arc<WorkloadManager>>,
    Path(id): Path<WorkloadId>,
) -> Result<Json<WorkloadStatusResponse>, ApiError> {
    let status = manager.get_status(&id).await?;
    
    Ok(Json(WorkloadStatusResponse {
        id,
        status,
        // ... other fields
    }))
}

async fn list_workloads(
    State(manager): State<Arc<WorkloadManager>>,
) -> Result<Json<WorkloadListResponse>, ApiError> {
    let workloads = manager.list_all().await?;
    
    Ok(Json(WorkloadListResponse {
        active: workloads.active,
        queued: workloads.queued,
        completed: workloads.completed,
        total: workloads.total,
    }))
}

async fn daemon_status(
    State(server): State<Arc<DaemonServer>>,
) -> Result<Json<DaemonStatusResponse>, ApiError> {
    Ok(Json(DaemonStatusResponse {
        uptime: server.uptime(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        registered_with_biomeos: server.biomeos_client.is_some(),
        workloads: server.workload_manager.get_summary().await,
        resources: server.resource_monitor.current_snapshot().await,
    }))
}
```

---

### Phase 6: Integration Tests (2-3 hours)

**Goal**: Test daemon mode end-to-end

```rust
// crates/daemon/tests/daemon_integration_tests.rs

#[tokio::test]
async fn test_daemon_startup_and_registration() {
    let config = DaemonConfig {
        port: 8084,
        register_with_biomeos: true,
        // ...
    };
    
    let daemon = DaemonServer::start(config).await.unwrap();
    
    // Verify registered with biomeOS
    assert!(daemon.biomeos_client.is_some());
    
    // Verify API server is running
    let response = reqwest::get("http://localhost:8084/health")
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_workload_submission() {
    // Start daemon
    let daemon = start_test_daemon().await;
    
    // Submit workload
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8084/api/v1/workload/submit")
        .json(&WorkloadRequest {
            biome_yaml: TEST_BIOME_YAML,
            requester: "test-client".to_string(),
            // ...
        })
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    let result: WorkloadResponse = response.json().await.unwrap();
    assert!(result.workload_id.is_valid());
}

#[tokio::test]
async fn test_resource_reporting() {
    let daemon = start_test_daemon().await;
    
    // Query resources
    let response = reqwest::get("http://localhost:8084/api/v1/resources")
        .await
        .unwrap();
    
    let resources: ResourceSnapshot = response.json().await.unwrap();
    
    assert!(resources.cpu.total_cores > 0);
    assert!(resources.memory.total_bytes > 0);
}
```

---

## 📊 Implementation Timeline

| Phase | Effort | Dependencies | Deliverable |
|-------|--------|--------------|-------------|
| Phase 1: Dual-Mode CLI | 2-3h | None | `toadstool daemon` command |
| Phase 2: Daemon Server Core | 4-6h | Phase 1 | HTTP server + biomeOS registration |
| Phase 3: Workload Manager | 4-6h | Phase 2 | Workload queue & execution |
| Phase 4: Resource Monitor | 2-3h | Phase 2 | Resource tracking |
| Phase 5: API Handlers | 3-4h | Phase 3, 4 | Complete API endpoints |
| Phase 6: Integration Tests | 2-3h | Phase 5 | Test suite |
| **Total** | **17-25h** | | **Production-ready daemon** |

**Parallelization**: Phases 3 and 4 can be developed in parallel after Phase 2.

---

## 🎯 Grade Impact

### Current: A+ (97/100)

**Categories to Improve**:
- Architecture: 98/100 → 100/100 (+2) - Complete ecosystem integration
- External Agnostic: 98/100 → 100/100 (+2) - Multi-tower coordination

**New Grade**: A+ (98/100) (+1 overall)

**Why Only +1?**
- Daemon mode is architecture extension, not fix
- Already have excellent foundation
- This completes the ecosystem vision

---

## 🔄 CLI vs Daemon Comparison

| Feature | CLI Mode | Daemon Mode |
|---------|----------|-------------|
| **Invocation** | `toadstool run biome.yaml` | `toadstool daemon --register` |
| **Lifecycle** | Exits after completion | Runs continuously |
| **Discovery** | Uses infant discovery | Registers + uses infant discovery |
| **API** | CLI only | HTTP + Unix socket |
| **Workloads** | Single, synchronous | Multiple, async/queued |
| **Use Case** | Direct project execution | Ecosystem compute service |
| **Integration** | Standalone | Full primal integration |
| **Resources** | Project-local | System-wide coordination |

---

## 🎊 Benefits

### 1. **API-Driven Workload Execution**
- Other primals request compute via HTTP
- Remote workload submission
- Programmatic control

### 2. **Resource Pooling**
- Multiple towers share compute
- Load balancing across instances
- Efficient utilization

### 3. **Persistent Services**
- Long-running workloads (databases, servers)
- Automatic restart on failure
- State persistence

### 4. **True Fungal Behavior** 🍄
- **CLI**: Specialized fruiting body (specific project)
- **Daemon**: Mycelium network (shared compute)
- Same organism, different forms!

---

## 🔧 Use Case Examples

### Use Case 1: BearDog Requests ML Inference

```rust
// BearDog discovers ToadStool via biomeOS
let compute = biomeos.get_provider(Capability::Compute).await?;

// Submit inference workload
let response = compute.post("/api/v1/workload/submit", json!({
    "type": "ml_inference",
    "model": "trust_evaluator_v2",
    "input": suspicious_transaction,
    "requester": "beardog-fraud-detection",
    "priority": "high"
})).await?;

// Wait for result
let result = compute.get(
    &format!("/api/v1/workload/{}", response.workload_id)
).await?;
```

### Use Case 2: Multi-Tower Load Balancing

```rust
// Tower 2 is overloaded, discovers available ToadStool on Tower 1
let available_compute = biomeos
    .get_provider_with_capacity(
        Capability::Compute,
        ResourceRequirements { cpu: 4, mem: "8Gi" }
    )
    .await?;

// Offload workload to Tower 1
available_compute.submit_workload(heavy_computation).await?;
```

### Use Case 3: Persistent Database Service

```yaml
# biome.yaml
services:
  - name: postgres-analytics
    type: container
    image: postgres:16
    persistent: true
    restart_policy: always
```

```bash
# Submit to daemon
curl -X POST http://localhost:8084/api/v1/workload/submit \
  -H "Content-Type: application/json" \
  -d '{
    "biome_yaml": "...",
    "type": "persistent",
    "restart_policy": "always"
  }'

# ToadStool daemon manages lifecycle
# - Starts postgres
# - Monitors health
# - Restarts on failure
# - Persists across daemon restarts
```

---

## 📋 Next Steps

### Immediate (This Session)
1. ✅ Document daemon mode architecture
2. ⏳ Create implementation roadmap
3. ⏳ Estimate effort and timeline

### Next Session
1. Implement Phase 1 (Dual-Mode CLI)
2. Implement Phase 2 (Daemon Server Core)
3. Test biomeOS registration

### Following Sessions
1. Complete Workload Manager (Phase 3)
2. Add Resource Monitor (Phase 4)
3. Implement API handlers (Phase 5)
4. Full integration testing (Phase 6)

---

## 🏆 Conclusion

**ToadStool daemon mode is**:
- ✅ Perfect alignment with infant discovery
- ✅ Natural ecosystem evolution
- ✅ Reuses existing BiomeExecutor
- ✅ Clean dual-mode architecture
- ✅ Production-ready in 20-26 hours

**Like the fungus**:
- **Fruiting body** (CLI) → Specialized, project-specific
- **Mycelium** (Daemon) → Network-wide, resource-sharing

**Grade impact**: A+ (97/100) → A+ (98/100)

🦀 **Ready to implement!** 🍄

---

*Created: January 4, 2026*  
*Status: Ready for Implementation*  
*Estimated Timeline: 3-4 weeks (part-time) or 1 week (full-time)*

