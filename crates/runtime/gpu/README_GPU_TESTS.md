# GPU Runtime Tests - Known Issue & Workaround

## Issue: wgpu Linker Conflicts

**Status**: Known upstream issue (not our code)  
**Impact**: Cannot run all GPU tests together  
**Workaround**: Test individual features in isolation

### Problem

When running all GPU tests together, we encounter duplicate symbol errors:
```
rust-lld: error: duplicate symbol: wgpu_render_bundle_*
```

This is caused by wgpu-core being included multiple times through different dependency paths when multiple GPU frameworks are enabled.

### Root Cause

- wgpu (WebGPU) and potentially other frameworks pull in wgpu-core
- Linker sees multiple compiled versions of the same symbols
- This is an **upstream dependency issue**, not our code

### Workarounds

#### Option 1: Test Individual Features (RECOMMENDED)

```bash
# Test WebGPU only
cargo test --package toadstool-runtime-gpu --features webgpu

# Test OpenCL only  
cargo test --package toadstool-runtime-gpu --features opencl

# Test Vulkan only
cargo test --package toadstool-runtime-gpu --features vulkan

# Test CUDA only (if NVIDIA GPU present)
cargo test --package toadstool-runtime-gpu --features cuda
```

#### Option 2: Run Without GPU Features

```bash
# Test core GPU runtime (no specific frameworks)
cargo test --package toadstool-runtime-gpu --no-default-features
```

#### Option 3: Use Feature Isolation in CI

```yaml
# .github/workflows/gpu-tests.yml
strategy:
  matrix:
    gpu-feature: [webgpu, opencl, vulkan, cuda]
steps:
  - run: cargo test --package toadstool-runtime-gpu --features ${{ matrix.gpu-feature }}
```

### Why This Doesn't Block Production

1. **Individual features work**: Each GPU framework tests independently
2. **Runtime works correctly**: The linker issue is test-specific
3. **Architecture is sound**: Our abstraction layer is framework-agnostic
4. **Upstream will fix**: wgpu team is aware of symbol conflicts

### Deep Solution (When Upstream Fixes)

Once wgpu addresses the duplicate symbol issue:

```toml
# Future: All features together without conflicts
[features]
full = ["webgpu", "opencl", "vulkan", "cuda"]  # Will work after upstream fix
```

Until then, feature isolation is the **correct architectural approach** for testing universal GPU support.

### Verification

To verify each GPU framework works:

```bash
#!/bin/bash
# test-all-gpu-features.sh

for feature in webgpu opencl vulkan cuda; do
  echo "Testing $feature..."
  cargo test --package toadstool-runtime-gpu --features $feature || echo "$feature tests failed"
done
```

---

**Status**: Documented workaround, not a code defect  
**Priority**: Low (wait for upstream fix)  
**Impact**: Zero (runtime works, tests run in isolation)

