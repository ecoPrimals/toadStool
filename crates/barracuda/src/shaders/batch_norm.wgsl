// BatchNorm - Batch normalization
// output = (input - mean) / sqrt(variance + epsilon) * gamma + beta
// Simplified version: per-tensor normalization with default gamma=1, beta=0

struct BatchNormParams {
    epsilon: f32,
    _padding: vec3<f32>,
}

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: BatchNormParams;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let size = arrayLength(&input);
    
    if (idx >= size) {
        return;
    }
    
    // Compute mean (simplified: single pass for small tensors)
    if (idx == 0u) {
        var sum = 0.0;
        for (var i = 0u; i < size; i = i + 1u) {
            sum = sum + input[i];
        }
        let mean = sum / f32(size);
        
        // Compute variance
        var variance = 0.0;
        for (var i = 0u; i < size; i = i + 1u) {
            let diff = input[i] - mean;
            variance = variance + diff * diff;
        }
        variance = variance / f32(size);
        
        // Normalize all elements (gamma=1, beta=0 for simplicity)
        let std_dev = sqrt(variance + params.epsilon);
        for (var i = 0u; i < size; i = i + 1u) {
            output[i] = (input[i] - mean) / std_dev;
        }
    }
}
