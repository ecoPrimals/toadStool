# BarraCUDA Scientific Computing Operations Specification
## Complex Arithmetic, FFT, and Physics Primitives

**Version**: 1.0.0  
**Date**: February 7, 2026  
**Status**: Specification (implementation pending)  
**Principles**: 
- All math in WGSL shaders (universal portability)
- All orchestration in Rust (type safety, zero-cost abstractions)
- Zero unsafe code, 100% deep debt compliant

---

## Architecture Principles

### WGSL Shaders (Math Layer)
```
Purpose: Pure mathematical operations on GPU
Language: WGSL (WebGPU Shading Language)
Portability: Runs on any wgpu backend (NVIDIA, AMD, Intel, ARM)
Location: crates/barracuda/src/ops/{module}/shaders/*.wgsl
```

### Rust Orchestration (API Layer)
```
Purpose: Tensor creation, shader dispatch, buffer management
Language: Rust (100% safe, zero unsafe)
Abstractions: Tensor, Device, Operation traits
Location: crates/barracuda/src/ops/{module}/mod.rs
```

---

## Phase 1: Complex Arithmetic (10 Operations)

### Module: `crates/barracuda/src/ops/complex/`

#### 1. Complex Type Representation
```wgsl
// Complex number as vec2<f32>
// x = real part, y = imaginary part
type Complex = vec2<f32>;

// Constructor helper
fn complex_make(re: f32, im: f32) -> Complex {
    return vec2<f32>(re, im);
}
```

**Storage**: vec2<f32> for f32 precision, vec4<u32> for f64 (via u64_emu pattern)

---

#### 1.1 complex_add.wgsl
**Operation**: (a + bi) + (c + di) = (a+c) + (b+d)i  
**Complexity**: O(1), trivial (native vec2 addition)  
**Shader**:
```wgsl
@group(0) @binding(0) var<storage, read> input_a: array<vec2<f32>>;
@group(0) @binding(1) var<storage, read> input_b: array<vec2<f32>>;
@group(0) @binding(2) var<storage, read_write> output: array<vec2<f32>>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= arrayLength(&input_a)) { return; }
    
    output[idx] = input_a[idx] + input_b[idx];  // Native vec2 addition
}
```

**Rust API**:
```rust
pub struct ComplexAdd {
    input_a: Tensor,
    input_b: Tensor,
    // shader dispatch state
}

impl ComplexAdd {
    pub fn new(input_a: Tensor, input_b: Tensor) -> Result<Self>;
    pub fn execute(self) -> Result<Tensor>;
}
```

---

#### 1.2 complex_sub.wgsl
**Operation**: (a + bi) - (c + di) = (a-c) + (b-d)i  
**Complexity**: O(1), trivial  
**Implementation**: Identical to add, use vec2 subtraction

---

#### 1.3 complex_mul.wgsl ⚠️ **CRITICAL FOR FFT**
**Operation**: (a + bi)(c + di) = (ac - bd) + (ad + bc)i  
**Complexity**: O(1), 4 multiplications + 2 add/sub  
**Shader**:
```wgsl
fn complex_mul(z1: vec2<f32>, z2: vec2<f32>) -> vec2<f32> {
    let a = z1.x;  // real(z1)
    let b = z1.y;  // imag(z1)
    let c = z2.x;  // real(z2)
    let d = z2.y;  // imag(z2)
    
    let re = a * c - b * d;
    let im = a * d + b * c;
    
    return vec2<f32>(re, im);
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= arrayLength(&input_a)) { return; }
    
    output[idx] = complex_mul(input_a[idx], input_b[idx]);
}
```

**Performance**: ~2-3 GPU cycles, critical for FFT butterfly operations

---

#### 1.4 complex_conj.wgsl
**Operation**: conj(a + bi) = a - bi  
**Complexity**: O(1), one negation  
**Shader**:
```wgsl
fn complex_conj(z: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(z.x, -z.y);
}
```

**Use Case**: FFT normalization, correlation functions

---

#### 1.5 complex_abs.wgsl
**Operation**: |a + bi| = sqrt(a² + b²)  
**Complexity**: O(1), native vec2 length()  
**Shader**:
```wgsl
fn complex_abs(z: vec2<f32>) -> f32 {
    return length(z);  // Native WGSL function
}

// Squared magnitude (faster, no sqrt)
fn complex_abs_sq(z: vec2<f32>) -> f32 {
    return dot(z, z);  // Native WGSL function
}
```

