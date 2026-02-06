//! FHE Property-Based Tests
//!
//! Validates fundamental cryptographic and mathematical properties:
//! 1. NTT-INTT Round-trip (perfect reconstruction)
//! 2. Modulus Switch Correctness (preserves mod relationships)
//! 3. Rotation Composition (rotate(a+b) = rotate(a) ∘ rotate(b))
//! 4. Homomorphic Properties (enc(a) + enc(b) = enc(a+b))
//! 5. Key Switch Security (ciphertext remains valid)

use barracuda::device::Device;
use barracuda::ops::{
    FheNtt, FheIntt, FheModulusSwitch, FheRotate, FhePolyAdd, FhePolySub, FhePolyMul,
};
use barracuda::tensor::Tensor;

/// Helper: Create test device
fn test_device() -> Device {
    pollster::block_on(Device::new()).expect("Failed to create device")
}

/// Helper: Create U64 tensor from data
fn test_tensor_u64(device: &Device, data: &[u64]) -> Tensor {
    let shape = vec![data.len()];
    Tensor::from_data(device, data, &shape).expect("Failed to create tensor")
}

/// Helper: Extract U64 data from tensor
fn tensor_to_u64(tensor: &Tensor) -> Vec<u64> {
    pollster::block_on(tensor.to_vec()).expect("Failed to read tensor")
}

/// Helper: Modular addition
fn mod_add(a: u64, b: u64, modulus: u64) -> u64 {
    ((a as u128 + b as u128) % modulus as u128) as u64
}

/// Helper: Modular subtraction
fn mod_sub(a: u64, b: u64, modulus: u64) -> u64 {
    if a >= b {
        (a - b) % modulus
    } else {
        (modulus - ((b - a) % modulus)) % modulus
    }
}

// ============================================================================
// PROPERTY 1: NTT-INTT Round-trip (Perfect Reconstruction)
// ============================================================================

#[test]
fn test_ntt_intt_roundtrip_small() {
    let device = test_device();
    let degree = 4;
    let modulus = 17; // Small prime for testing

    // Test data
    let input_data = vec![1u64, 2, 3, 4];
    let input = test_tensor_u64(&device, &input_data);

    // Forward NTT
    let ntt = FheNtt::new(&device, input.clone(), degree, modulus)
        .expect("Failed to create NTT");
    let ntt_output = ntt.execute().expect("NTT failed");

    // Inverse NTT
    let intt = FheIntt::new(&device, ntt_output, degree, modulus)
        .expect("Failed to create INTT");
    let recovered = intt.execute().expect("INTT failed");

    // Verify perfect reconstruction
    let recovered_data = tensor_to_u64(&recovered);
    assert_eq!(
        input_data, recovered_data,
        "NTT-INTT round-trip failed: input={:?}, recovered={:?}",
        input_data, recovered_data
    );
}

#[test]
fn test_ntt_intt_roundtrip_powers_of_two() {
    let device = test_device();
    let test_cases = vec![
        (4, 17u64),
        (8, 257),
        (16, 65537),
    ];

    for (degree, modulus) in test_cases {
        // Generate test data
        let input_data: Vec<u64> = (1..=degree).map(|i| i as u64 % modulus).collect();
        let input = test_tensor_u64(&device, &input_data);

        // NTT → INTT
        let ntt = FheNtt::new(&device, input.clone(), degree as u32, modulus)
            .expect("Failed to create NTT");
        let ntt_output = ntt.execute().expect("NTT failed");

        let intt = FheIntt::new(&device, ntt_output, degree as u32, modulus)
            .expect("Failed to create INTT");
        let recovered = intt.execute().expect("INTT failed");

        // Verify
        let recovered_data = tensor_to_u64(&recovered);
        assert_eq!(
            input_data, recovered_data,
            "NTT-INTT round-trip failed for degree={}, modulus={}: input={:?}, recovered={:?}",
            degree, modulus, input_data, recovered_data
        );
    }
}

#[test]
fn test_ntt_intt_roundtrip_random_data() {
    let device = test_device();
    let degree = 16;
    let modulus = 65537u64;

    // Random data
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let input_data: Vec<u64> = (0..degree)
        .map(|_| rng.gen_range(0..modulus))
        .collect();
    let input = test_tensor_u64(&device, &input_data);

    // NTT → INTT
    let ntt = FheNtt::new(&device, input.clone(), degree, modulus)
        .expect("Failed to create NTT");
    let ntt_output = ntt.execute().expect("NTT failed");

    let intt = FheIntt::new(&device, ntt_output, degree, modulus)
        .expect("Failed to create INTT");
    let recovered = intt.execute().expect("INTT failed");

    // Verify
    let recovered_data = tensor_to_u64(&recovered);
    assert_eq!(
        input_data, recovered_data,
        "NTT-INTT round-trip failed for random data: degree={}, modulus={}",
        degree, modulus
    );
}

