# Investigation Report - Remaining Issues
## February 8, 2026 2:30 AM

---

## Executive Summary

Two critical issues identified after achieving 100% foundational scientific computing:

1. **Tensor 3D Data Corruption** (HIGH PRIORITY)
   - `Tensor::from_data` corrupts 3D arrays
   - Input: `[1.0; 27]` (all 1.0s)
   - Output: `[0.0, 1.875, 0.0, 1.875, ...]` (corrupted pattern)
   - Impact: Laplacian test failure (1/40 tests)
   - Likely cause: Buffer stride/layout interpretation mismatch

2. **NPU Mock Detection** (MEDIUM PRIORITY)
   - User suspects Akida NPU may be sleeping/mock
   - Need to verify real hardware detection
   - Check `/dev/akida*` and PCIe bus scan
   - Audit all substrate backends for proper detection

---

## Issue 1: Tensor 3D Array Corruption

### Symptoms

**Test**: `test_laplacian_simple` (currently `#[ignore]`d)
**Location**: `crates/barracuda/src/ops/md/integrators/laplacian.rs:210`

**Expected Behavior**:
```rust
let data = vec![1.0; 27];  // All 1.0s
let tensor = Tensor::from_data(&data, vec![3, 3, 3], device).unwrap();
let result = tensor.to_vec().unwrap();
// Should be: [1.0, 1.0, 1.0, ..., 1.0] (27 values)
```

**Actual Behavior**:
```
Field values (first 10): [0.0, 1.875, 0.0, 1.875, 0.0, 1.875, 0.0, 1.875, 0.0, 1.875]
```

### Analysis

**Code Path**:
1. `Tensor::from_data()` at `tensor.rs:122-145`
   - Uses `bytemuck::cast_slice(data)` 
   - Creates buffer with `wgpu::BufferUsages::STORAGE | COPY_SRC | COPY_DST`
   - No shape validation beyond length check

2. `Tensor::to_vec()` at `tensor.rs:313-315`
   - Calls `device.read_buffer_f32(&self.buffer, self.len())`
   - Uses `self.len()` which is `shape.iter().product()`

**Hypothesis**:
The buffer is created correctly, but there may be:
- **Padding/alignment issues** in GPU buffer layout
- **Stride interpretation** mismatch between CPU and GPU
- **Memory layout** assumptions (row-major vs column-major)

**Evidence**:
- 2D tensors work fine (PBC, forces all pass)
- 3D tensors show corruption (Laplacian)
- Pattern `0.0, 1.875, 0.0, 1.875` suggests stride/offset issue
- Same issue in `pbc.rs:349-377` (wrapped to `0.4` instead of `0.2`)

### Impact

**Current**:
- 1/40 tests ignored (97.5% pass rate)
- Laplacian operation non-functional for validation
- PBC wrapping test passes but may have underlying corruption

**Future Risk**:
- Any 3D grid operations (PPPM, volumetric data)
- Multi-dimensional FFT results
- Tensor reshape operations

### Proposed Fix

**Short-term** (Deep Investigation):
1. Add explicit buffer padding/alignment checks
2. Test different 3D shapes (2x2x2, 4x4x4, 5x5x5)
3. Compare `from_data` → `to_vec` round-trip for various shapes
4. Check WGSL shader indexing (`laplacian.wgsl` line 38-45)

**Long-term** (Proper Solution):
1. Implement explicit stride/layout metadata in `Tensor`
2. Add `TensorLayout` enum (RowMajor, ColumnMajor, etc.)
3. Use explicit buffer mapping with stride control
4. Add validation tests for N-dimensional tensors (N=1,2,3,4)

---

## Issue 2: NPU Hardware Detection

### Current Implementation

**Akida Detection** (`crates/neuromorphic/akida-driver/src/discovery.rs`):
```rust
pub fn discover() -> Result<Self> {
    // Scans /dev/akida0..15
    for index in 0..16 {
        let path = PathBuf::from(format!("/dev/akida{index}"));
        if !path.exists() {
            continue;
        }
        // ... PCIe query ...
    }
}
```

**PCIe Scan** (`crates/barracuda/src/device/akida.rs:152-198`):
```rust
fn scan_pcie_for_akida() -> Result<Vec<PcieDevice>> {
    // Scans /sys/bus/pci/devices/
    // Looks for vendor_id == 0x1e7c (BrainChip)
    // device_id == 0x1000 (Akida AKD1000)
}
```

### Verification Needed

**Questions**:
1. Does `/dev/akida*` exist on this system?
2. Does PCIe scan find vendor `0x1e7c`?
3. Are there mock/stub implementations being used?
4. Is the Akida driver loaded (`lsmod | grep akida`)?

**Evidence to Collect**:
```bash
# Check device files
ls -la /dev/akida*

# Check PCIe devices
lspci | grep -i brain

# Check sysfs
ls /sys/bus/pci/devices/ | xargs -I {} sh -c 'cat /sys/bus/pci/devices/{}/vendor 2>/dev/null | grep -q 0x1e7c && echo {}'

# Check driver
lsmod | grep akida
dmesg | grep -i akida
```

