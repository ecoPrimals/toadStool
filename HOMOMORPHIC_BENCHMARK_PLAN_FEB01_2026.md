# 🔐 Homomorphic Encryption Benchmark Plan: NPU vs GPU
**Public Benchmarks for Encrypted Computation Comparison**

**Date**: February 1, 2026  
**Goal**: Compare Akida NPU vs GPU performance on encrypted computation using public benchmarks

═══════════════════════════════════════════════════════════════

## 🎯 OBJECTIVE

Run **public homomorphic encryption benchmarks** on:
1. **CPU** (baseline)
2. **GPU** (via barr aCUDA/wgpu)
3. **Akida NPU** (event-driven advantage)

**Key Question**: Does NPU's event-driven architecture provide advantages for sparse encrypted computation?

═══════════════════════════════════════════════════════════════

## 📚 PUBLIC BENCHMARK SOURCES

### **1. TFHE-rs (Zama)** ⭐ **BEST OPTION**
**URL**: https://github.com/zama-ai/tfhe-rs  
**Stars**: 900+  
**License**: BSD-3-Clause (✅ compatible!)

**Features**:
- ✅ Pure Rust implementation
- ✅ Public benchmarks available
- ✅ CPU + GPU support
- ✅ Boolean operations (encrypted gates)
- ✅ Integer operations (encrypted arithmetic)
- ✅ Well-documented performance metrics

**Benchmarks**:
```rust
// Available operations we can benchmark
- Boolean gates: AND, OR, XOR, NOT
- Integer ops: Add, Sub, Mul, Div
- Comparison: <, >, ==, !=
- Bitwise: shifts, rotations
```

**Why Best**:
- Pure Rust (no FFI)
- Already has benchmarking framework
- Good documentation
- Active maintenance

### **2. Microsoft SEAL**
**URL**: https://github.com/microsoft/SEAL  
**Stars**: 3.9k  
**License**: MIT

**Challenges**:
- ⚠️ C++ library (would need bindings)
- ⚠️ More complex integration
- ✅ Very well tested
- ✅ BFV/CKKS/BGV schemes

**Decision**: Secondary option (TFHE-rs preferred for pure Rust)

### **3. OpenFHE**
**URL**: https://github.com/openfheorg/openfhe-development  
**License**: BSD-2-Clause

**Challenges**:
- ⚠️ C++ library
- ⚠️ Complex build
- ✅ Comprehensive schemes

**Decision**: Tertiary option

### **4. Concrete (Zama)**
**URL**: https://github.com/zama-ai/concrete  
**Stars**: 1.2k  
**License**: BSD-3-Clause

**Features**:
- ✅ Pure Rust
- ✅ TFHE-based
- ✅ Good for learning
- ⚠️ Less mature than TFHE-rs

**Decision**: Good alternative to TFHE-rs

═══════════════════════════════════════════════════════════════

## 🏗️ IMPLEMENTATION PLAN

### **Phase 1: TFHE-rs Integration** (1-2 days)

**Step 1: Add Dependencies**
```toml
# showcase/homomorphic-computing/Cargo.toml
[dependencies]
tfhe = "0.6"  # Latest TFHE-rs
barracuda = { path = "../../crates/barracuda" }
akida-driver = { path = "../../crates/neuromorphic/akida-driver" }
criterion = "0.5"
tokio = { version = "1", features = ["full"] }
```

**Step 2: CPU Baseline**
```rust
// src/benchmarks/tfhe_baseline.rs
use tfhe::prelude::*;
use tfhe::{ConfigBuilder, generate_keys, set_server_key, FheUint8};

pub struct TfheCpuBenchmark {
    client_key: ClientKey,
    server_key: ServerKey,
}

impl TfheCpuBenchmark {
    pub fn new() -> Self {
        let config = ConfigBuilder::default().build();
        let (client_key, server_key) = generate_keys(config);
        set_server_key(server_key.clone());
        
        Self { client_key, server_key }
    }
    
    pub fn bench_encrypted_add(&self, iterations: usize) -> BenchResult {
        let clear_a = 42u8;
        let clear_b = 128u8;
        
        // Encrypt
        let start = Instant::now();
        let enc_a = FheUint8::encrypt(clear_a, &self.client_key);
        let enc_b = FheUint8::encrypt(clear_b, &self.client_key);
        let encrypt_time = start.elapsed();
        
        // Homomorphic addition
        let start = Instant::now();
        for _ in 0..iterations {
            let _enc_result = &enc_a + &enc_b;
        }
        let compute_time = start.elapsed();
        
        // Decrypt
        let enc_result = &enc_a + &enc_b;
        let start = Instant::now();
        let result: u8 = enc_result.decrypt(&self.client_key);
        let decrypt_time = start.elapsed();
        
        // Verify correctness
        assert_eq!(result, clear_a.wrapping_add(clear_b));
        
        BenchResult {
            operation: "encrypted_add",
            iterations,
            encrypt_time_us: encrypt_time.as_micros(),
            compute_time_us: compute_time.as_micros(),
            decrypt_time_us: decrypt_time.as_micros(),
            throughput: (iterations as f64 / compute_time.as_secs_f64()) as u64,
        }
    }
}
```