// ============================================================================
// PROPERTY 2: Modulus Switch Correctness
// ============================================================================

#[test]
fn test_modulus_switch_preserves_value() {
    let device = test_device();
    let degree = 4;
    let modulus_from = 257u64;
    let modulus_to = 17u64;

    // Test data
    let input_data = vec![10u64, 20, 30, 40];
    let input = test_tensor_u64(&device, &input_data);

    // Switch modulus
    let switch = FheModulusSwitch::new(&device, input.clone(), degree, modulus_from, modulus_to)
        .expect("Failed to create modulus switch");
    let output = switch.execute().expect("Modulus switch failed");

    // Verify: output[i] ≡ input[i] (mod modulus_to)
    let output_data = tensor_to_u64(&output);
    for (i, (&inp, &out)) in input_data.iter().zip(output_data.iter()).enumerate() {
        let expected = inp % modulus_to;
        assert_eq!(
            out, expected,
            "Modulus switch correctness failed at index {}: input={}, output={}, expected={}",
            i, inp, out, expected
        );
    }
}

#[test]
fn test_modulus_switch_idempotent() {
    let device = test_device();
    let degree = 8;
    let modulus = 257u64;

    // Test data
    let input_data: Vec<u64> = (0..degree).map(|i| (i * 10) % modulus).collect();
    let input = test_tensor_u64(&device, &input_data);

    // Switch to same modulus (should be identity)
    let switch = FheModulusSwitch::new(&device, input.clone(), degree as u32, modulus, modulus)
        .expect("Failed to create modulus switch");
    let output = switch.execute().expect("Modulus switch failed");

    // Verify: output == input
    let output_data = tensor_to_u64(&output);
    assert_eq!(
        input_data, output_data,
        "Idempotent modulus switch failed: modulus={}", modulus
    );
}

// ============================================================================
// PROPERTY 3: Rotation Composition
// ============================================================================

#[test]
fn test_rotation_composition() {
    let device = test_device();
    let degree = 8;
    let modulus = 257u64;

    // Test data
    let input_data: Vec<u64> = (0..degree).map(|i| i as u64).collect();
    let input = test_tensor_u64(&device, &input_data);

    // Rotate by 2, then by 3 (should equal rotate by 5)
    let rotate1 = FheRotate::new(&device, input.clone(), degree as u32, modulus, 2)
        .expect("Failed to create rotation 1");
    let intermediate = rotate1.execute().expect("Rotation 1 failed");

    let rotate2 = FheRotate::new(&device, intermediate, degree as u32, modulus, 3)
        .expect("Failed to create rotation 2");
    let result_composed = rotate2.execute().expect("Rotation 2 failed");

    // Direct rotate by 5
    let rotate_direct = FheRotate::new(&device, input.clone(), degree as u32, modulus, 5)
        .expect("Failed to create direct rotation");
    let result_direct = rotate_direct.execute().expect("Direct rotation failed");

    // Verify: rotate(2) ∘ rotate(3) == rotate(5)
    let composed_data = tensor_to_u64(&result_composed);
    let direct_data = tensor_to_u64(&result_direct);
    assert_eq!(
        composed_data, direct_data,
        "Rotation composition failed: rotate(2) ∘ rotate(3) != rotate(5)"
    );
}

#[test]
fn test_rotation_inverse() {
    let device = test_device();
    let degree = 8;
    let modulus = 257u64;

    // Test data
    let input_data: Vec<u64> = (0..degree).map(|i| (i * 10) as u64).collect();
    let input = test_tensor_u64(&device, &input_data);

    // Rotate forward by k, then backward by k (should be identity)
    let k = 3;
    let rotate_fwd = FheRotate::new(&device, input.clone(), degree as u32, modulus, k)
        .expect("Failed to create forward rotation");
    let rotated = rotate_fwd.execute().expect("Forward rotation failed");

    let rotate_back = FheRotate::new(&device, rotated, degree as u32, modulus, degree as u32 - k)
        .expect("Failed to create backward rotation");
    let recovered = rotate_back.execute().expect("Backward rotation failed");

    // Verify: rotate(k) ∘ rotate(-k) == identity
    let recovered_data = tensor_to_u64(&recovered);
    assert_eq!(
        input_data, recovered_data,
        "Rotation inverse failed: rotate({}) ∘ rotate({}) != identity",
        k, degree as u32 - k
    );
}