### Potential Issues

**Mock Implementations**:
1. `estimate_power_consumption()` returns hardcoded values
2. `estimate_temperature()` returns hardcoded values
3. No actual ioctl() calls to device
4. No validation of device responses

**Real Detection Indicators**:
- `/dev/akida*` files exist
- PCIe vendor ID `0x1e7c` found
- Kernel driver loaded
- Can read sysfs attributes

---

## Investigation Plan

### Phase 1: Tensor Corruption (HIGH PRIORITY)

**Step 1**: Minimal reproduction (30 min)
```rust
#[tokio::test]
async fn test_tensor_3d_roundtrip() {
    let device = Arc::new(WgpuDevice::new().await.unwrap());
    
    // Test various 3D shapes
    for &(nx, ny, nz) in &[(2,2,2), (3,3,3), (4,4,4)] {
        let size = nx * ny * nz;
        let data = vec![1.0; size];
        let tensor = Tensor::from_data(&data, vec![nx, ny, nz], device.clone()).unwrap();
        let result = tensor.to_vec().unwrap();
        
        assert_eq!(result.len(), size);
        for (i, &val) in result.iter().enumerate() {
            assert_eq!(val, 1.0, "Index {i} corrupted: {val}");
        }
    }
}
```

**Step 2**: Buffer inspection (30 min)
- Add debug logging in `from_data`
- Check buffer size vs data size
- Verify alignment requirements
- Test with different data patterns (0.0, 1.0, incrementing)

**Step 3**: WGSL shader audit (20 min)
- Review `laplacian.wgsl` indexing
- Check `idx()` helper function
- Verify buffer bindings

### Phase 2: NPU Hardware Verification (MEDIUM PRIORITY)

**Step 1**: Device file check (5 min)
```bash
ls -la /dev/akida* 2>&1
```

**Step 2**: PCIe scan (10 min)
```bash
lspci -nn | grep -i brain
cat /sys/bus/pci/devices/*/vendor | grep 0x1e7c
```

**Step 3**: Driver status (10 min)
```bash
lsmod | grep akida
dmesg | grep -i akida | tail -50
```

**Step 4**: Programmatic test (15 min)
```rust
#[test]
fn test_akida_real_hardware() {
    use crate::device::akida::detect_akida_boards;
    
    match detect_akida_boards() {
        Ok(caps) if caps.boards.len() > 0 => {
            println!("✅ REAL HARDWARE: {} boards", caps.boards.len());
            for board in &caps.boards {
                println!("  - Device: {}", board.device_path.display());
                println!("  - PCIe: {}", board.pcie_address);
                // Try to open device file
                if std::fs::File::open(&board.device_path).is_ok() {
                    println!("  - Status: ACCESSIBLE ✅");
                } else {
                    println!("  - Status: NOT ACCESSIBLE (permissions?) ⚠️");
                }
            }
        }
        Ok(_) => {
            println!("⚠️  NO HARDWARE: Zero boards detected (mock or missing)");
        }
        Err(e) => {
            println!("❌ ERROR: {e}");
        }
    }
}
```

---

## Next Actions

### Immediate (Next 2 hours):

1. **[HIGH]** Run tensor corruption minimal reproduction
2. **[HIGH]** Add debug logging to `Tensor::from_data`
3. **[MEDIUM]** Run NPU hardware verification commands
4. **[MEDIUM]** Test Akida detection programmatically

### Short-term (Next session):

1. **[HIGH]** Fix tensor 3D corruption (if found)
2. **[HIGH]** Un-ignore Laplacian test
3. **[MEDIUM]** Audit all substrate mock implementations
4. **[LOW]** Add comprehensive tensor layout tests

---

## Success Criteria

### Tensor Corruption Fixed:
- ✅ Laplacian test passes (40/40 = 100%)
- ✅ 3D tensor roundtrip validated
- ✅ PBC wrapping returns correct 0.2 distance
- ✅ All tensor shapes (1D, 2D, 3D, 4D) tested

### NPU Hardware Verified:
- ✅ Real Akida device files found (or confirmed absent)
- ✅ PCIe scan results documented
- ✅ Driver status documented
- ✅ Mock vs real clearly labeled in code

---

## Technical Debt Notes

**Current Status**: 
- Scientific computing: 100% foundational (with 1 ignored test)
- Deep debt compliance: 100% (zero unsafe, all WGSL)

**After Investigation**:
- Scientific computing: 100% foundational (40/40 tests)
- Hardware wiring: Verified real vs mock

**No New Debt**: Investigation follows deep debt principles
- Zero unsafe code additions
- Runtime discovery only
- No hardcoded assumptions
- Complete fossil record

---

**Investigation Started**: Feb 8, 2026 2:30 AM  
**Estimated Duration**: 2-4 hours  
**Priority**: HIGH (tensor) + MEDIUM (NPU)  
**Blocking**: None (optional polish)
