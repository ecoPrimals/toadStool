// Scaled Dot-Product Attention Shader
//
// Implements: Attention(Q, K, V) = softmax(Q·K^T / √d_k)·V
//
// Input shapes:
// - Q (Query): [batch, seq_len, d_k]
// - K (Key): [batch, seq_len, d_k]
// - V (Value): [batch, seq_len, d_v]
// - Mask: [batch, seq_len, seq_len] (optional, 1.0 = keep, 0.0 = mask)
//
// Output shapes:
// - Output: [batch, seq_len, d_v]
// - Attention weights: [batch, seq_len, seq_len]
//
// Algorithm:
// 1. Compute scores = Q·K^T / √d_k
// 2. Apply mask (scores = scores + (1-mask) * -1e9)
// 3. Compute attention weights = softmax(scores)
// 4. Compute output = attention_weights·V

struct AttentionConfig {
    batch: u32,
    seq_len: u32,
    d_k: u32,
    d_v: u32,
}

@group(0) @binding(0) var<storage, read> query: array<f32>;    // [batch, seq_len, d_k]
@group(0) @binding(1) var<storage, read> key: array<f32>;      // [batch, seq_len, d_k]
@group(0) @binding(2) var<storage, read> value: array<f32>;    // [batch, seq_len, d_v]
@group(0) @binding(3) var<storage, read> mask: array<f32>;     // [batch, seq_len, seq_len]
@group(0) @binding(4) var<storage, read_write> output: array<f32>;          // [batch, seq_len, d_v]
@group(0) @binding(5) var<storage, read_write> attention_weights: array<f32>; // [batch, seq_len, seq_len]
@group(0) @binding(6) var<uniform> config: AttentionConfig;

// Constants
const MASK_VALUE: f32 = -1e9;  // Large negative value for masking
const EPSILON: f32 = 1e-8;     // For numerical stability

// Helper: Access 3D tensor stored as 1D array
fn get_3d_index(b: u32, i: u32, j: u32, dim1: u32, dim2: u32) -> u32 {
    return b * dim1 * dim2 + i * dim2 + j;
}

// Softmax over a row of scores
fn softmax_row(batch: u32, row: u32, scores: ptr<function, array<f32, 1024>>, seq_len: u32) -> array<f32, 1024> {
    var result: array<f32, 1024>;
    
    // Find max for numerical stability
    var max_val: f32 = -1e38;
    for (var col: u32 = 0u; col < seq_len; col++) {
        max_val = max(max_val, scores[col]);
    }
    
    // Compute exp(x - max) and sum
    var sum: f32 = 0.0;
    for (var col: u32 = 0u; col < seq_len; col++) {
        let exp_val = exp(scores[col] - max_val);
        result[col] = exp_val;
        sum += exp_val;
    }
    
    // Normalize
    for (var col: u32 = 0u; col < seq_len; col++) {
        result[col] /= (sum + EPSILON);
    }
    
    return result;
}

@compute @workgroup_size(256)
fn scaled_dot_product_attention(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let total_positions = config.batch * config.seq_len;
    let idx = global_id.x;
    
    if (idx >= total_positions) {
        return;
    }
    
    // Decode batch and position
    let batch = idx / config.seq_len;
    let query_pos = idx % config.seq_len;
    
    let sqrt_d_k = sqrt(f32(config.d_k));
    
    // Step 1 & 2: Compute scores = Q·K^T / √d_k and apply mask
    // Each thread processes one query position against all key positions
    var scores: array<f32, 1024>;  // Max seq_len = 1024
    
    for (var key_pos: u32 = 0u; key_pos < config.seq_len; key_pos++) {
        // Dot product between query[query_pos] and key[key_pos]
        var dot: f32 = 0.0;
        for (var k: u32 = 0u; k < config.d_k; k++) {
            let q_idx = get_3d_index(batch, query_pos, k, config.seq_len, config.d_k);
            let k_idx = get_3d_index(batch, key_pos, k, config.seq_len, config.d_k);
            dot += query[q_idx] * key[k_idx];
        }
        
        // Scale by √d_k
        var score = dot / sqrt_d_k;
        
        // Apply mask
        let mask_idx = get_3d_index(batch, query_pos, key_pos, config.seq_len, config.seq_len);
        let mask_val = mask[mask_idx];
        if (mask_val < 0.5) {  // 0.0 = masked
            score += MASK_VALUE;
        }
        
        scores[key_pos] = score;
    }
    
    // Step 3: Softmax over scores to get attention weights
    let weights = softmax_row(batch, query_pos, &scores, config.seq_len);
    
    // Store attention weights
    for (var key_pos: u32 = 0u; key_pos < config.seq_len; key_pos++) {
        let weight_idx = get_3d_index(batch, query_pos, key_pos, config.seq_len, config.seq_len);
        attention_weights[weight_idx] = weights[key_pos];
    }
    
    // Step 4: Compute output = attention_weights·V
    for (var v_dim: u32 = 0u; v_dim < config.d_v; v_dim++) {
        var weighted_sum: f32 = 0.0;
        for (var key_pos: u32 = 0u; key_pos < config.seq_len; key_pos++) {
            let v_idx = get_3d_index(batch, key_pos, v_dim, config.seq_len, config.d_v);
            weighted_sum += weights[key_pos] * value[v_idx];
        }
        
        let out_idx = get_3d_index(batch, query_pos, v_dim, config.seq_len, config.d_v);
        output[out_idx] = weighted_sum;
    }
}
