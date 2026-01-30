// Embedding - Lookup table operation
// output[i] = embeddings[indices[i]]
// Simplified version: 1D embeddings

struct EmbeddingParams {
    embedding_dim: u32,
    _padding: vec3<u32>,
}

@group(0) @binding(0) var<storage, read> embeddings: array<f32>;
@group(0) @binding(1) var<storage, read> indices: array<u32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;
@group(0) @binding(3) var<uniform> params: EmbeddingParams;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let num_indices = arrayLength(&indices);
    
    if (idx >= num_indices) {
        return;
    }
    
    let embedding_idx = indices[idx];
    let offset = embedding_idx * params.embedding_dim;
    
    // Copy embedding vector to output
    for (var i = 0u; i < params.embedding_dim; i = i + 1u) {
        output[idx * params.embedding_dim + i] = embeddings[offset + i];
    }
}
