// Embedding operation - Lookup table for token embeddings
// Maps integer indices to dense vectors
// Essential for NLP: word embeddings, positional encodings, token representations

struct EmbeddingParams {
    batch_size: u32,      // Number of sequences
    seq_length: u32,      // Sequence length
    embedding_dim: u32,   // Dimension of embeddings
    vocab_size: u32,      // Size of vocabulary
}

@group(0) @binding(0) var<storage, read> indices: array<u32>;  // Input indices
@group(0) @binding(1) var<storage, read> weight: array<f32>;   // Embedding table [vocab_size, embedding_dim]
@group(0) @binding(2) var<storage, read_write> output: array<f32>; // Output embeddings
@group(0) @binding(3) var<uniform> params: EmbeddingParams;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let total_elements = params.batch_size * params.seq_length;
    
    if (idx >= total_elements) {
        return;
    }
    
    // Get the token index for this position
    let token_idx = indices[idx];
    
    // Bounds check for token index
    if (token_idx >= params.vocab_size) {
        // Out of vocabulary - zero embedding
        for (var d = 0u; d < params.embedding_dim; d = d + 1u) {
            output[idx * params.embedding_dim + d] = 0.0;
        }
        return;
    }
    
    // Look up embedding vector from weight table
    let embedding_start = token_idx * params.embedding_dim;
    
    // Copy embedding to output
    for (var d = 0u; d < params.embedding_dim; d = d + 1u) {
        output[idx * params.embedding_dim + d] = weight[embedding_start + d];
    }
}
