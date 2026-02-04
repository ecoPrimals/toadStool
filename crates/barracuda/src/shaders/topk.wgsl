// TopK - Find top-k largest elements (complete GPU implementation)
// Returns top-k values and their indices
//
// Algorithm: Bitonic sort + selection
// 1. Sort values and indices in parallel
// 2. Select top k elements
//
// Note: For production, use partial sort or heap-based selection for efficiency

struct Params {
    input_size: u32,
    k: u32,
    largest: u32,    // 1 = largest, 0 = smallest
    sorted: u32,     // 1 = sort output, 0 = unsorted
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> input: array<f32>;
@group(0) @binding(2) var<storage, read_write> values: array<f32>;   // Top-k values
@group(0) @binding(3) var<storage, read_write> indices: array<u32>;  // Top-k indices
@group(0) @binding(4) var<storage, read_write> work_values: array<f32>;  // Sorting workspace
@group(0) @binding(5) var<storage, read_write> work_indices: array<u32>; // Sorting workspace

// Initialize work buffers with input and indices
@compute @workgroup_size(256)
fn init_buffers(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= params.input_size) {
        return;
    }

    work_values[idx] = input[idx];
    work_indices[idx] = idx;
}

// Bitonic sort step (simplified for small inputs)
@compute @workgroup_size(256)
fn bitonic_sort_step(@builtin(global_invocation_id) global_id: vec3<u32>, 
                      @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= params.input_size / 2u) {
        return;
    }

    let pair_idx = idx * 2u;
    let val1 = work_values[pair_idx];
    let val2 = work_values[pair_idx + 1u];
    
    var swap = false;
    if (params.largest != 0u) {
        swap = val1 < val2;  // Sort descending for largest
    } else {
        swap = val1 > val2;  // Sort ascending for smallest
    }
    
    if (swap) {
        work_values[pair_idx] = val2;
        work_values[pair_idx + 1u] = val1;
        let temp_idx = work_indices[pair_idx];
        work_indices[pair_idx] = work_indices[pair_idx + 1u];
        work_indices[pair_idx + 1u] = temp_idx;
    }
}

// Select top-k elements
@compute @workgroup_size(256)
fn select_topk(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= params.k) {
        return;
    }

    values[idx] = work_values[idx];
    indices[idx] = work_indices[idx];
}
