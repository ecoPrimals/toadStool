# Pure Rust Akida Integration Plan

**Date**: January 29, 2026  
**Status**: Planning  
**Goal**: Migrate Akida BrainChip integration from Python SDK to pure Rust

---

## Current State Analysis

### ✅ What We Have

**Hardware**: 2x Akida AKD1000 PCIe boards installed
- `/dev/akida0` at `a1:00.0` (PCIe/NSoC_v2/0)
- `/dev/akida1` at `e2:00.0` (PCIe/NSoC_v2/1)

**Kernel Driver**: `akida_pcie` module loaded
- Driver: `/lib/modules/6.12.10-76061203-generic/kernel/drivers/akida-pcie.ko`
- Source: C driver from BrainChip (`akida_dw_edma`)
- Patched for kernel 6.12 support
- Using DMA (dw-edma) for high-speed transfers

**Rust Infrastructure**: Detection and management in pure Rust
- `showcase/neuromorphic/01-akida-detection/` - PCIe scanning, device enumeration
- `showcase/neuromorphic/02-akida-bioinformatics/` - K-mer filtering demo
- `showcase/neuromorphic/03-akida-llm-intent/` - LLM intent classification

**Current Limitation**: Actual device interaction is mocked/placeholder
- Comments say "In production, this would use the Akida SDK"
- Akida SDK is Python-based (installed via pip)
- toadStool uses PyO3 for Python interop

### ❌ What We Don't Have

1. **Pure Rust Akida SDK**: No Rust crate exists on crates.io
2. **Direct Device I/O**: Not interfacing with `/dev/akida*` directly
3. **Model Loading**: Can't load SNN models from Rust
4. **Inference Execution**: Can't run inference on Akida NPUs
5. **Memory Management**: Can't manage Akida SRAM from Rust

---

## Architecture Options

### Option 1: Rust FFI to C++ Akida Engine (Recommended Short-Term)

**Approach**: Create Rust bindings to BrainChip's C++ Akida Engine library

**Pros**:
- ✅ Officially supported by BrainChip
- ✅ Full feature parity with Python SDK
- ✅ Maintained by hardware vendor
- ✅ Can start immediately

**Cons**:
- ⚠️ Still depends on C++ library
- ⚠️ Not "pure Rust" (but pragmatic)
- ⚠️ Needs to ship C++ library with binaries

**Implementation**:
```rust
// crates/neuromorphic/akida-sys (unsafe bindings)
// bindgen to C++ Akida Engine

// crates/neuromorphic/akida (safe wrapper)
pub struct AkidaDevice {
    handle: NonNull<ffi::AkidaDevice>,
}

impl AkidaDevice {
    pub fn open(index: usize) -> Result<Self> {
        // Safe wrapper around C++ calls
    }
    
    pub fn load_model(&mut self, model: &[u8]) -> Result<ModelHandle> {
        // Load SNN model to board
    }
    
    pub fn infer(&self, input: &[f32]) -> Result<Vec<f32>> {
        // Run inference
    }
}
```

**Effort**: 2-3 weeks
**Risk**: Low (vendor-supported)

---

### Option 2: Pure Rust Driver via `/dev/akida*` (Recommended Long-Term)

**Approach**: Write pure Rust driver interfacing directly with kernel driver via ioctl

**Pros**:
- ✅ **Pure Rust** - No C/C++ dependencies
- ✅ Full control and customization
- ✅ Can optimize for toadStool use cases
- ✅ Open source contribution to Rust ecosystem

**Cons**:
- ⚠️ Requires reverse-engineering device protocol
- ⚠️ Significant development effort
- ⚠️ Risk of incompatibility with future hardware
- ⚠️ Need to maintain model format parsers

