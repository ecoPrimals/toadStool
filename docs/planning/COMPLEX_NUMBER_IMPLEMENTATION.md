# Complex Number Implementation Proposal
## BarraCUDA Scientific Computing: Phase 1.1

**Date**: February 7, 2026  
**Priority**: CRITICAL (blocks all FFT work)  
**Effort**: 3-4 weeks  
**Status**: Design phase

---

## 🎯 Design Goals

1. **Native feel**: Complex should work like f32/f64 in WGSL
2. **Performance**: Minimal overhead vs manual vec2 math
3. **Precision options**: Both f32 and f64 (via u64_emu) variants
4. **Composability**: Works with existing tensor infrastructure

---

## 📐 Type Design

### Option A: vec2<f32> Native (RECOMMENDED)

**Advantages**:
- WGSL native vec2 operations (SIMD-friendly)
- Direct GPU register mapping
- Minimal shader complexity
- Compatible with existing texture formats (RG32F)

**Implementation**:
```wgsl
// Complex as vec2<f32>
// x = real, y = imaginary
type Complex = vec2<f32>;

fn complex_make(re: f32, im: f32) -> Complex {
    return vec2<f32>(re, im);
}

fn complex_real(z: Complex) -> f32 { return z.x; }
fn complex_imag(z: Complex) -> f32 { return z.y; }
```

### Option B: Struct (More Explicit)

