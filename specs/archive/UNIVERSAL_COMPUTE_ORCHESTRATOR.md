# Universal Compute Orchestrator Specification

**Version**: 1.0.0  
**Status**: PROPOSED  
**Date**: January 17, 2026  
**Authors**: ToadStool Core Team

---

## 1. OVERVIEW

### 1.1 Purpose

This specification defines ToadStool's evolution into a **Universal Compute Orchestrator** - a 100% Pure Rust platform capable of executing workloads across multiple runtime environments while maintaining zero C dependencies in its core binary.

### 1.2 Design Philosophy

**Core Principle**: Architectural Inversion
- **Traditional**: Application depends on runtime (C/WASM embedded in binary)
- **ToadStool**: Application orchestrates runtime (C/WASM as external workloads)

**Key Insight**: Treat C/WASM-JIT like Python - as orchestrated runtimes, not embedded dependencies.

### 1.3 Goals

1. **100% Pure Rust Core**: ToadStool binary contains zero C code
2. **TRUE UniBin**: Trivial cross-compilation to any architecture
3. **Runtime Flexibility**: Execute any workload type optimally
4. **Security**: All external runtimes execute in sandboxes
5. **Ecosystem Service**: Universal compute platform for all primals

---

## 2. ARCHITECTURE

### 2.1 Core Components

```
ToadStool Core (100% Pure Rust)
│
├─ Runtime Manager
│  ├─ Runtime Registry
│  ├─ Runtime Selection Logic
│  └─ Runtime Health Monitoring
│
├─ Workload Scheduler
│  ├─ Queue Management
│  ├─ Priority Scheduling
│  └─ Load Balancing
│
├─ Sandbox Manager
│  ├─ seccomp Filters
│  ├─ Linux Namespaces
│  └─ Resource Limits
│
└─ Runtime Executors
   ├─ Native Executor (Rust)
   ├─ WASM Interpreter Executor (wasmi)
   ├─ WASM JIT Executor (wasmtime subprocess)
   ├─ Python Executor (subprocess)
   ├─ C/C++ Executor (compile + sandbox)
   └─ Container Executor (future)
```

### 2.2 Runtime Portfolio

| Runtime | Implementation | Startup | Execution | Use Case |
|---------|---------------|---------|-----------|----------|
| **Native** | Compiled Rust | Instant | Fastest | Production code |
| **WASM (wasmi)** | Pure Rust interpreter | Fast | Moderate | Plugins, short tasks |
| **WASM-JIT** | wasmtime subprocess | Slow | Fast | Long-running compute |
| **Python** | subprocess | Moderate | Moderate | AI/ML, scripting |
| **C/C++** | gcc/clang + sandbox | Slow | Fast | Legacy code |
| **Container** | Docker/Podman | Slow | Isolated | Maximum isolation |

### 2.3 Dependency Model

**Embedded Dependencies** (Compiled into ToadStool):
- Pure Rust crates only
- No C/C++ libraries
- No foreign function interfaces (except system libc)

**External Runtimes** (System tools/binaries):
- Python interpreter (`/usr/bin/python3`)
- C compilers (`gcc`, `clang`)
- WASM JIT (`wasmtime` CLI)
- Container engines (`docker`, `podman`)

**Key Distinction**: Runtimes are capabilities (detected at runtime), not obligations (required at compile time).

---

## 3. RUNTIME SPECIFICATIONS

### 3.1 Native Runtime (Rust)

**Type**: Compiled, in-process  
**Language**: Rust  
**Startup**: Instant (already loaded)  
**Execution**: Native speed  
**Isolation**: ToadStool process  

**Implementation**:
- Rust workloads compiled as dynamic libraries (.so)
- Loaded via `libloading` crate
- Execute in ToadStool's process space
- Resource limits via cgroups

**Security**:
- Same process (minimal isolation)
- Rust memory safety guarantees
- Suitable for trusted workloads only

---

### 3.2 WASM Interpreter Runtime (wasmi)

**Type**: Pure Rust interpreter  
**Version**: wasmi v1.0+  
**Startup**: Fast (< 1ms for small modules)  
**Execution**: 2-10× slower than native  
**Isolation**: Sandboxed memory  