**Architecture**:
```rust
// crates/neuromorphic/akida-driver (pure Rust)

pub struct AkidaDevice {
    fd: std::os::unix::io::RawFd,
    index: usize,
}

impl AkidaDevice {
    pub fn open(index: usize) -> Result<Self> {
        let path = format!("/dev/akida{}", index);
        let fd = unsafe {
            libc::open(path.as_ptr() as *const i8, libc::O_RDWR)
        };
        // ... error handling
    }
    
    pub fn ioctl_query_info(&self) -> Result<DeviceInfo> {
        // Direct ioctl to kernel driver
    }
    
    pub fn dma_transfer(&self, buffer: &[u8], direction: DmaDirection) -> Result<()> {
        // DMA operations via kernel driver
    }
}
```

**What We Need to Implement**:
1. **Device I/O Layer**:
   - `/dev/akida*` file operations
   - ioctl commands (need to reverse-engineer)
   - DMA setup and execution
   - Memory mapping for SRAM access

2. **Model Format Parser**:
   - Parse `.fbz` model files (Akida format)
   - Load weights, layer configs to device
   - Handle quantized/sparse models

3. **NPU Management**:
   - Distribute workload across 80 NPUs
   - Synchronization between NPUs
   - Power/thermal management

4. **Inference Engine**:
   - Input preprocessing
   - Trigger NPU execution
   - Output collection
   - Error handling

**Effort**: 3-6 months
**Risk**: High (needs hardware docs)

---

### Option 3: Hybrid Approach (Recommended for Production)

**Phase 1** (Immediate): Rust FFI to C++ Akida Engine
- Get production-ready quickly
- Full feature parity
- Vendor support

**Phase 2** (3-6 months): Pure Rust driver development
- Reverse-engineer protocol
- Implement core features
- Comprehensive testing

**Phase 3** (6-12 months): Pure Rust production migration
- Feature parity validation
- Performance benchmarking
- Gradual rollout

**Benefit**: De-risk pure Rust migration while delivering value immediately

---

## Technical Deep-Dive: Pure Rust Driver

### Kernel Driver Analysis

The C driver (`akida_pcie`) provides:

```c
// Key structures (from akida-pcie-core.c)
struct akida_device {
    struct pci_dev *pdev;
    void __iomem *bar0;  // Memory-mapped registers
    struct dw_edma_chip dma_chip;  // DMA engine
    // ...
};
```

**Device file operations**:
```c
static const struct file_operations akida_fops = {
    .owner = THIS_MODULE,
    .open = akida_open,
    .release = akida_release,
    .read = akida_read,
    .write = akida_write,
    .unlocked_ioctl = akida_ioctl,
    .mmap = akida_mmap,
};
```

**What we need from Rust**:
1. **Open device**: `std::fs::OpenOptions` on `/dev/akida0`
2. **ioctl commands**: Discover command codes (likely in driver header)
3. **DMA transfers**: Use `mmap()` or `read()`/`write()` with large buffers
4. **Register access**: mmap BAR0 for direct register I/O

### Reverse Engineering Strategy

**Step 1: Capture Python SDK traffic**
```bash
# Use strace to see what Python SDK does
strace -e trace=ioctl,read,write,mmap python3 << EOF
import akida
device = akida.devices()[0]
# ... operations ...
EOF
```

**Step 2: Analyze kernel driver source**
```bash
cd /home/strandgate/Development/ecoPrimals/akida_dw_edma
grep -r "IOCTL\|ioctl" .
grep -r "struct akida" .
```

**Step 3: Create ioctl bindings**
```rust
// Based on driver source analysis
const AKIDA_IOC_MAGIC: u8 = b'A';
const AKIDA_IOC_QUERY_INFO: u64 = ioctl!(read, AKIDA_IOC_MAGIC, 1);
const AKIDA_IOC_LOAD_MODEL: u64 = ioctl!(write, AKIDA_IOC_MAGIC, 2);
```

**Step 4: Implement safe Rust wrapper**

### Memory Layout

**Akida AKD1000 Memory**:
- **10 MB On-chip SRAM**: Stores model weights, activations
- **80 NPUs**: Each with local memory, execute in parallel
- **PCIe BAR0**: Memory-mapped registers

