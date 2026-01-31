// TopK - Find top K largest values
// Simplified version: returns indices of top K elements
// Note: This is a basic implementation, production would use parallel sorting

struct TopKParams {
    k: u32,
}

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;
@group(0) @binding(2) var<uniform> params: TopKParams;

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let size = arrayLength(&input);
    let k = min(params.k, u32(size));
    
    // Simple selection: find top K indices
    // For each output position, find the next largest element
    for (var i = 0u; i < k; i = i + 1u) {
        var max_val = -3.402823466e+38; // -FLT_MAX
        var max_idx = 0u;
        
        for (var j = 0u; j < size; j = j + 1u) {
            var is_used = false;
            
            // Check if this index was already selected
            for (var prev = 0u; prev < i; prev = prev + 1u) {
                if (output[prev] == j) {
                    is_used = true;
                    break;
                }
            }
            
            if (!is_used && input[j] > max_val) {
                max_val = input[j];
                max_idx = j;
            }
        }
        
        output[i] = max_idx;
    }
}