**Implementation**:
```rust
use wasmi::{Engine, Module, Store, Linker};

pub struct WasmiRuntime {
    engine: Engine,
    linker: Linker<()>,
}

impl WasmiRuntime {
    pub fn new() -> Self {
        let engine = Engine::default();
        let linker = Linker::new(&engine);
        Self { engine, linker }
    }
    
    pub async fn execute(&self, wasm_bytes: &[u8]) -> Result<Output> {
        let module = Module::new(&self.engine, wasm_bytes)?;
        let mut store = Store::new(&self.engine, ());
        let instance = self.linker.instantiate(&mut store, &module)?;
        
        // Execute entry point
        let result = instance.get_typed_func::<(), i32>(&mut store, "main")?
            .call(&mut store, ())?;
        
        Ok(Output { exit_code: result })
    }
}
```

**Use Cases**:
- Plugin systems (fast load times critical)
- Event handlers (short execution)
- Untrusted code (security critical)
- Embedded systems (resource constrained)

**Performance Characteristics**:
- Startup: Orders of magnitude faster than JIT
- Memory: Low overhead (interpreter state only)
- Execution: Acceptable for < 10 second workloads

---

### 3.3 WASM JIT Runtime (wasmtime)

**Type**: External subprocess  
**Version**: wasmtime v20.0+  
**Startup**: Slow (compilation overhead)  
**Execution**: Near-native speed  
**Isolation**: Process boundary + sandbox  

**Implementation**:
```rust
pub struct WasmJITRuntime {
    wasmtime_path: PathBuf,
    sandbox: Sandbox,
}

impl WasmJITRuntime {
    pub async fn execute(&self, wasm_bytes: &[u8]) -> Result<Output> {
        // Write WASM to temp file
        let temp_file = NamedTempFile::new()?;
        temp_file.write_all(wasm_bytes)?;
        
        // Execute via wasmtime CLI in sandbox
        let output = self.sandbox
            .execute_command(&self.wasmtime_path, &["run", temp_file.path()])
            .await?;
        
        Ok(output)
    }
}
```

**Use Cases**:
- Long-running compute (> 10 seconds)
- Performance-critical WASM
- Data processing pipelines
- Scientific computing

**Performance Characteristics**:
- Startup: 100-1000ms (compile WASM → native)
- Memory: Higher (JIT compiler + optimizations)
- Execution: 90-95% of native speed

---

### 3.4 Python Runtime

**Type**: External subprocess  
**Version**: Python 3.8+  
**Startup**: Moderate (50-200ms)  
**Execution**: Moderate (interpreted)  
**Isolation**: Process boundary  

**Implementation**:
```rust
pub struct PythonRuntime {
    python_path: PathBuf,
    sandbox: Sandbox,
}

impl PythonRuntime {
    pub async fn execute(&self, python_code: &str) -> Result<Output> {
        let temp_file = NamedTempFile::new()?;
        temp_file.write_all(python_code.as_bytes())?;
        
        let output = self.sandbox
            .execute_command(&self.python_path, &[temp_file.path()])
            .await?;
        
        Ok(output)
    }
}
```

**Use Cases**:
- AI/ML workloads (NumPy, PyTorch, etc.)
- Data analysis scripts
- Rapid prototyping
- Ecosystem integration

---

### 3.5 C/C++ Runtime

**Type**: Compile + sandbox execution  
**Version**: gcc 9+, clang 10+  
**Startup**: Slow (compilation required)  
**Execution**: Native speed  
**Isolation**: Process boundary + sandbox  

**Implementation**:
```rust
pub struct CRuntime {
    compiler: Compiler,
    sandbox: Sandbox,
}

pub enum Compiler {
    GCC { path: PathBuf, flags: Vec<String> },
    Clang { path: PathBuf, flags: Vec<String> },
}

impl CRuntime {
    pub async fn execute(&self, c_code: &str) -> Result<Output> {
        // 1. Write source to temp file
        let source_file = NamedTempFile::with_suffix(".c")?;
        source_file.write_all(c_code.as_bytes())?;
        
        // 2. Compile
        let binary_file = NamedTempFile::new()?;
        self.compile(&source_file, &binary_file).await?;
        
        // 3. Execute in sandbox
        let output = self.sandbox
            .execute_binary(&binary_file)
            .await?;
        
        Ok(output)
    }
    
    async fn compile(&self, source: &Path, output: &Path) -> Result<()> {
        let status = Command::new(self.compiler.path())
            .args(["-o", output.to_str().unwrap(), source.to_str().unwrap()])
            .args(self.compiler.flags())
            .status()
            .await?;
        
        if !status.success() {
            return Err(anyhow!("Compilation failed"));
        }
        
        Ok(())
    }
}
```