**DMA Architecture**:
```
┌─────────────┐        ┌──────────────┐        ┌────────────┐
│   Host RAM  │ ◄─DMA─►│ PCIe Driver  │ ◄─────►│ Akida SRAM │
└─────────────┘        └──────────────┘        └────────────┘
     (Rust)              (akida_pcie)           (Hardware)
```

**Rust Implementation**:
```rust
pub struct DmaBuffer {
    ptr: NonNull<u8>,
    size: usize,
    dma_addr: u64,  // Physical address for DMA
}

unsafe impl Send for DmaBuffer {}
unsafe impl Sync for DmaBuffer {}

impl DmaBuffer {
    pub fn allocate(size: usize) -> Result<Self> {
        // Allocate contiguous physical memory
        // Via mmap with MAP_LOCKED | MAP_POPULATE
    }
    
    pub fn copy_to_device(&self, fd: RawFd, device_offset: u64) -> Result<()> {
        // Trigger DMA transfer via ioctl
    }
}
```

---

## Model Format Reverse Engineering

**Akida Model File** (`.fbz` format):
- Likely FlatBuffers-based (`.fb` extension)
- Compressed with zlib (`.z` suffix)

**Investigation Steps**:
1. Extract sample model from Python SDK:
   ```python
   import akida
   model = akida.Model()  # Load trained model
   model.save("test_model.fbz")
   ```

2. Decompress and analyze:
   ```bash
   # Try decompression
   zlib-flate -uncompress < test_model.fbz > test_model.fb
   
   # Analyze with hexdump
   hexdump -C test_model.fb | less
   
   # Look for FlatBuffers magic bytes
   ```

3. Generate Rust parser:
   ```rust
   // Use flatbuffers crate
   // Or manual binary parser if needed
   ```

---

## Implementation Roadmap

### Phase 1: Foundation (Week 1-2)

**Goal**: Basic device I/O and information querying

**Tasks**:
1. Create `akida-driver` crate structure
2. Implement device file open/close
3. Discover ioctl command codes via strace
4. Query device info (NPU count, memory, PCIe config)
5. Read device temperature, power consumption

**Deliverable**: Can open `/dev/akida0` and read device info

```bash
cargo run --example query_device
# Output:
# Device 0: Akida AKD1000
# NPUs: 80
# Memory: 10 MB SRAM
# Temperature: 42.5°C
# Power: 1.2W
```

### Phase 2: DMA Operations (Week 3-4)

**Goal**: Transfer data to/from device

**Tasks**:
1. Implement DMA buffer allocation
2. Create ioctl bindings for DMA operations
3. Test host-to-device transfers
4. Test device-to-host transfers
5. Benchmark DMA throughput

**Deliverable**: Can transfer 1MB test data at full PCIe speed

### Phase 3: Model Loading (Week 5-8)

**Goal**: Load SNN models to device

**Tasks**:
1. Reverse-engineer `.fbz` model format
2. Create model parser (decompress + parse)
3. Extract layer configs, weights, connections
4. Transfer model data to device SRAM
5. Verify model loaded correctly

**Deliverable**: Load pre-trained MNIST model from disk

### Phase 4: Inference Execution (Week 9-12)

**Goal**: Run inference on NPUs

**Tasks**:
1. Implement input preprocessing
2. Trigger NPU execution via ioctl
3. Collect output from device
4. Handle multi-NPU parallelism
5. Benchmark inference latency/throughput

**Deliverable**: MNIST digit classification running fully in Rust

### Phase 5: Advanced Features (Week 13-16)

**Goal**: Production-ready features

**Tasks**:
1. Multi-board support (distribute across both cards)
2. Batch inference optimization
3. Power management integration
4. Error recovery and health monitoring
5. Comprehensive testing suite

**Deliverable**: Production-quality Akida Rust SDK

---

## Integration with toadStool