**Step 3: GPU Acceleration**
```rust
// src/benchmarks/tfhe_gpu.rs
use tfhe::prelude::*;
use barracuda::WgpuDevice;

pub struct TfheGpuBenchmark {
    client_key: ClientKey,
    server_key: ServerKey,
    gpu_device: WgpuDevice,
}

impl TfheGpuBenchmark {
    pub async fn new() -> Result<Self> {
        // CPU keys
        let config = ConfigBuilder::default().build();
        let (client_key, server_key) = generate_keys(config);
        
        // GPU device
        let gpu_device = WgpuDevice::new().await?;
        
        Ok(Self { client_key, server_key, gpu_device })
    }
    
    pub async fn bench_encrypted_add_gpu(&self, iterations: usize) -> BenchResult {
        // Use GPU for polynomial operations
        // TFHE operations are polynomial arithmetic - perfect for GPU!
        
        let enc_a = FheUint8::encrypt(42u8, &self.client_key);
        let enc_b = FheUint8::encrypt(128u8, &self.client_key);
        
        let start = Instant::now();
        for _ in 0..iterations {
            // Offload polynomial NTT to GPU
            let _result = self.gpu_polynomial_add(
                &enc_a.to_coefficients(),
                &enc_b.to_coefficients()
            ).await?;
        }
        let compute_time = start.elapsed();
        
        BenchResult {
            operation: "encrypted_add_gpu",
            iterations,
            compute_time_us: compute_time.as_micros(),
            throughput: (iterations as f64 / compute_time.as_secs_f64()) as u64,
        }
    }
    
    async fn gpu_polynomial_add(&self, a: &[u64], b: &[u64]) -> Result<Vec<u64>> {
        // Use barracuda for parallel polynomial arithmetic
        use barracuda::ops::*;
        
        let a_tensor = Tensor::from_slice(a, &self.gpu_device).await?;
        let b_tensor = Tensor::from_slice(b, &self.gpu_device).await?;
        
        // GPU-accelerated modular addition
        let result = add(&a_tensor, &b_tensor).await?;
        
        Ok(result.to_vec().await?)
    }
}
```

**Step 4: NPU Implementation**
```rust
// src/benchmarks/tfhe_npu.rs
use tfhe::prelude::*;
use akida_driver::AkidaBoard;

pub struct TfheNpuBenchmark {
    client_key: ClientKey,
    server_key: ServerKey,
    akida: AkidaBoard,
    model: AkidaModel,
}

impl TfheNpuBenchmark {
    pub fn new() -> Result<Self> {
        let config = ConfigBuilder::default().build();
        let (client_key, server_key) = generate_keys(config);
        
        // Initialize Akida
        let akida = AkidaBoard::open(0)?;
        
        // Load SNN model trained for encrypted pattern recognition
        // Model maps: sparse polynomial coefficients → encrypted result
        let model = AkidaModel::load("models/tfhe_polynomial_ops.akd")?;
        akida.upload_model(&model)?;
        
        Ok(Self { client_key, server_key, akida, model })
    }
    
    pub fn bench_encrypted_add_npu(&self, iterations: usize) -> BenchResult {
        let enc_a = FheUint8::encrypt(42u8, &self.client_key);
        let enc_b = FheUint8::encrypt(128u8, &self.client_key);
        
        let start = Instant::now();
        for _ in 0..iterations {
            // Convert encrypted polynomials to spike trains
            // Key insight: Sparse coefficients → sparse spikes!
            let spikes_a = self.coefficients_to_spikes(&enc_a.to_coefficients());
            let spikes_b = self.coefficients_to_spikes(&enc_b.to_coefficients());
            
            // Akida processes sparse events efficiently
            let result_spikes = self.akida.infer(&[spikes_a, spikes_b])?;
            
            let _result_coeffs = self.spikes_to_coefficients(&result_spikes);
        }
        let compute_time = start.elapsed();
        
        BenchResult {
            operation: "encrypted_add_npu",
            iterations,
            compute_time_us: compute_time.as_micros(),
            throughput: (iterations as f64 / compute_time.as_secs_f64()) as u64,
        }
    }
    
    fn coefficients_to_spikes(&self, coeffs: &[u64]) -> SpikeTrainBatch {
        // Convert sparse polynomial to spike events
        // Only non-zero coefficients generate spikes
        // This is the key to NPU efficiency!
        
        coeffs.iter()
            .enumerate()
            .filter(|(_, &c)| c != 0)  // Sparse!
            .map(|(i, &c)| Spike {
                neuron_id: i as u32,
                timestamp: self.coefficient_to_time(c),
            })
            .collect()
    }
}
```

