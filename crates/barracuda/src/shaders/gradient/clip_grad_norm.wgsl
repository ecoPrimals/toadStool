// Clip Grad Norm - Gradient clipping by total norm
// Prevents exploding gradients by scaling gradients if their norm exceeds max_norm
//
// Algorithm:
// 1. Compute total norm: ||g|| = sqrt(Σ g_i^2) via workgroup tree reduction
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
@group(0) @binding(2) var<storage, read_write> norm_buffer: array<f32>;
@group(0) @binding(3) var<storage, read_write> output: array<f32>;

var<workgroup> shared_sq: array<f32, 256>;

// Step 1: Compute norm squared via workgroup tree reduction (same pattern as reduce.wgsl)
// Each workgroup writes its partial sum to norm_buffer[workgroup_id].
// When num_workgroups==1, norm_buffer[0] has the final sum.
@compute @workgroup_size(256)
fn compute_norm(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
) {
    let tid = local_id.x;
    let gid = global_id.x;

    var grad_sq = 0.0;
    if (gid < params.size) {
        let g = gradients[gid];
        grad_sq = g * g;
    }
    shared_sq[tid] = grad_sq;
    workgroupBarrier();

    // Tree reduction for sum of squares
    for (var stride = 128u; stride > 0u; stride = stride / 2u) {
        if (tid < stride) {
            shared_sq[tid] = shared_sq[tid] + shared_sq[tid + stride];
        }
        workgroupBarrier();
    }

    if (tid == 0u) {
        norm_buffer[workgroup_id.x] = shared_sq[0];
    }
}

// Step 2: Reduce partial sums to single value (when multiple workgroups)
// Dispatched with 1 workgroup when num_workgroups > 1
@compute @workgroup_size(256)
fn compute_norm_final(
    @builtin(local_invocation_id) local_id: vec3<u32>,
) {
    let tid = local_id.x;
    let num_partials = (params.size + 255u) / 256u;

    // Thread 0 sums all partials (typically small: e.g. 4 for size 1000)
    if (tid == 0u) {
        var total = 0.0;
        for (var i = 0u; i < num_partials; i++) {
            total = total + norm_buffer[i];
        }
        norm_buffer[0] = total;
    }
}

// Step 3: Clip gradients based on computed norm
@compute @workgroup_size(256)
fn clip_gradients(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= params.size) {
        return;
    }

    let norm_sq = norm_buffer[0];
    let total_norm = sqrt(norm_sq);
    var scale = 1.0;

    if (total_norm > params.max_norm) {
        scale = params.max_norm / (total_norm + 1e-8);
    }

    output[idx] = gradients[idx] * scale;
}