// ============================================================================
// PROPERTY 4: Homomorphic Properties
// ============================================================================

#[test]
fn test_homomorphic_addition() {
    let device = test_device();
    let degree = 4;
    let modulus = 257u64;

    // Test data
    let a_data = vec![10u64, 20, 30, 40];
    let b_data = vec![5u64, 15, 25, 35];
    let a = test_tensor_u64(&device, &a_data);
    let b = test_tensor_u64(&device, &b_data);

    // Homomorphic addition: enc(a) + enc(b)
    let add = FhePolyAdd::new(&device, a.clone(), b.clone(), degree, modulus)
        .expect("Failed to create poly add");
    let result = add.execute().expect("Poly add failed");

    // Expected: (a + b) mod modulus
    let result_data = tensor_to_u64(&result);
    for (i, ((&a_val, &b_val), &result_val)) in a_data.iter().zip(b_data.iter()).zip(result_data.iter()).enumerate() {
        let expected = mod_add(a_val, b_val, modulus);
        assert_eq!(
            result_val, expected,
            "Homomorphic addition failed at index {}: a={}, b={}, result={}, expected={}",
            i, a_val, b_val, result_val, expected
        );
    }
}

#[test]
fn test_homomorphic_subtraction() {
    let device = test_device();
    let degree = 4;
    let modulus = 257u64;

    // Test data
    let a_data = vec![40u64, 30, 20, 10];
    let b_data = vec![5u64, 15, 25, 35];
    let a = test_tensor_u64(&device, &a_data);
    let b = test_tensor_u64(&device, &b_data);

    // Homomorphic subtraction: enc(a) - enc(b)
    let sub = FhePolySub::new(&device, a.clone(), b.clone(), degree, modulus)
        .expect("Failed to create poly sub");
    let result = sub.execute().expect("Poly sub failed");

    // Expected: (a - b) mod modulus
    let result_data = tensor_to_u64(&result);
    for (i, ((&a_val, &b_val), &result_val)) in a_data.iter().zip(b_data.iter()).zip(result_data.iter()).enumerate() {
        let expected = mod_sub(a_val, b_val, modulus);
        assert_eq!(
            result_val, expected,
            "Homomorphic subtraction failed at index {}: a={}, b={}, result={}, expected={}",
            i, a_val, b_val, result_val, expected
        );
    }
}

#[test]
fn test_homomorphic_associativity() {
    let device = test_device();
    let degree = 4;
    let modulus = 257u64;

    // Test data
    let a_data = vec![10u64, 20, 30, 40];
    let b_data = vec![5u64, 15, 25, 35];
    let c_data = vec![3u64, 7, 11, 13];
    let a = test_tensor_u64(&device, &a_data);
    let b = test_tensor_u64(&device, &b_data);
    let c = test_tensor_u64(&device, &c_data);

    // (a + b) + c
    let ab = FhePolyAdd::new(&device, a.clone(), b.clone(), degree, modulus)
        .expect("Failed to create a+b");
    let ab_result = ab.execute().expect("a+b failed");
    let abc1 = FhePolyAdd::new(&device, ab_result, c.clone(), degree, modulus)
        .expect("Failed to create (a+b)+c");
    let result1 = abc1.execute().expect("(a+b)+c failed");

    // a + (b + c)
    let bc = FhePolyAdd::new(&device, b.clone(), c.clone(), degree, modulus)
        .expect("Failed to create b+c");
    let bc_result = bc.execute().expect("b+c failed");
    let abc2 = FhePolyAdd::new(&device, a.clone(), bc_result, degree, modulus)
        .expect("Failed to create a+(b+c)");
    let result2 = abc2.execute().expect("a+(b+c) failed");

    // Verify: (a + b) + c == a + (b + c)
    let result1_data = tensor_to_u64(&result1);
    let result2_data = tensor_to_u64(&result2);
    assert_eq!(
        result1_data, result2_data,
        "Associativity failed: (a+b)+c != a+(b+c)"
    );
}

// ============================================================================
// PROPERTY 5: Key Switch Security (Structural Validity)
// ============================================================================

