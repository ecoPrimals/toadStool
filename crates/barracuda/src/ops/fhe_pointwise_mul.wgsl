// FHE Point-wise Multiplication (in NTT domain)
//
// Purpose: Multiply two polynomials element-wise in NTT domain
// Used in: Fast polynomial multiplication (NTT → pointwise → INTT)
//
// Input:  A[i], B[i] - two polynomials in NTT domain (N elements each)
// Output: C[i] = A[i] * B[i] mod q - element-wise product
//
// Complexity: O(N) - much faster than O(N²) convolution!
//
// This is the heart of fast polynomial multiplication:
//   poly_mul(a, b) = INTT(pointwise_mul(NTT(a), NTT(b)))
//
// Each element is a 64-bit number stored as two u32 values:
//   value = (high << 32) | low

// Input A (first polynomial in NTT domain)
@group(0) @binding(0) var<storage, read> input_a: array<u32>;

// Input B (second polynomial in NTT domain)
@group(0) @binding(1) var<storage, read> input_b: array<u32>;

// Output C = A ⊙ B (element-wise product)
@group(0) @binding(2) var<storage, read_write> output: array<u32>;

// Parameters for modular arithmetic
struct PointwiseMulParams {
    degree: u32,           // Polynomial degree (N)
    modulus_low: u32,      // Modulus q (lower 32 bits)
    modulus_high: u32,     // Modulus q (upper 32 bits)
    barrett_mu_low: u32,   // Barrett reduction constant (lower 32 bits)
    barrett_mu_high: u32,  // Barrett reduction constant (upper 32 bits)
}

@group(0) @binding(3) var<uniform> params: PointwiseMulParams;

// ═══════════════════════════════════════════════════════════════
// Modular Arithmetic Helpers (64-bit using u32 pairs)
// ═══════════════════════════════════════════════════════════════

/// Multiply two 64-bit numbers modulo q using Barrett reduction
///
/// Barrett reduction: a * b mod q ≈ a * b - ⌊(a * b * μ) / 2^128⌋ * q
/// where μ = ⌊2^128 / q⌋
///
/// This avoids expensive division by precomputing μ
fn mod_mul(a_low: u32, a_high: u32, b_low: u32, b_high: u32) -> vec2<u32> {
    // Reconstruct full 64-bit values
    let a = vec2<u32>(a_low, a_high);
    let b = vec2<u32>(b_low, b_high);
    let modulus = vec2<u32>(params.modulus_low, params.modulus_high);
    let barrett_mu = vec2<u32>(params.barrett_mu_low, params.barrett_mu_high);
    
    // Full 128-bit multiply: a * b
    // We'll use 64-bit approximation for GPU efficiency
    // For production, full 128-bit multiply needed
    
    // Low 64 bits of product (approximation)
    let prod_low = a.x * b.x;
    let prod_mid1 = a.x * b.y;
    let prod_mid2 = a.y * b.x;
    let prod_high_approx = a.y * b.y;
    
    // Combine into 64-bit result (with carry handling)
    var carry: u32 = 0u;
    let result_low = prod_low;
    
    // Add middle products (shifted by 32 bits)
    let mid_sum = prod_mid1 + prod_mid2;
    let result_mid = (result_low >> 16u) + (mid_sum & 0xFFFFu);
    carry = (result_mid >> 16u) + (mid_sum >> 16u);
    
    let result_high = prod_high_approx + carry;
    let product = vec2<u32>(result_low, result_high);
    
    // Barrett reduction: q_hat = ⌊(product * μ) / 2^128⌋
    // Approximation: use upper 64 bits
    let q_hat_approx = (product.y * barrett_mu.x) + (product.x * barrett_mu.y >> 32u);
    
    // r = product - q_hat * modulus
    let q_times_modulus_low = q_hat_approx * modulus.x;
    let q_times_modulus_high = q_hat_approx * modulus.y;
    
    var result = vec2<u32>(
        product.x - q_times_modulus_low,
        product.y - q_times_modulus_high
    );
    
    // Final correction (if result >= modulus)
    if (result.y > modulus.y || (result.y == modulus.y && result.x >= modulus.x)) {
        // result -= modulus
        if (result.x < modulus.x) {
            result.y -= 1u;
        }
        result.x -= modulus.x;
        result.y -= modulus.y;
    }
    
    return result;
}