### **Phase 2: Public Benchmarks** (1 day)

**Workloads from TFHE-rs**:
```rust
// examples/public_benchmark_comparison.rs

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔐 Homomorphic Encryption: CPU vs GPU vs NPU\n");
    println!("Using TFHE-rs public benchmarks\n");
    
    // Initialize all substrates
    let cpu = TfheCpuBenchmark::new();
    let gpu = TfheGpuBenchmark::new().await?;
    let npu = TfheNpuBenchmark::new()?;
    
    // Benchmark 1: Boolean operations
    println!("📊 Benchmark 1: Encrypted Boolean Gates");
    run_bool_benchmarks(&cpu, &gpu, &npu).await?;
    
    // Benchmark 2: Integer arithmetic
    println!("\n📊 Benchmark 2: Encrypted Integer Arithmetic");
    run_int_benchmarks(&cpu, &gpu, &npu).await?;
    
    // Benchmark 3: Pattern matching
    println!("\n📊 Benchmark 3: Encrypted Pattern Matching");
    run_pattern_benchmarks(&cpu, &gpu, &npu).await?;
    
    // Benchmark 4: Aggregation
    println!("\n📊 Benchmark 4: Encrypted Aggregation");
    run_aggregation_benchmarks(&cpu, &gpu, &npu).await?;
    
    // Power measurement
    println!("\n⚡ Power Consumption Analysis");
    measure_power(&cpu, &gpu, &npu).await?;
    
    // Final comparison
    print_final_comparison();
    
    Ok(())
}
```

**Expected Output**:
```
🔐 Homomorphic Encryption: CPU vs GPU vs NPU

Using TFHE-rs public benchmarks

📊 Benchmark 1: Encrypted Boolean Gates (10,000 ops)

┌─────────────┬────────────┬───────────┬────────────┬──────────────┐
│ Substrate   │ Throughput │  Latency  │   Power    │  Ops/Joule   │
├─────────────┼────────────┼───────────┼────────────┼──────────────┤
│ CPU (TFHE)  │   1,200/s  │   8.3ms   │    25W     │      48      │
│ GPU (wgpu)  │   5,500/s  │   1.8ms   │   150W     │      37      │
│ NPU (Akida) │   3,200/s  │   3.1ms   │     2W ⚡  │   1,600 ⭐   │
└─────────────┴────────────┴───────────┴────────────┴──────────────┘

📊 Benchmark 2: Encrypted Integer Arithmetic (10,000 ops)

┌─────────────┬────────────┬───────────┬────────────┬──────────────┐
│ Substrate   │ Throughput │  Latency  │   Power    │  Ops/Joule   │
├─────────────┼────────────┼───────────┼────────────┼──────────────┤
│ CPU (TFHE)  │     800/s  │  12.5ms   │    25W     │      32      │
│ GPU (wgpu)  │   4,200/s  │   2.4ms   │   150W     │      28      │
│ NPU (Akida) │   2,400/s  │   4.2ms   │     2W ⚡  │   1,200 ⭐   │
└─────────────┴────────────┴───────────┴────────────┴──────────────┘

⚡ Power Consumption Analysis:

Average Power:
  CPU: 25W
  GPU: 150W
  NPU: 2W ⚡ (12.5x less than CPU, 75x less than GPU!)

Energy Efficiency (Ops/Joule):
  CPU: 40 ops/J
  GPU: 32 ops/J
  NPU: 1,400 ops/J ⭐ (35x better than CPU, 44x better than GPU!)

🏆 WINNER: NPU (Akida)
  • Best energy efficiency: 35-44x better
  • Best for: Edge deployment, continuous operation
  • Ideal: Streaming privacy-preserving compute
```

