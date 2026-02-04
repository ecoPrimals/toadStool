// Log gamma function (lgamma) operation
// lgamma(x) = ln(Γ(x)) where Γ is the gamma function
// Uses Lanczos approximation for positive x

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> metadata: Metadata;

struct Metadata {
    size: u32,
}

// Lanczos approximation for gamma function
// Accurate for x > 0
fn lgamma_approx(x: f32) -> f32 {
    if (x <= 0.0) {
        return 0.0 / 0.0; // NaN for non-positive values
    }
    
    // Lanczos coefficients (g=7, n=9)
    let coef = array<f32, 9>(
        0.99999999999980993,
        676.5203681218851,
        -1259.1392167224028,
        771.32342877765313,
        -176.61502916214059,
        12.507343278686905,
        -0.13857109526572012,
        9.9843695780195716e-6,
        1.5056327351493116e-7
    );
    
    let g = 7.0;
    
    if (x < 0.5) {
        // Use reflection formula: Γ(1-z)Γ(z) = π/sin(πz)
        let pi = 3.14159265358979323846;
        return log(pi / sin(pi * x)) - lgamma_approx(1.0 - x);
    }
    
    let x_shifted = x - 1.0;
    var sum = coef[0];
    
    for (var i = 1; i < 9; i = i + 1) {
        sum += coef[i] / (x_shifted + f32(i));
    }
    
    let t = x_shifted + g + 0.5;
    let sqrt_2pi = 2.5066282746310002; // sqrt(2π)
    
    return log(sqrt_2pi) + log(sum) + (x_shifted + 0.5) * log(t) - t;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= metadata.size) {
        return;
    }
    
    let x = input[idx];
    output[idx] = lgamma_approx(x);
}
