// Clip Grad Norm - Gradient clipping by total norm
// Prevents exploding gradients by scaling gradients if their norm exceeds max_norm
//
// Algorithm:
// 1. Compute total norm: ||g|| = sqrt(Σ g_i^2)
// 2. If ||g|| > max_norm: scale = max_norm / ||g||
// 3. Apply scaling: g_clipped = g * scale

struct Params {
    size: u32,
    max_norm: f32,
    _pad1: u32,
    _pad2: u32,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> gradients: array<f32>;
@group(0) @binding(2) var<storage, read_write> norm_buffer: array<atomic<i32>>; // [1] - computed norm squared (atomic)
@group(0) @binding(3) var<storage, read_write> output: array<f32>;

// Step 1: Compute norm squared (parallel reduction partial sums)
@compute @workgroup_size(256)
fn compute_norm(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= params.size) {
        return;
    }

    let grad = gradients[idx];
    let grad_sq = grad * grad;
    
    // Atomic add to accumulate norm squared
    // Note: This is a simplification. In production, use parallel reduction.
    atomicAdd(&norm_buffer[0], bitcast<i32>(grad_sq));
}

// Step 2: Clip gradients based on computed norm
@compute @workgroup_size(256)
fn clip_gradients(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= params.size) {
        return;
    }

    let norm_sq = bitcast<f32>(atomicLoad(&norm_buffer[0]));
    let total_norm = sqrt(norm_sq);
    var scale = 1.0;
    
    if (total_norm > params.max_norm) {
        scale = params.max_norm / (total_norm + 1e-8);
    }
    
    output[idx] = gradients[idx] * scale;
}
