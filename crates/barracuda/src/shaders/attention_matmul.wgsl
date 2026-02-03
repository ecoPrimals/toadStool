// Attention Matrix Multiplication: Compute QK^T scores
// Pass 1 of multi-pass attention implementation
//
// Computes: scores[i,j] = Q[i] · K[j] / sqrt(d_k)
// Where Q, K are [batch, heads, seq_len, head_dim]

struct AttentionParams {
    batch_size: u32,
    num_heads: u32,
    seq_len: u32,
    head_dim: u32,
}

@group(0) @binding(0) var<storage, read> query: array<f32>;     // [batch, heads, seq, head_dim]
@group(0) @binding(1) var<storage, read> key: array<f32>;       // [batch, heads, seq, head_dim]
@group(0) @binding(2) var<storage, read_write> scores: array<f32>; // [batch, heads, seq, seq]
@group(0) @binding(3) var<uniform> params: AttentionParams;

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let batch_head = global_id.z;
    let i = global_id.y; // query position
    let j = global_id.x; // key position
    
    if (i >= params.seq_len || j >= params.seq_len) {
        return;
    }
    
    let batch = batch_head / params.num_heads;
    let head = batch_head % params.num_heads;
    
    if (batch >= params.batch_size) {
        return;
    }
    
    // Compute dot product: Q[i] · K[j]
    var score = 0.0;
    for (var d = 0u; d < params.head_dim; d = d + 1u) {
        let q_idx = batch * params.num_heads * params.seq_len * params.head_dim
                  + head * params.seq_len * params.head_dim
                  + i * params.head_dim
                  + d;
        
        let k_idx = batch * params.num_heads * params.seq_len * params.head_dim
                  + head * params.seq_len * params.head_dim
                  + j * params.head_dim
                  + d;
        
        score += query[q_idx] * key[k_idx];
    }
    
    // Scale by sqrt(d_k) for numerical stability
    let scale = sqrt(f32(params.head_dim));
    score = score / scale;
    
    // Store score
    let score_idx = batch * params.num_heads * params.seq_len * params.seq_len
                  + head * params.seq_len * params.seq_len
                  + i * params.seq_len
                  + j;
    
    scores[score_idx] = score;
}
