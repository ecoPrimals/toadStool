// Attention Apply: Apply attention weights to values
// Pass 3 of multi-pass attention implementation
//
// Computes: output[i,d] = sum_j (attention_weights[i,j] * V[j,d])
// This is the final step: weighted sum of values

struct AttentionParams {
    batch_size: u32,
    num_heads: u32,
    seq_len: u32,
    head_dim: u32,
}

@group(0) @binding(0) var<storage, read> weights: array<f32>;   // [batch, heads, seq, seq]
@group(0) @binding(1) var<storage, read> value: array<f32>;     // [batch, heads, seq, head_dim]
@group(0) @binding(2) var<storage, read_write> output: array<f32>; // [batch, heads, seq, head_dim]
@group(0) @binding(3) var<uniform> params: AttentionParams;

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let batch_head = global_id.z;
    let i = global_id.y; // output position
    let d = global_id.x; // dimension
    
    if (i >= params.seq_len || d >= params.head_dim) {
        return;
    }
    
    let batch = batch_head / params.num_heads;
    let head = batch_head % params.num_heads;
    
    if (batch >= params.batch_size) {
        return;
    }
    
    // Compute weighted sum: sum_j (weights[i,j] * V[j,d])
    var weighted_sum = 0.0;
    
    for (var j = 0u; j < params.seq_len; j = j + 1u) {
        let weight_idx = batch * params.num_heads * params.seq_len * params.seq_len
                       + head * params.seq_len * params.seq_len
                       + i * params.seq_len
                       + j;
        
        let value_idx = batch * params.num_heads * params.seq_len * params.head_dim
                      + head * params.seq_len * params.head_dim
                      + j * params.head_dim
                      + d;
        
        weighted_sum += weights[weight_idx] * value[value_idx];
    }
    
    // Store output
    let out_idx = batch * params.num_heads * params.seq_len * params.head_dim
                + head * params.seq_len * params.head_dim
                + i * params.head_dim
                + d;
    
    output[out_idx] = weighted_sum;
}
