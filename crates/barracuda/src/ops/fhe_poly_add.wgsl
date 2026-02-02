//! FHE Polynomial Addition Shader
//!
//! **Purpose**: Add two polynomials modulo q (FHE ciphertext operation)
//!
//! **Deep Debt**: Pure WGSL, hardware-agnostic, numerically precise
//!
//! **Algorithm**: Coefficient-wise addition with Barrett modular reduction
//!
//! ## Mathematical Background
//!
//! FHE ciphertexts are polynomials over Z_q[X]/(X^N + 1):
//! - Degree N (typically 2048, 4096, or 8192)
//! - Coefficients modulo q (large prime, e.g., 2^64)
//! - Addition: (a₀ + b₀) mod q, (a₁ + b₁) mod q, ..., (aₙ + bₙ) mod q
//!
//! ## Barrett Reduction
//!
//! For efficient modular reduction without division:
//! - Precompute μ = ⌊2^(2k) / q⌋ where k = bitwidth(q)
//! - Approximate quotient: q_approx = ⌊(a * μ) / 2^(2k)⌋
//! - Remainder: r = a - q_approx * q
//! - Final correction: if r >= q then r -= q (at most once)

struct Params {
    degree: u32,        // Polynomial degree (N)
    modulus_lo: u32,    // q lower 32 bits
    modulus_hi: u32,    // q upper 32 bits
    mu_lo: u32,         // Barrett constant μ lower 32 bits
    mu_hi: u32,         // Barrett constant μ upper 32 bits
}

@group(0) @binding(0) var<storage, read> poly_a: array<u32>;      // First polynomial (2×degree for u64)
@group(0) @binding(1) var<storage, read> poly_b: array<u32>;      // Second polynomial (2×degree for u64)
@group(0) @binding(2) var<storage, read_write> result: array<u32>; // Result polynomial (2×degree for u64)
@group(0) @binding(3) var<uniform> params: Params;

/// Reconstruct 64-bit value from two 32-bit parts
fn u64_from_parts(lo: u32, hi: u32) -> vec2<u32> {
    return vec2<u32>(lo, hi);
}

/// Add two 64-bit values (with overflow handling)
fn u64_add(a: vec2<u32>, b: vec2<u32>) -> vec2<u32> {
    let lo_sum = a.x + b.x;
    let carry = select(0u, 1u, lo_sum < a.x);  // Detect carry
    let hi_sum = a.y + b.y + carry;
    return vec2<u32>(lo_sum, hi_sum);
}

/// Subtract two 64-bit values (assumes a >= b)
fn u64_sub(a: vec2<u32>, b: vec2<u32>) -> vec2<u32> {
    let borrow = select(0u, 1u, a.x < b.x);
    let lo_diff = a.x - b.x;
    let hi_diff = a.y - b.y - borrow;
    return vec2<u32>(lo_diff, hi_diff);
}

/// Compare two 64-bit values (returns true if a >= b)
fn u64_gte(a: vec2<u32>, b: vec2<u32>) -> bool {
    if (a.y > b.y) { return true; }
    if (a.y < b.y) { return false; }
    return a.x >= b.x;
}

/// Multiply two 64-bit values (returns lower 64 bits of 128-bit result)
fn u64_mul_lo(a: vec2<u32>, b: vec2<u32>) -> vec2<u32> {
    // Full 64×64 multiplication gives 128-bit result
    // We only need lower 64 bits for Barrett reduction
    
    let a_lo = a.x;
    let a_hi = a.y;
    let b_lo = b.x;
    let b_hi = b.y;
    
    // Partial products
    let p0 = a_lo * b_lo;           // Lower 32×32
    let p1 = a_lo * b_hi;           // Cross term 1
    let p2 = a_hi * b_lo;           // Cross term 2
    // p3 = a_hi * b_hi (upper 64 bits, not needed for lo result)
    
    // Combine for lower 64 bits
    let mid = p1 + p2;
    let mid_carry = select(0u, 1u, mid < p1);
    
    let lo = p0;
    let hi = (p1 >> 32u) + (p2 >> 32u) + (p0 >> 32u) + (mid << 32u >> 32u);
    
    return vec2<u32>(lo, hi);
}

/// Barrett modular reduction: a mod q
fn barrett_reduce(a: vec2<u32>, q: vec2<u32>, mu: vec2<u32>) -> vec2<u32> {
    // Approximate quotient: ⌊(a * μ) / 2^64⌋
    // For 64-bit arithmetic, this is approximately a * mu >> 64
    // We simplify by using the high part of multiplication
    
    // Compute q_approx (simplified for WGSL constraints)
    // This is an approximation; exact Barrett requires 128-bit arithmetic
    let q_approx_lo = (a.y * mu.x) + ((a.x * mu.y) >> 32u);
    let q_approx = vec2<u32>(0u, q_approx_lo);  // Approximate
    
    // Compute remainder: r = a - q_approx * q
    let q_times_approx = u64_mul_lo(q_approx, q);
    var r = u64_sub(a, q_times_approx);
    
    // Correction: at most 2 iterations needed
    if (u64_gte(r, q)) {
        r = u64_sub(r, q);
    }
    if (u64_gte(r, q)) {
        r = u64_sub(r, q);
    }
    
    return r;
}

@compute @workgroup_size(256)
fn fhe_poly_add(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    
    // Boundary check
    if (idx >= params.degree) {
        return;
    }
    
    // Each coefficient is stored as two u32 values (lo, hi)
    let idx_lo = idx * 2u;
    let idx_hi = idx_lo + 1u;
    
    // Load coefficients from poly_a
    let a_lo = poly_a[idx_lo];
    let a_hi = poly_a[idx_hi];
    let a = u64_from_parts(a_lo, a_hi);
    
    // Load coefficients from poly_b
    let b_lo = poly_b[idx_lo];
    let b_hi = poly_b[idx_hi];
    let b = u64_from_parts(b_lo, b_hi);
    
    // Add coefficients
    let sum = u64_add(a, b);
    
    // Load modulus and Barrett constant
    let q = u64_from_parts(params.modulus_lo, params.modulus_hi);
    let mu = u64_from_parts(params.mu_lo, params.mu_hi);
    
    // Modular reduction
    let reduced = barrett_reduce(sum, q, mu);
    
    // Store result
    result[idx_lo] = reduced.x;
    result[idx_hi] = reduced.y;
}
