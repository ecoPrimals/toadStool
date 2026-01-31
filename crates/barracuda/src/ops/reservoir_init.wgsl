// Reservoir initialization shader
// Generates random sparse matrix with controlled spectral radius

struct Params {
    size: u32,
    n: u32,
    spectral_radius: f32,
    connectivity: f32,
    seed: u32,
}

@group(0) @binding(0) var<storage, read_write> output: array<f32>;
@group(0) @binding(1) var<uniform> params: Params;

// Simple LCG (Linear Congruential Generator) for reproducible randomness
fn rand(state: ptr<function, u32>) -> f32 {
    let a = 1664525u;
    let c = 1013904223u;
    *state = (*state * a + c);
    return f32(*state) / 4294967296.0; // Normalize to [0, 1)
}

@compute @workgroup_size(256)
fn reservoir_init(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= params.n) {
        return;
    }
    
    // Initialize RNG with seed + index for unique values per element
    var rng_state = params.seed + idx;
    
    // Generate random value in [-1, 1]
    let random_val = rand(&rng_state) * 2.0 - 1.0;
    
    // Apply sparsity: randomly zero out based on connectivity
    let sparse_check = rand(&rng_state);
    var value = 0.0;
    if (sparse_check < params.connectivity) {
        value = random_val;
    }
    
    // Scale by spectral radius (simplified - actual spectral radius requires eigenvalue computation)
    // For reservoir initialization, we use a scaling approximation
    let scale_factor = params.spectral_radius / sqrt(f32(params.size) * params.connectivity);
    output[idx] = value * scale_factor;
}