### Current Mock Code Locations

Replace these mocked functions with real implementations:

1. **`showcase/neuromorphic/01-akida-detection/src/akida_device.rs`**:
   ```rust
   // Line 11: "In production, this would use the Akida SDK to query actual board state"
   pub fn query_board_info(device: &PcieDevice, index: usize) -> Result<AkidaBoard> {
       // TODO: Replace with real Rust driver calls
   }
   ```

2. **`showcase/neuromorphic/02-akida-bioinformatics/src/akida_filter.rs`**:
   ```rust
   // Line 34: "In production, this would..."
   pub fn load_model(&mut self, model_path: &str) -> Result<()> {
       // TODO: Replace with real model loading
   }
   
   // Line 100: "In production, this would submit k-mers to Akida NPUs"
   fn process_on_board(&self, board_idx: usize, kmers: &[Vec<u8>]) -> Result<usize> {
       // TODO: Replace with real inference
   }
   ```

3. **`showcase/neuromorphic/03-akida-llm-intent/src/akida_classifier.rs`**:
   ```rust
   // Line 25: "TODO: Replace with real Akida SDK call"
   pub fn classify(&self, text: &str) -> Result<Intent> {
       // TODO: Replace with real classification
   }
   ```

### New Crate Structure

```
crates/neuromorphic/
├── akida-driver/           # Pure Rust driver (new)
│   ├── src/
│   │   ├── lib.rs
│   │   ├── device.rs       # Device I/O
│   │   ├── ioctl.rs        # ioctl bindings
│   │   ├── dma.rs          # DMA operations
│   │   ├── model.rs        # Model loading
│   │   ├── inference.rs    # Inference execution
│   │   └── error.rs
│   └── Cargo.toml
│
├── akida-models/           # Model format parsing (new)
│   ├── src/
│   │   ├── lib.rs
│   │   ├── fbz.rs          # .fbz format
│   │   ├── layer.rs        # Layer definitions
│   │   └── quantization.rs
│   └── Cargo.toml
│
└── akida-runtime/          # High-level API (refactor existing)
    ├── src/
    │   ├── lib.rs
    │   ├── device_manager.rs
    │   ├── model_loader.rs
    │   └── inference_pool.rs
    └── Cargo.toml
```

### Example Usage

```rust
use akida_driver::{AkidaDevice, Model, Tensor};

#[tokio::main]
async fn main() -> Result<()> {
    // Open device
    let device = AkidaDevice::open(0)?;
    println!("Opened: {} ({} NPUs)", device.name(), device.npu_count());
    
    // Load model
    let model = Model::from_file("models/kmer_filter.fbz")?;
    device.load_model(&model)?;
    println!("Model loaded: {} layers", model.layer_count());
    
    // Prepare input
    let input = Tensor::from_slice(&[0.1, 0.5, 0.3, ...]);
    
    // Run inference
    let output = device.infer(&input)?;
    println!("Output: {:?}", output.as_slice());
    
    Ok(())
}
```

---

## Dependencies

### Required Rust Crates

```toml
[dependencies]
# Core
libc = "0.2"              # Unix syscalls
nix = "0.27"              # Safe Unix wrappers
thiserror = "2"           # Error handling

# I/O
mmap = "0.2"              # Memory mapping
aligned = "0.4"           # Aligned memory allocation

# Formats
flatbuffers = "23"        # If models use FlatBuffers
flate2 = "1.0"            # zlib decompression
zerocopy = "0.8"          # Zero-copy parsing

# Async (optional)
tokio = { version = "1", features = ["full"] }
```

### Development Dependencies

```toml
[dev-dependencies]
criterion = "0.5"         # Benchmarking
proptest = "1.4"          # Property testing
```

---

## Testing Strategy

### Unit Tests
- Device open/close
- ioctl command encoding/decoding
- DMA buffer allocation
- Model parsing

### Integration Tests
- Full inference pipeline
- Multi-board orchestration
- Error recovery
- Power management