**Advantages**:
- Self-documenting (.re, .im)
- Type safety (can't accidentally use as vec2)
- Future extensibility (metadata, precision flags)

**Implementation**:
```wgsl
struct Complex {
    re: f32,
    im: f32,
}

fn complex_make(re: f32, im: f32) -> Complex {
    return Complex(re, im);
}
```

### Option C: ComplexF64 via U64 Emulation

**Advantages**:
- Double precision for physics (energy conservation)
- Follows FHE pattern (u64_emu.wgsl)
- Necessary for high-accuracy FFT

**Implementation**:
```wgsl
struct ComplexF64 {
    re_lo: u32, re_hi: u32,  // Real part (u64 emulated)
    im_lo: u32, im_hi: u32,  // Imaginary part (u64 emulated)
}
```

**Recommendation**: Start with **Option A** (vec2<f32>), add Option C later if physics demands it.

---

## 🔧 Core Operations

### Priority Tier 1 (FFT Blockers)

#### 1. complex_add.wgsl
```wgsl
// (a + bi) + (c + di) = (a+c) + (b+d)i
fn complex_add(z1: Complex, z2: Complex) -> Complex {
    return z1 + z2;  // vec2 addition (native!)
}
```
**Complexity**: Trivial (vec2 native)  
**Performance**: 1 SIMD op

---

#### 2. complex_sub.wgsl
```wgsl
// (a + bi) - (c + di) = (a-c) + (b-d)i
fn complex_sub(z1: Complex, z2: Complex) -> Complex {
    return z1 - z2;  // vec2 subtraction (native!)
}
```
**Complexity**: Trivial  
**Performance**: 1 SIMD op

---

#### 3. complex_mul.wgsl
```wgsl
// (a + bi)(c + di) = (ac - bd) + (ad + bc)i
fn complex_mul(z1: Complex, z2: Complex) -> Complex {
    let a = z1.x;  // real(z1)
    let b = z1.y;  // imag(z1)
    let c = z2.x;  // real(z2)
    let d = z2.y;  // imag(z2)
    
    let re = a * c - b * d;  // Real part
    let im = a * d + b * c;  // Imaginary part
    
    return vec2<f32>(re, im);
}
```
**Complexity**: 6 ops (4 mul, 2 add/sub)  
**Performance**: ~2-3 cycles on modern GPU  
**Critical for**: FFT butterfly (twiddle × input)

---

#### 4. complex_conj.wgsl
```wgsl
// conj(a + bi) = a - bi
fn complex_conj(z: Complex) -> Complex {
    return vec2<f32>(z.x, -z.y);
}
```
**Complexity**: Trivial (1 negation)  
**Performance**: < 1 cycle  
**Critical for**: FFT normalization, correlation

---

#### 5. complex_abs.wgsl
```wgsl
// |a + bi| = sqrt(a² + b²)
fn complex_abs(z: Complex) -> f32 {
    return length(z);  // vec2 native!
}

// Squared magnitude (faster, no sqrt)
fn complex_abs_sq(z: Complex) -> f32 {
    return dot(z, z);  // vec2 native!
}
```
**Complexity**: Trivial (native vec2)  
**Performance**: 1 op (length) or < 1 op (dot)  
**Critical for**: Power spectrum, structure factors

---

#### 6. complex_exp.wgsl
```wgsl
// exp(a + bi) = exp(a) * [cos(b) + i·sin(b)]  (Euler's formula)
fn complex_exp(z: Complex) -> Complex {
    let exp_re = exp(z.x);
    let cos_im = cos(z.y);
    let sin_im = sin(z.y);
    
    return vec2<f32>(exp_re * cos_im, exp_re * sin_im);
}
```
**Complexity**: 3 transcendentals (exp, cos, sin)  
**Performance**: ~10-15 cycles  
**Critical for**: FFT twiddle factors W_N^k = exp(-2πik/N)

---

### Priority Tier 2 (Extended Operations)

#### 7. complex_div.wgsl
```wgsl
// (a + bi) / (c + di) = (a+bi)(c-di) / (c²+d²)
fn complex_div(z1: Complex, z2: Complex) -> Complex {
    let denom = complex_abs_sq(z2);  // c² + d²
    let num = complex_mul(z1, complex_conj(z2));  // (a+bi)(c-di)
    return num / denom;  // vec2 scalar divide
}
```
**Complexity**: 1 mul + 1 conj + 1 dot + 1 div  
**Performance**: ~5 cycles

---

#### 8. complex_sqrt.wgsl
```wgsl
// sqrt(a + bi) via polar form
// z = r·exp(iθ) => sqrt(z) = sqrt(r)·exp(iθ/2)
fn complex_sqrt(z: Complex) -> Complex {
    let r = complex_abs(z);
    let theta = atan2(z.y, z.x);
    
    let sqrt_r = sqrt(r);
    let half_theta = theta * 0.5;
    
    return vec2<f32>(sqrt_r * cos(half_theta), sqrt_r * sin(half_theta));
}
```
**Complexity**: 1 length + 1 atan2 + 1 sqrt + 2 trig  
**Performance**: ~15 cycles

---

#### 9. complex_log.wgsl
```wgsl
// log(a + bi) = log|z| + i·arg(z)
fn complex_log(z: Complex) -> Complex {
    let r = complex_abs(z);
    let theta = atan2(z.y, z.x);
    
    return vec2<f32>(log(r), theta);
}
```
**Complexity**: 1 length + 1 atan2 + 1 log  
**Performance**: ~10 cycles

---

#### 10. complex_pow.wgsl
```wgsl
// (a + bi)^n via De Moivre's theorem
// z^n = r^n · exp(inθ)
fn complex_pow(z: Complex, n: f32) -> Complex {
    let r = complex_abs(z);
    let theta = atan2(z.y, z.x);
    
    let r_pow_n = pow(r, n);
    let n_theta = n * theta;
    
    return vec2<f32>(r_pow_n * cos(n_theta), r_pow_n * sin(n_theta));
}
```
**Complexity**: 1 length + 1 atan2 + 1 pow + 2 trig  
**Performance**: ~15 cycles

---

## 📦 Module Structure

```
crates/barracuda/src/ops/complex/
├── mod.rs                 # Rust API wrapper
├── compute.rs             # Shader dispatch logic
├── tests.rs               # Comprehensive unit tests
└── shaders/
    ├── add.wgsl           # complex_add
    ├── sub.wgsl           # complex_sub
    ├── mul.wgsl           # complex_mul (CRITICAL for FFT!)
    ├── conj.wgsl          # complex_conj
    ├── abs.wgsl           # complex_abs, abs_sq
    ├── exp.wgsl           # complex_exp (CRITICAL for FFT twiddles!)
    ├── div.wgsl           # complex_div
    ├── sqrt.wgsl          # complex_sqrt
    ├── log.wgsl           # complex_log
    └── pow.wgsl           # complex_pow
```

---

## 🧪 Testing Strategy

### Unit Tests (Rust)
```rust
#[test]
fn test_complex_mul() {
    // (3 + 4i) * (1 + 2i) = (3 - 8) + (6 + 4)i = -5 + 10i
    let z1 = vec2(3.0, 4.0);
    let z2 = vec2(1.0, 2.0);
    let result = complex_mul(z1, z2);
    assert_approx_eq!(result, vec2(-5.0, 10.0), 1e-6);
}

#[test]
fn test_complex_exp() {
    // exp(iπ) = -1 (Euler's identity)
    let z = vec2(0.0, std::f32::consts::PI);
    let result = complex_exp(z);
    assert_approx_eq!(result, vec2(-1.0, 0.0), 1e-5);
}
```

### Validation Tests
1. **Commutativity**: z1 + z2 = z2 + z1
2. **Associativity**: (z1 + z2) + z3 = z1 + (z2 + z3)
3. **Euler's identity**: exp(iπ) + 1 = 0
4. **De Moivre**: (cos θ + i sin θ)^n = cos(nθ) + i sin(nθ)
5. **Inverse properties**: z * (1/z) = 1, sqrt(z)² = z

### Benchmark Tests
```rust
#[bench]
fn bench_complex_mul_1m(b: &mut Bencher) {
    // 1M complex multiplications
    // Target: < 10ms on RTX 3090
}
```

---

## 🚀 Implementation Phases

### Week 1: Foundation
- [ ] Design Complex type (vec2 vs struct decision)
- [ ] Implement add, sub, conj (trivial ops)
- [ ] Set up module structure
- [ ] Write basic unit tests

### Week 2: Critical Ops
- [ ] Implement complex_mul (FFT blocker!)
- [ ] Implement complex_exp (twiddle blocker!)
- [ ] Implement complex_abs
- [ ] Comprehensive testing (Euler's identity, etc.)

### Week 3: Extended Ops
- [ ] Implement div, sqrt, log, pow
- [ ] Validation tests (inverse properties)
- [ ] Performance benchmarks

### Week 4: Integration
- [ ] Tensor integration (Complex tensor type)
- [ ] Documentation + examples
- [ ] FFT team handoff (unblock Phase 1.2)

---

## 🔗 Dependencies

**Input Dependencies**: None (foundational type)  
**Output Dependencies**: FFT (blocks everything!)

**Blocks**:
- ✅ Phase 1.2: Complex FFT
- ✅ 90% of physics primitives (PPPM, structure factors, wave propagation)

**Enables**:
- FFT team can start adapting NTT → FFT
- Complex-valued ML architectures (future)
- Quantum computing primitives (far future)

---

## 📊 Success Criteria

1. ✅ All 10 complex operations pass unit tests
2. ✅ Euler's identity verified to 1e-5 precision
3. ✅ 1M complex_mul < 10ms on RTX 3090
4. ✅ Zero unsafe code, 100% WGSL
5. ✅ FFT team confirms: "We can proceed"

---

## 🎓 Learning from Existing Patterns

### Pattern 1: Simple Approximation (from erf.wgsl)
```wgsl
// Abramowitz & Stegun formula
fn erf_approx(x: f32) -> f32 {
    let a1 = 0.254829592;
    // ... polynomial coefficients
    let t = 1.0 / (1.0 + p * abs_x);
    return sign * (1.0 - polynomial(t) * exp(-x²));
}
```
**Lesson**: Polynomial approximations work well for special functions.

### Pattern 2: Reflection Formula (from lgamma.wgsl)
```wgsl
if (x < 0.5) {
    // Use reflection: Γ(1-z)Γ(z) = π/sin(πz)
    return log(pi / sin(pi * x)) - lgamma_approx(1.0 - x);
}
```
**Lesson**: Analytic continuations extend domain.

### Pattern 3: Iterative Refinement (from u64_emu in NTT)
```wgsl
fn u64_mod_simple(a: U64, m: U64) -> U64 {
    var result = a;
    for (var i = 0u; i < 128u; i++) {
        if (u64_ge(result, m)) {
            result = u64_sub(result, m);
        }
    }
    return result;
}
```
**Lesson**: Simple loops work when GPU parallelism hides latency.

---

## 💡 Design Decision: vec2<f32> vs struct

**Recommendation**: **vec2<f32>** (Option A)

**Rationale**:
1. **Performance**: Native SIMD, direct register mapping
2. **Simplicity**: 90% of ops are trivial with vec2 (add, sub, conj)
3. **Compatibility**: Works with existing texture/buffer formats
4. **Precedent**: GLSL/HLSL complex math uses vec2 convention

**Migration Path**: If struct becomes necessary later (type safety, metadata), we can wrap vec2 internally. Start simple.

---

**Status**: ✅ Design complete, ready for implementation  
**Next Step**: Week 1 - Implement foundation (add, sub, conj, module structure)  
**Blocker Removal**: FFT can proceed after Week 2 (mul + exp complete)