**Use Cases**:
- Legacy C/C++ libraries
- Performance-critical algorithms
- System-level code
- Ecosystem compatibility

**Security Considerations**:
- Compilation in isolated environment
- Binary execution in strict sandbox
- Memory limits enforced
- No network access by default

---

## 4. RUNTIME SELECTION

### 4.1 Selection Strategies

**Manual Selection**:
```rust
let workload = Workload {
    code: wasm_bytes,
    runtime: RuntimeType::WasmInterpreter,  // Explicit
};
```

**Auto-Selection**:
```rust
let workload = Workload {
    code: wasm_bytes,
    runtime: RuntimeType::Auto {
        hints: SelectionHints {
            estimated_duration: Some(Duration::from_secs(5)),
            priority: Priority::High,
            trusted: false,
        }
    },
};
```

### 4.2 Selection Heuristics

**For WASM workloads**:
```
IF estimated_duration < 10 seconds:
    → Use WASM Interpreter (wasmi)
ELSE IF estimated_duration >= 10 seconds:
    → Use WASM JIT (wasmtime)
ELSE IF no estimate:
    → Use WASM Interpreter (safe default)
```

**For C workloads**:
```
IF trusted AND native_lib_available:
    → Use Native Runtime (dynamic library)
ELSE:
    → Use C Runtime (compile + sandbox)
```

**For Python workloads**:
```
ALWAYS use Python Runtime (no alternatives)
```

### 4.3 Fallback Strategy

```
PRIMARY RUNTIME FAILED
    ↓
CHECK FALLBACK QUEUE
    ↓
TRY ALTERNATIVE RUNTIME
    ↓
SUCCESS → Return result
FAILURE → Return error with fallback log
```

Example fallback chains:
- WASM-JIT → WASM-Interpreter → Error
- Native → Container → Error
- Python3.12 → Python3.11 → Python3.10 → Error

---

## 5. SECURITY MODEL

### 5.1 Sandbox Architecture

**Linux Namespaces**:
- `CLONE_NEWPID`: Isolated process tree
- `CLONE_NEWNET`: Network isolation
- `CLONE_NEWNS`: Mount namespace isolation
- `CLONE_NEWUTS`: Hostname isolation

**seccomp Filters**:
```rust
pub struct SeccompProfile {
    pub allow_list: Vec<Syscall>,
    pub deny_list: Vec<Syscall>,
    pub default_action: Action,
}

// Example: WASM runtime profile
let wasm_profile = SeccompProfile {
    allow_list: vec![
        Syscall::Read,
        Syscall::Write,
        Syscall::Exit,
        Syscall::Mmap,
        Syscall::Munmap,
    ],
    deny_list: vec![
        Syscall::Socket,  // No network
        Syscall::Fork,    // No spawning
        Syscall::Execve,  // No exec
    ],
    default_action: Action::Deny,
};
```

**Resource Limits** (via cgroups v2):
- CPU: Quota-based limiting
- Memory: Hard limits with OOM handling
- I/O: Bandwidth throttling
- PIDs: Maximum process count

### 5.2 Trust Levels

**Trusted**:
- Source: Verified primals
- Isolation: Minimal (same process or light sandbox)
- Resources: Higher limits
- Examples: Native runtime, internal WASM

**Untrusted**:
- Source: External users, unknown code
- Isolation: Maximum (strict sandbox)
- Resources: Lower limits
- Examples: User WASM, arbitrary C code

### 5.3 Audit Trail

All runtime executions logged:
```rust
pub struct ExecutionAuditLog {
    pub workload_id: Uuid,
    pub runtime: RuntimeType,
    pub start_time: SystemTime,
    pub end_time: SystemTime,
    pub exit_code: i32,
    pub resource_usage: ResourceUsage,
    pub security_events: Vec<SecurityEvent>,
}
```

---

## 6. API SPECIFICATION