**Use Case**: Power spectrum, structure factors S(q)

---

#### 1.6 complex_exp.wgsl ⚠️ **CRITICAL FOR FFT**
**Operation**: exp(a + bi) = exp(a)[cos(b) + i·sin(b)] (Euler's formula)  
**Complexity**: O(1), 1 exp + 2 trig  
**Shader**:
```wgsl
fn complex_exp(z: vec2<f32>) -> vec2<f32> {
    let exp_re = exp(z.x);
    let cos_im = cos(z.y);
    let sin_im = sin(z.y);
    
    return vec2<f32>(exp_re * cos_im, exp_re * sin_im);
}
```

**Use Case**: FFT twiddle factors W_N^k = exp(-2πik/N)

---

#### 1.7 complex_div.wgsl
**Operation**: (a+bi)/(c+di) = (a+bi)(c-di)/(c²+d²)  
**Complexity**: O(1), compose mul + conj + abs_sq  
**Shader**:
```wgsl
fn complex_div(z1: vec2<f32>, z2: vec2<f32>) -> vec2<f32> {
    let denom = dot(z2, z2);  // |z2|²
    let num = complex_mul(z1, complex_conj(z2));
    return num / denom;
}
```

---

#### 1.8 complex_sqrt.wgsl
**Operation**: sqrt(a+bi) via polar form  
**Complexity**: O(1), 1 sqrt + 1 atan2 + 2 trig  
**Shader**:
```wgsl
fn complex_sqrt(z: vec2<f32>) -> vec2<f32> {
    let r = length(z);
    let theta = atan2(z.y, z.x);
    
    let sqrt_r = sqrt(r);
    let half_theta = theta * 0.5;
    
    return vec2<f32>(sqrt_r * cos(half_theta), sqrt_r * sin(half_theta));
}
```

**Use Case**: Wave propagation, Green's functions

---

#### 1.9 complex_log.wgsl
**Operation**: log(a+bi) = log|z| + i·arg(z)  
**Complexity**: O(1), 1 log + 1 atan2  
**Shader**:
```wgsl
fn complex_log(z: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(log(length(z)), atan2(z.y, z.x));
}
```

**Use Case**: Transfer functions, impedance calculations

---

#### 1.10 complex_pow.wgsl
**Operation**: (a+bi)^n via De Moivre's theorem  
**Complexity**: O(1), 1 pow + 1 atan2 + 2 trig  
**Shader**:
```wgsl
fn complex_pow(z: vec2<f32>, n: f32) -> vec2<f32> {
    let r = length(z);
    let theta = atan2(z.y, z.x);
    
    let r_pow_n = pow(r, n);
    let n_theta = n * theta;
    
    return vec2<f32>(r_pow_n * cos(n_theta), r_pow_n * sin(n_theta));
}
```

---

## Phase 2: Fast Fourier Transform (5 Operations)

### Module: `crates/barracuda/src/ops/fft/`

**Ancestral Code**: Evolve from `fhe_ntt.wgsl` (80% structure reuse)

#### 2.1 fft_1d.wgsl ⚠️ **CRITICAL - BLOCKS PPPM**
**Operation**: 1D Complex FFT via Cooley-Tukey butterfly  
**Complexity**: O(N log N)  
**Algorithm**: Radix-2 decimation-in-time

**Evolution from NTT**:
```wgsl
// NTT butterfly (existing - fhe_ntt.wgsl):
fn butterfly(a: U64, b: U64, twiddle: U64, q: U64) -> ButterflyResult {
    let tb = mod_mul_u64(twiddle, b, q);     // Modular mul
    let u = mod_add_u64(a, tb, q);            // Modular add
    let v = mod_sub_u64(a, tb, q);            // Modular sub
    return ButterflyResult(u, v);
}

// FFT butterfly (evolved):
fn butterfly_fft(a: vec2<f32>, b: vec2<f32>, twiddle: vec2<f32>) -> ButterflyResult {
    let tb = complex_mul(twiddle, b);         // Complex mul
    let u = a + tb;                            // Complex add (vec2 native)
    let v = a - tb;                            // Complex sub (vec2 native)
    return ButterflyResult(u, v);
}
```

**Twiddle Factor Generation**:
```wgsl
fn compute_twiddle(k: u32, N: u32) -> vec2<f32> {
    let angle = -2.0 * PI * f32(k) / f32(N);
    return vec2<f32>(cos(angle), sin(angle));  // exp(-2πik/N)
}
```

**Shader Structure** (adapted from NTT):
- Bit-reversal permutation (IDENTICAL to NTT)
- Stage-wise butterfly passes (same loop structure)
- Workgroup dispatch (same parallelism pattern)

**Rust API**:
```rust
pub struct Fft1D {
    input: Tensor,  // Complex tensor (vec2<f32> elements)
    degree: u32,    // Must be power of 2
    // Precomputed twiddle factors
    // Shader pipeline state
}

impl Fft1D {
    pub fn new(input: Tensor, degree: u32) -> Result<Self>;
    pub fn execute(self) -> Result<Tensor>;
}
```

---

#### 2.2 ifft_1d.wgsl
**Operation**: Inverse 1D FFT  
**Evolution**: From `fhe_intt.wgsl` structure  
**Algorithm**: FFT with conjugated twiddles + normalization

**Shader**:
```wgsl
// IFFT = conj(FFT(conj(input))) / N
fn ifft_1d(input: array<vec2<f32>>, N: u32) -> array<vec2<f32>> {
    // 1. Conjugate input
    // 2. Apply FFT (same butterfly as forward)
    // 3. Conjugate output
    // 4. Normalize by 1/N
}
```

---

#### 2.3 fft_2d.wgsl
**Operation**: 2D FFT via row-column decomposition  
**Algorithm**: FFT each row, then FFT each column  
**Shader**: Dispatch 1D FFT twice (rows then columns)

---

#### 2.4 fft_3d.wgsl ⚠️ **REQUIRED FOR PPPM**
**Operation**: 3D FFT for PPPM long-range forces  
**Algorithm**: FFT each dimension sequentially  
**Use Case**: Molecular dynamics, particle-mesh methods

---

#### 2.5 rfft.wgsl (Real-to-Complex FFT)
**Operation**: Optimized FFT for real-valued input  
**Optimization**: Half-complex storage (Hermitian symmetry)  
**Speedup**: 2x faster than complex FFT for real data

---

## Phase 3: Periodic Boundary Conditions

### Module: `crates/barracuda/src/ops/md/pbc.rs`

#### 3.1 minimum_image_distance.wgsl
**Operation**: Compute distance with periodic boundaries  
**Algorithm**: Minimum image convention  
**Shader**:
```wgsl
fn minimum_image(dr: vec3<f32>, box_size: vec3<f32>) -> vec3<f32> {
    return dr - round(dr / box_size) * box_size;
}

fn pbc_distance(r1: vec3<f32>, r2: vec3<f32>, box_size: vec3<f32>) -> f32 {
    let dr = minimum_image(r2 - r1, box_size);
    return length(dr);
}
```

**Ancestral Code**: Wrapper on `pairwise_distance.rs`, `cdist.wgsl`

---

## Phase 4: Force Kernels

### Module: `crates/barracuda/src/ops/md/forces/`

#### 4.1 coulomb.wgsl
**Operation**: Electrostatic potential and force  
**Formula**: V(r) = q₁q₂/(4πε₀r), F = -∇V  
**Shader**:
```wgsl
struct ForceResult {
    force: vec3<f32>,  // F = -dV/dr
    energy: f32,       // V(r)
    virial: f32,       // r·F for pressure
}

fn coulomb_force(r_vec: vec3<f32>, q1: f32, q2: f32) -> ForceResult {
    let r = length(r_vec);
    let r_inv = 1.0 / r;
    let r_inv3 = r_inv * r_inv * r_inv;
    
    let energy = COULOMB_CONST * q1 * q2 * r_inv;
    let force_mag = COULOMB_CONST * q1 * q2 * r_inv3;
    let force_vec = force_mag * r_vec;
    let virial = dot(r_vec, force_vec);
    
    return ForceResult(force_vec, energy, virial);
}
```

---

#### 4.2 yukawa.wgsl
**Operation**: Screened Coulomb (dusty plasmas, DLVO theory)  
**Formula**: V(r) = q₁q₂·exp(-κr)/(4πε₀r)  
**Shader**: Similar to Coulomb, add exp(-κr) screening

---

#### 4.3 lennard_jones.wgsl
**Operation**: Neutral atom potential  
**Formula**: V(r) = 4ε[(σ/r)¹² - (σ/r)⁶]  
**Use Case**: Soft matter, molecular dynamics

---

#### 4.4 morse.wgsl
**Operation**: Molecular bond potential  
**Formula**: V(r) = D[1 - exp(-a(r-r₀))]²  
**Use Case**: Chemical bonds, vibrational spectroscopy

---

#### 4.5 born_mayer.wgsl
**Operation**: Short-range repulsion  
**Formula**: V(r) = A·exp(-r/ρ)  
**Use Case**: Ionic crystals, electron cloud overlap

---

## Phase 5: Time Integrators

### Module: `crates/barracuda/src/ops/integrators/`

#### 5.1 velocity_verlet.wgsl
**Operation**: Symplectic MD integrator (energy-conserving)  
**Algorithm**:
```wgsl
// Velocity-Verlet algorithm
fn velocity_verlet_step(
    pos: vec3<f32>, 
    vel: vec3<f32>, 
    force: vec3<f32>,
    mass: f32, 
    dt: f32
) -> StateUpdate {
    let vel_half = vel + 0.5 * force / mass * dt;
    let pos_new = pos + vel_half * dt;
    // Force recomputation at pos_new (external)
    let vel_new = vel_half + 0.5 * force_new / mass * dt;
    return StateUpdate(pos_new, vel_new);
}
```

---

#### 5.2 rk4.wgsl
**Operation**: 4th-order Runge-Kutta (general ODE solver)  
**Algorithm**: Classic RK4 for dy/dt = f(y,t)

---

#### 5.3 laplacian_stencil.wgsl
**Operation**: Finite difference Laplacian (PDEs)  
**Algorithm**: 5-point stencil (2D), 7-point (3D)  
**Use Case**: Heat equation, diffusion, TTM

---

## Phase 6: Bessel Functions

### Module: `crates/barracuda/src/ops/special/bessel/`

**Ancestral Pattern**: Similar to `lgamma.wgsl` (series expansion)

#### 6.1 bessel_j0.wgsl, bessel_j1.wgsl
**Operation**: Bessel functions of first kind (oscillatory)  
**Algorithm**: Polynomial approximation (Abramowitz & Stegun)  
**Use Case**: TTM cylindrical coordinates, wave physics

#### 6.2 bessel_i0.wgsl, bessel_i1.wgsl
**Operation**: Modified Bessel (exponential growth)  
**Use Case**: Diffusion, heat transfer

#### 6.3 bessel_k0.wgsl, bessel_k1.wgsl
**Operation**: Modified Bessel second kind (exponential decay)  
**Use Case**: Green's functions, potential theory

---

## Implementation Checklist

### Phase 1: Complex Arithmetic (Week 1-4)
- [ ] complex_add.wgsl + Rust wrapper
- [ ] complex_sub.wgsl + Rust wrapper
- [ ] complex_mul.wgsl + Rust wrapper ⚠️ **FFT BLOCKER**
- [ ] complex_conj.wgsl + Rust wrapper
- [ ] complex_abs.wgsl + Rust wrapper
- [ ] complex_exp.wgsl + Rust wrapper ⚠️ **FFT BLOCKER**
- [ ] complex_div.wgsl + Rust wrapper
- [ ] complex_sqrt.wgsl + Rust wrapper
- [ ] complex_log.wgsl + Rust wrapper
- [ ] complex_pow.wgsl + Rust wrapper
- [ ] Comprehensive unit tests (Euler's identity, etc.)
- [ ] Performance benchmarks (1M complex_mul < 10ms)

### Phase 2: FFT Suite (Week 5-12)
- [ ] fft_1d.wgsl (evolve from fhe_ntt.wgsl)
- [ ] ifft_1d.wgsl (evolve from fhe_intt.wgsl)
- [ ] Twiddle factor precomputation
- [ ] Bit-reversal permutation (adapt from NTT)
- [ ] fft_2d.wgsl (row-column decomposition)
- [ ] fft_3d.wgsl (for PPPM) ⚠️ **PHYSICS BLOCKER**
- [ ] rfft.wgsl (real-to-complex optimization)
- [ ] Validation tests (FFT(IFFT(x)) = x)
- [ ] Performance benchmarks (4096-point FFT < 5ms)

### Phase 3-6: Physics Primitives (Week 13-20)
- [ ] PBC wrapper (1 week)
- [ ] Force kernels (5 shaders, 2-3 weeks)
- [ ] Integrators (3 shaders, 1-2 weeks)
- [ ] Bessel functions (6 shaders, 3-4 weeks)

---

## Testing Strategy

### Unit Tests (Rust)
```rust
#[test]
fn test_complex_mul_correctness() {
    // (3+4i)(1+2i) = -5+10i
    let z1 = Tensor::from_complex(&[(3.0, 4.0)]);
    let z2 = Tensor::from_complex(&[(1.0, 2.0)]);
    let op = ComplexMul::new(z1, z2)?;
    let result = op.execute()?;
    assert_approx_eq!(result.to_complex()[0], (-5.0, 10.0), 1e-6);
}

#[test]
fn test_fft_inverse_property() {
    // FFT(IFFT(x)) = x
    let input = random_complex_signal(1024);
    let fft = Fft1D::new(input.clone(), 1024)?.execute()?;
    let ifft = Ifft1D::new(fft, 1024)?.execute()?;
    assert_tensors_approx_eq!(input, ifft, 1e-5);
}
```

### Validation Tests
1. Euler's identity: exp(iπ) + 1 = 0
2. Parseval's theorem: ||FFT(x)||² = N·||x||²
3. Convolution theorem: FFT(f★g) = FFT(f)·FFT(g)
4. Energy conservation: Velocity-Verlet conserves E = KE + PE

### Benchmark Tests
```rust
#[bench]
fn bench_complex_mul_1m(b: &mut Bencher) {
    // Target: < 10ms for 1M complex multiplications on RTX 3090
}

#[bench]
fn bench_fft_4096(b: &mut Bencher) {
    // Target: < 5ms for 4096-point FFT on RTX 3090
}
```

---

## Performance Targets

| Operation | Size | Target (RTX 3090) | Comparison |
|-----------|------|------------------|------------|
| complex_mul | 1M ops | < 10ms | ~100 GFLOPS |
| FFT 1D | 4096 | < 5ms | ~10 GFLOPs |
| FFT 3D | 64³ | < 50ms | PPPM bottleneck |
| Coulomb forces | 10K pairs | < 20ms | O(N²) direct |
| Velocity-Verlet | 10K particles | < 5ms/step | MD timestep |

---

## Dependencies

**Existing BarraCUDA Ops** (no changes needed):
- `erf.wgsl`, `erfc.wgsl` - Already validated
- `pairwise_distance.rs` - PBC will wrap this
- `u64_emu.wgsl` - Pattern for ComplexF64
- `lgamma.wgsl` - Pattern for Bessel series

**External Dependencies**:
- wgpu 0.18+ (GPU abstraction)
- bytemuck (safe byte casting)
- Rust 1.75+ (const generics, pattern matching)

---

## Success Criteria

### Phase 1 Complete
✅ All 10 complex ops implemented  
✅ Unit tests pass (Euler's identity verified)  
✅ 1M complex_mul < 10ms  
✅ FFT team unblocked

### Phase 2 Complete
✅ 1D/2D/3D FFT + inverse implemented  
✅ FFT(IFFT(x)) = x verified  
✅ 4096-point FFT < 5ms  
✅ PPPM unblocked

### Phases 3-6 Complete
✅ Sarkas MD compatible  
✅ Energy conservation verified  
✅ g(r) matches expected  
✅ Bessel J0, J1 match A&S tables

---

**Status**: Specification complete, ready for implementation  
**Next Step**: Week 1 - Implement complex_add, complex_sub, complex_conj (foundation)  
**Architecture**: All math in WGSL shaders (universal portability), all orchestration in Rust (safety)