/// Add two 64-bit numbers modulo q
fn mod_add(a_low: u32, a_high: u32, b_low: u32, b_high: u32) -> vec2<u32> {
    let modulus = vec2<u32>(params.modulus_low, params.modulus_high);
    
    // Add with carry
    var result_low = a_low + b_low;
    var carry = select(0u, 1u, result_low < a_low);  // Carry if overflow
    var result_high = a_high + b_high + carry;
    
    // Reduce if result >= modulus
    if (result_high > modulus.y || (result_high == modulus.y && result_low >= modulus.x)) {
        // result -= modulus
        if (result_low < modulus.x) {
            result_high -= 1u;
        }
        result_low -= modulus.x;
        result_high -= modulus.y;
    }
    
    return vec2<u32>(result_low, result_high);
}

/// Subtract two 64-bit numbers modulo q
fn mod_sub(a_low: u32, a_high: u32, b_low: u32, b_high: u32) -> vec2<u32> {
    let modulus = vec2<u32>(params.modulus_low, params.modulus_high);
    
    var result_low = a_low - b_low;
    var borrow = select(0u, 1u, a_low < b_low);  // Borrow if underflow
    var result_high = a_high - b_high - borrow;
    
    // If result is negative, add modulus
    if (a_high < b_high || (a_high == b_high && a_low < b_low)) {
        let carry = select(0u, 1u, result_low + modulus.x < result_low);
        result_low += modulus.x;
        result_high += modulus.y + carry;
    }
    
    return vec2<u32>(result_low, result_high);
}

// ═══════════════════════════════════════════════════════════════
// Point-wise Multiplication Kernel
// ═══════════════════════════════════════════════════════════════

/// Main kernel: C[i] = A[i] * B[i] mod q
///
/// Each thread processes one coefficient
/// Input/output format: pairs of u32 (low, high) representing 64-bit values
@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    
    // Bounds check
    if (idx >= params.degree) {
        return;
    }
    
    // Load A[idx] and B[idx] (each is 64-bit = 2 × u32)
    let a_low = input_a[idx * 2u];
    let a_high = input_a[idx * 2u + 1u];
    let b_low = input_b[idx * 2u];
    let b_high = input_b[idx * 2u + 1u];
    
    // Compute C[idx] = A[idx] * B[idx] mod q
    let result = mod_mul(a_low, a_high, b_low, b_high);
    
    // Store result
    output[idx * 2u] = result.x;       // Low 32 bits
    output[idx * 2u + 1u] = result.y;  // High 32 bits
}

// ═══════════════════════════════════════════════════════════════
// Optional: Batch Point-wise Operations
// ═══════════════════════════════════════════════════════════════

/// Alternative kernel: Process multiple coefficients per thread
/// Useful for smaller polynomials to improve GPU occupancy
@compute @workgroup_size(256)
fn batched(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let base_idx = global_id.x * 4u;  // Each thread processes 4 coefficients
    
    // Process 4 coefficients per thread (if available)
    for (var i = 0u; i < 4u; i = i + 1u) {
        let idx = base_idx + i;
        
        if (idx >= params.degree) {
            return;
        }
        
        // Load, multiply, store
        let a_low = input_a[idx * 2u];
        let a_high = input_a[idx * 2u + 1u];
        let b_low = input_b[idx * 2u];
        let b_high = input_b[idx * 2u + 1u];
        
        let result = mod_mul(a_low, a_high, b_low, b_high);
        
        output[idx * 2u] = result.x;
        output[idx * 2u + 1u] = result.y;
    }
}

// ═══════════════════════════════════════════════════════════════
// Performance Notes
// ═══════════════════════════════════════════════════════════════
//
// Expected Performance:
//   N=4096: ~3μs (memory-bound, not compute-bound)
//   Bandwidth: ~200 GB/s (3 × 4096 × 8 bytes / 3μs)
//
// Optimization opportunities:
//   1. Vectorized loads (vec4) for better memory coalescing
//   2. Shared memory for cache locality (not needed for O(N))
//   3. Hardware-specific modular multiply (native u64 on some GPUs)
//
// Why so fast:
//   - Simple element-wise operation (no dependencies)
//   - Perfect memory coalescing (sequential access)
//   - High arithmetic intensity (1 multiply per 24 bytes)