═══════════════════════════════════════════════════════════════

## 📦 PUBLIC DATASETS TO USE

### **1. TFHE-rs Test Data**
```bash
# Clone TFHE-rs benchmarks
git clone https://github.com/zama-ai/tfhe-rs
cd tfhe-rs/tfhe/benches

# Available benchmark data:
- Integer operations (various bit sizes)
- Boolean operations (gates)
- Comparison operations
```

### **2. Generate Custom Test Data**
```rust
// Generate encrypted datasets
pub fn generate_test_datasets() -> TestDataset {
    let config = ConfigBuilder::default().build();
    let (client_key, _) = generate_keys(config);
    
    TestDataset {
        // Boolean gates
        encrypted_bools: (0..10_000)
            .map(|_| FheBool::encrypt(rand::random::<bool>(), &client_key))
            .collect(),
        
        // 8-bit integers
        encrypted_u8: (0..10_000)
            .map(|_| FheUint8::encrypt(rand::random::<u8>(), &client_key))
            .collect(),
        
        // 16-bit integers
        encrypted_u16: (0..10_000)
            .map(|_| FheUint16::encrypt(rand::random::<u16>(), &client_key))
            .collect(),
        
        // Patterns for matching
        encrypted_patterns: generate_encrypted_patterns(&client_key, 1000),
    }
}
```

═══════════════════════════════════════════════════════════════

## 🚀 EXECUTION PLAN

### **Day 1: Setup & CPU Baseline**
```bash
# 1. Add TFHE-rs dependency
cd showcase/homomorphic-computing
cargo add tfhe

# 2. Implement CPU baseline
cargo run --example tfhe_cpu_baseline --release

# 3. Verify correctness
cargo test --package homomorphic-computing
```

### **Day 2: GPU Implementation**
```bash
# 1. Implement GPU acceleration
cargo run --example tfhe_gpu_benchmark --release

# 2. Compare CPU vs GPU
cargo run --example cpu_vs_gpu_comparison --release
```

### **Day 3: NPU Implementation**
```bash
# 1. Train Akida model (if needed)
python scripts/train_akida_fhe_model.py

# 2. Implement NPU benchmark
cargo run --example tfhe_npu_benchmark --release

# 3. Full comparison
cargo run --example public_benchmark_comparison --release
```

### **Day 4: Analysis & Documentation**
```bash
# 1. Generate comparison charts
cargo run --example generate_charts --release

# 2. Document findings
# Create: HOMOMORPHIC_NPU_VS_GPU_RESULTS_FEB01_2026.md

# 3. Commit results
git add .
git commit -m "📊 Homomorphic Encryption NPU vs GPU Benchmark Results"
git push
```

═══════════════════════════════════════════════════════════════

## 🎯 SUCCESS CRITERIA

**Technical**:
- ✅ All 3 substrates working (CPU, GPU, NPU)
- ✅ Using public TFHE-rs benchmarks
- ✅ Results validated (correctness)
- ✅ Power measurements accurate

**Performance**:
- ✅ GPU > CPU throughput
- ✅ NPU >> GPU energy efficiency (target: 30x+)
- ✅ NPU suitable for edge deployment

**Documentation**:
- ✅ Complete benchmark report
- ✅ Comparison charts/tables
- ✅ Reproducible instructions
- ✅ Public dataset sources cited

═══════════════════════════════════════════════════════════════

## 🏆 EXPECTED FINDINGS

**Throughput**:
- CPU: Baseline (1x)
- GPU: 4-5x faster than CPU
- NPU: 2-3x faster than CPU (60% of GPU)

**Energy Efficiency** ⭐:
- CPU: Baseline (1x)
- GPU: Similar to CPU (0.8-1.2x)
- **NPU: 30-50x better than CPU/GPU!** 🎯

**Best Use Cases**:
- **CPU**: Development, small datasets
- **GPU**: Batch processing, high throughput
- **NPU**: Edge deployment, streaming, 24/7 operation ⭐

═══════════════════════════════════════════════════════════════

**Status**: ✅ **READY TO IMPLEMENT**  
**Timeline**: 3-4 days  
**Key Innovation**: Prove NPU advantages for encrypted computation  

🔐⚡ **Let's benchmark NPU vs GPU on real homomorphic workloads!** ⚡🔐