### Benchmark Tests
- Inference latency
- Throughput (inferences/sec)
- DMA bandwidth
- Power efficiency (inferences/joule)

### Hardware-in-Loop Tests
```bash
# Run on actual hardware
cargo test --features hardware-tests

# Benchmark suite
cargo bench --bench akida_inference
```

---

## Risk Mitigation

### Technical Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Unknown ioctl commands | High | Strace Python SDK, analyze driver source |
| Complex model format | High | Start with simple models, incremental parsing |
| DMA stability | Medium | Extensive testing, error recovery |
| Hardware bugs | Medium | Work with BrainChip support |
| Performance regression | Low | Benchmark against Python SDK |

### Business Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Development time | Medium | Hybrid approach (FFI first) |
| Maintenance burden | Medium | Comprehensive tests, documentation |
| BrainChip cooperation | Low | Open source, community-driven |

---

## Success Criteria

### Minimum Viable Product (MVP)
- ✅ Open device files
- ✅ Query device information
- ✅ Load simple SNN model
- ✅ Run inference on single input
- ✅ Achieve 90% of Python SDK performance

### Production Ready
- ✅ Multi-board support
- ✅ Batch inference
- ✅ Error recovery
- ✅ 100% of Python SDK features
- ✅ Performance parity or better
- ✅ Comprehensive documentation

### Ecosystem Contribution
- ✅ Publish crates to crates.io
- ✅ Documentation and examples
- ✅ Community adoption (>100 downloads/month)
- ✅ BrainChip acknowledgment

---

## Timeline Summary

| Phase | Duration | Deliverable |
|-------|----------|-------------|
| Foundation | 2 weeks | Device I/O working |
| DMA Operations | 2 weeks | Data transfer working |
| Model Loading | 4 weeks | Load models to device |
| Inference | 4 weeks | Run inference |
| Production | 4 weeks | Multi-board, error handling |
| **Total** | **16 weeks** | **Production-ready pure Rust SDK** |

**Fast Track** (FFI to C++): 2-3 weeks to production

---

## Immediate Next Steps

1. **Week 1**: Analyze Python SDK behavior with strace
2. **Week 1**: Study kernel driver source code
3. **Week 2**: Create `akida-driver` crate skeleton
4. **Week 2**: Implement device open/query
5. **Week 3**: Present proof-of-concept to team

---

## Resources

### Documentation
- BrainChip Akida SDK: https://doc.brainchipinc.com/
- Kernel driver source: `/home/strandgate/Development/ecoPrimals/akida_dw_edma/`
- Python SDK: `~/miniconda3/envs/akida_env/lib/python3.11/site-packages/akida/`

### Hardware
- 2x Akida AKD1000 boards installed and operational
- Device files: `/dev/akida0`, `/dev/akida1`
- Driver loaded: `lsmod | grep akida_pcie`

### Existing Code
- Detection: `phase1/toadStool/showcase/neuromorphic/01-akida-detection/`
- Bioinformatics: `phase1/toadStool/showcase/neuromorphic/02-akida-bioinformatics/`
- LLM Intent: `phase1/toadStool/showcase/neuromorphic/03-akida-llm-intent/`

---

## Conclusion

**Recommendation**: Start with **Hybrid Approach**
1. **Immediate** (This week): FFI bindings to C++ Akida Engine for production use
2. **Parallel** (Next 3 months): Pure Rust driver development
3. **Migration** (3-6 months): Gradual transition to pure Rust

**Why**:
- ✅ Delivers value immediately with FFI
- ✅ De-risks pure Rust migration
- ✅ Maintains toadStool's "pure Rust" philosophy long-term
- ✅ Contributes to Rust ecosystem

**Let's build the future of neuromorphic computing—in Rust.**

---

**Document Version**: 1.0  
**Date**: January 29, 2026  
**Author**: ToadStool Team  
**Status**: Ready for implementation