### 6.1 Workload Submission

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadSubmission {
    /// Workload identifier
    pub id: Option<Uuid>,
    
    /// Runtime selection
    pub runtime: RuntimeSelection,
    
    /// Code/binary to execute
    pub payload: WorkloadPayload,
    
    /// Execution configuration
    pub config: ExecutionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuntimeSelection {
    /// Use specific runtime
    Explicit(RuntimeType),
    
    /// Auto-select based on hints
    Auto {
        hints: SelectionHints,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkloadPayload {
    /// WebAssembly bytecode
    Wasm { bytes: Vec<u8> },
    
    /// Native Rust code
    Rust { source: String },
    
    /// Python script
    Python { source: String },
    
    /// C/C++ source
    C { source: String, compiler: CCompiler },
    
    /// Pre-compiled binary
    Binary { bytes: Vec<u8>, format: BinaryFormat },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    /// Resource limits
    pub limits: ResourceLimits,
    
    /// Timeout
    pub timeout: Duration,
    
    /// Environment variables
    pub env: HashMap<String, String>,
    
    /// Input data
    pub stdin: Option<Vec<u8>>,
}
```

### 6.2 Workload Status

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadStatus {
    pub id: Uuid,
    pub state: WorkloadState,
    pub runtime: RuntimeType,
    pub created_at: SystemTime,
    pub started_at: Option<SystemTime>,
    pub completed_at: Option<SystemTime>,
    pub resource_usage: Option<ResourceUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkloadState {
    Queued,
    Preparing,
    Running,
    Completed { exit_code: i32 },
    Failed { error: String },
    Timeout,
    Cancelled,
}
```

### 6.3 Runtime Capabilities

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeCapabilities {
    /// Available runtimes
    pub runtimes: Vec<RuntimeInfo>,
    
    /// System resources
    pub resources: SystemResources,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeInfo {
    pub runtime_type: RuntimeType,
    pub version: String,
    pub available: bool,
    pub health: HealthStatus,
    pub performance: PerformanceProfile,
}
```

---

## 7. IMPLEMENTATION PHASES

### Phase 1: Pure Rust Core (Week 1)

**Objective**: Remove all C dependencies from ToadStool binary

**Tasks**:
1. Migrate `sys-info` → `sysinfo` (1-2 hours)
2. Implement wasmi WASM runtime (3-4 days)
3. Update build system and CI/CD (1 day)
4. Testing and validation (1-2 days)

**Success Criteria**:
- `cargo tree | grep -E "\-sys "` shows only thin wrappers
- ARM cross-compilation works without C toolchain
- All existing tests pass

**Deliverables**:
- ToadStool binary: 100% Pure Rust
- wasmi runtime: Functional and tested
- Documentation: Migration guide

---

### Phase 2: C/C++ Runtime (Week 2)

**Objective**: Enable C/C++ code execution as orchestrated runtime

**Tasks**:
1. Implement C compiler detection (1 day)
2. Build sandbox execution framework (2 days)
3. Create C runtime API (1 day)
4. Security testing (2 days)
5. Documentation (1 day)

**Success Criteria**:
- Can compile and execute C code
- Sandbox prevents escape attempts
- Resource limits enforced
- Audit trail complete

**Deliverables**:
- C Runtime module
- Security test suite
- API documentation

---

### Phase 3: WASM-JIT Runtime (Week 3)

**Objective**: Enable long-running WASM via wasmtime subprocess

**Tasks**:
1. Implement wasmtime subprocess executor (1-2 days)
2. Build runtime selection logic (1 day)
3. Performance benchmarking (1-2 days)
4. Auto-selection heuristics (1 day)
5. Documentation (1 day)

**Success Criteria**:
- WASM-JIT runtime functional
- Performance meets expectations
- Auto-selection works correctly
- Benchmark results documented

**Deliverables**:
- WASM-JIT runtime module
- Runtime selection engine
- Performance documentation

---

### Phase 4: Ecosystem Integration (Week 4)

**Objective**: Integrate with other primals and document patterns

**Tasks**:
1. Update RPC protocol (1 day)
2. Update primal clients (2 days)
3. E2E testing with ecosystem (2 days)
4. Documentation and examples (2 days)

**Success Criteria**:
- Other primals can request compute
- Runtime selection works across primals
- Performance acceptable
- Complete documentation

**Deliverables**:
- Updated RPC protocol
- Primal integration examples
- Ecosystem documentation

---

## 8. PERFORMANCE EXPECTATIONS

### 8.1 Baseline Metrics

**Native Runtime**:
- Startup: < 1ms
- Execution: 100% (native speed)
- Memory: Minimal overhead

**WASM Interpreter (wasmi)**:
- Startup: < 10ms (small modules)
- Execution: 10-50% of native (workload dependent)
- Memory: Low (interpreter state only)

**WASM JIT (wasmtime)**:
- Startup: 100-1000ms (compilation)
- Execution: 90-95% of native
- Memory: Moderate (JIT + optimizations)

**Python**:
- Startup: 50-200ms
- Execution: 5-50% of native (workload dependent)
- Memory: Moderate (interpreter + libraries)

**C/C++**:
- Startup: 500-5000ms (compilation)
- Execution: 100% (native speed)
- Memory: Minimal (compiled binary)

### 8.2 Optimization Targets

**Short WASM** (< 10s execution):
- Target: wasmi with < 10ms startup
- Acceptable: 20-50% of native execution speed
- Memory: < 10MB overhead

**Long WASM** (> 10s execution):
- Target: wasmtime with acceptable startup cost
- Acceptable: > 80% of native execution speed
- Memory: < 100MB overhead

---

## 9. TESTING REQUIREMENTS

### 9.1 Unit Tests

**Per Runtime**:
- Initialization and cleanup
- Basic execution
- Error handling
- Resource limits
- Timeout handling

### 9.2 Integration Tests

**Runtime Selection**:
- Manual selection
- Auto-selection heuristics
- Fallback chains

**Cross-Runtime**:
- Sequential execution
- Concurrent execution
- Resource sharing

### 9.3 Security Tests

**Sandbox**:
- Escape attempt prevention
- Resource limit enforcement
- Syscall filtering
- Network isolation

**Malicious Code**:
- Fork bombs
- Memory exhaustion
- CPU spinning
- File system attacks

### 9.4 Performance Tests

**Benchmarks**:
- Startup latency
- Execution throughput
- Memory usage
- Concurrent workloads

**Regression Tests**:
- Compare against baselines
- Track performance over time
- Identify bottlenecks

---

## 10. MONITORING & OBSERVABILITY

### 10.1 Metrics

**Runtime Metrics**:
```rust
pub struct RuntimeMetrics {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub avg_startup_ms: f64,
    pub avg_execution_ms: f64,
    pub avg_memory_mb: f64,
}
```

**System Metrics**:
- CPU utilization per runtime
- Memory usage per runtime
- I/O bandwidth per runtime
- Active workloads by runtime

### 10.2 Health Checks

**Per Runtime**:
```rust
pub struct RuntimeHealth {
    pub available: bool,
    pub version: String,
    pub last_check: SystemTime,
    pub error_rate: f64,
    pub avg_latency: Duration,
}
```

### 10.3 Alerts

**Thresholds**:
- Error rate > 10%
- Avg latency > 5s
- Memory usage > 80%
- Runtime unavailable

---

## 11. FUTURE ENHANCEMENTS

### 11.1 Additional Runtimes

- **JavaScript/Node.js**: V8 or Deno
- **Go**: Go runtime
- **Java/JVM**: OpenJDK
- **WASI**: Full WASI preview 2 support

### 11.2 Advanced Features

- **GPU Acceleration**: CUDA/OpenCL via runtimes
- **Distributed Execution**: Multi-node workloads
- **Persistent State**: Runtime state persistence
- **Hot Reload**: Runtime updates without restart

### 11.3 Optimization

- **JIT Warmup**: Pre-compile common WASM modules
- **Runtime Pooling**: Reuse runtime instances
- **Caching**: Cache compilation artifacts
- **Predictive Selection**: ML-based runtime selection

---

## 12. COMPLIANCE & STANDARDS

### 12.1 WebAssembly

- **Core Specification**: WASM 1.0 (wasmi)
- **WASI**: Preview 1 (minimum), Preview 2 (target)
- **Extensions**: SIMD, threads, bulk memory operations

### 12.2 Security

- **CIS Benchmarks**: Linux container security
- **NIST**: Secure coding guidelines
- **CVE**: Vulnerability tracking and patching

### 12.3 Performance

- **Benchmark Suites**: Coremark, PolybenchC
- **Comparison**: vs native execution
- **SLA**: 99.9% uptime for runtime manager

---

## 13. CONCLUSION

This specification defines ToadStool's transformation into a Universal Compute Orchestrator - a 100% Pure Rust platform that can execute any workload type while maintaining zero C dependencies in its core.

**Key Innovations**:
1. **Architectural Inversion**: C/WASM-JIT as runtimes, not dependencies
2. **Runtime Flexibility**: Short + Long WASM capabilities
3. **TRUE UniBin**: Trivial cross-compilation
4. **Ecosystem Service**: Universal compute for all primals

**Expected Outcome**: World-class compute platform with no compromises!

---

**Version**: 1.0.0  
**Status**: PROPOSED  
**Next Review**: After Phase 1 completion  
**Approval Required**: Architecture Board