#[test]
fn test_key_switch_preserves_structure() {
    // Note: Full key switching requires key pairs, which we don't have in unit tests
    // This test validates that the decomposition step preserves structure
    let device = test_device();
    let degree = 4;
    let modulus = 257u64;
    let decomp_base = 2;
    let decomp_levels = 4;

    // Test data (simulated ciphertext)
    let input_data = vec![100u64, 150, 200, 250];
    let input = test_tensor_u64(&device, &input_data);

    // Key switch decomposition
    use barracuda::ops::FheKeySwitch;
    let key_switch = FheKeySwitch::new(&device, input.clone(), degree, modulus, decomp_base, decomp_levels)
        .expect("Failed to create key switch");
    let output = key_switch.execute().expect("Key switch failed");

    // Verify: output shape is correct (degree * decomp_levels)
    let output_data = tensor_to_u64(&output);
    assert_eq!(
        output_data.len(),
        (degree * decomp_levels) as usize,
        "Key switch output size incorrect: expected {}, got {}",
        degree * decomp_levels,
        output_data.len()
    );

    // Verify: all values are within modulus
    for (i, &val) in output_data.iter().enumerate() {
        assert!(
            val < modulus,
            "Key switch output value out of range at index {}: {} >= {}",
            i, val, modulus
        );
    }
}

#[test]
fn test_key_switch_deterministic() {
    let device = test_device();
    let degree = 4;
    let modulus = 257u64;
    let decomp_base = 2;
    let decomp_levels = 4;

    // Test data
    let input_data = vec![50u64, 100, 150, 200];
    let input = test_tensor_u64(&device, &input_data);

    // Key switch twice with same parameters
    use barracuda::ops::FheKeySwitch;
    let ks1 = FheKeySwitch::new(&device, input.clone(), degree, modulus, decomp_base, decomp_levels)
        .expect("Failed to create key switch 1");
    let output1 = ks1.execute().expect("Key switch 1 failed");

    let ks2 = FheKeySwitch::new(&device, input.clone(), degree, modulus, decomp_base, decomp_levels)
        .expect("Failed to create key switch 2");
    let output2 = ks2.execute().expect("Key switch 2 failed");

    // Verify: same input → same output (deterministic)
    let output1_data = tensor_to_u64(&output1);
    let output2_data = tensor_to_u64(&output2);
    assert_eq!(
        output1_data, output2_data,
        "Key switch non-deterministic: same input produced different outputs"
    );
}

// ============================================================================
// Cross-Property Integration Tests
// ============================================================================

#[test]
fn test_ntt_preserves_addition() {
    let device = test_device();
    let degree = 8;
    let modulus = 257u64;

    // Test data
    let a_data: Vec<u64> = (0..degree).map(|i| (i * 5) as u64).collect();
    let b_data: Vec<u64> = (0..degree).map(|i| (i * 3) as u64).collect();
    let a = test_tensor_u64(&device, &a_data);
    let b = test_tensor_u64(&device, &b_data);

    // Method 1: Add in time domain, then NTT
    let add_time = FhePolyAdd::new(&device, a.clone(), b.clone(), degree as u32, modulus)
        .expect("Failed to create time domain add");
    let sum_time = add_time.execute().expect("Time domain add failed");
    let ntt_sum = FheNtt::new(&device, sum_time, degree as u32, modulus)
        .expect("Failed to create NTT of sum");
    let result1 = ntt_sum.execute().expect("NTT of sum failed");

    // Method 2: NTT individually, then add in frequency domain
    let ntt_a = FheNtt::new(&device, a.clone(), degree as u32, modulus)
        .expect("Failed to create NTT of a");
    let ntt_a_result = ntt_a.execute().expect("NTT of a failed");

    let ntt_b = FheNtt::new(&device, b.clone(), degree as u32, modulus)
        .expect("Failed to create NTT of b");
    let ntt_b_result = ntt_b.execute().expect("NTT of b failed");

    let add_freq = FhePolyAdd::new(&device, ntt_a_result, ntt_b_result, degree as u32, modulus)
        .expect("Failed to create frequency domain add");
    let result2 = add_freq.execute().expect("Frequency domain add failed");

    // Verify: NTT(a + b) == NTT(a) + NTT(b)
    let result1_data = tensor_to_u64(&result1);
    let result2_data = tensor_to_u64(&result2);
    assert_eq!(
        result1_data, result2_data,
        "NTT linearity failed: NTT(a+b) != NTT(a) + NTT(b)"
    );
}
