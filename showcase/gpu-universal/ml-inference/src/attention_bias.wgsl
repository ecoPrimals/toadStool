// Attention Bias Shader
//
// Adds bias to attention scores: output[i,j] = scores[i,j] + bias[i,j]
//
// Supports:
// - Positional bias
// - ALiBi (Attention with Linear Biases)
// - Relative position bias
// - Custom bias patterns

struct BiasConfig {
    batch: u32,
    seq_len: u32,
}

@group(0) @binding(0) var<storage, read> scores: array<f32>;  // [batch, seq_len, seq_len]
@group(0) @binding(1) var<storage, read> bias: array<f32>;    // [seq_len, seq_len] or [batch, seq_len, seq_len]
@group(0) @binding(2) var<storage, read_write> output: array<f32>;  // [batch, seq_len, seq_len]
@group(0) @binding(3) var<uniform> config: BiasConfig;

@compute @workgroup_size(256)
fn attention_bias(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let total_elements = config.batch * config.seq_len * config.seq_len;
    let idx = global_id.x;
    
    if (idx >= total_elements) {
        return;
    }
    
    // Decode indices
    let batch = idx / (config.seq_len * config.seq_len);
    let remainder = idx % (config.seq_len * config.seq_len);
    let i = remainder / config.seq_len;
    let j = remainder % config.seq_len;
    
    // Determine bias index (shared across batch or per-batch)
    let bias_idx = if (arrayLength(&bias) == config.seq_len * config.seq_len) {
        // Shared bias
        i * config.seq_len + j
    } else {
        // Per-batch bias
        idx
    };
    
    // Add bias to scores
    output[idx] = scores[idx] + bias[bias_idx];
}
