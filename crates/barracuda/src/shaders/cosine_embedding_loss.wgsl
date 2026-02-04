// cosine_embedding_loss.wgsl - Cosine embedding loss
//
// Measures similarity between embeddings using cosine similarity
// Used in metric learning, face recognition, and contrastive learning
//
// Loss = { 1 - cos(x1, x2)           if label = 1
//        { max(0, cos(x1, x2) - margin) if label = -1

struct Params {
    size: u32,
    margin: f32,
    _padding: vec2<u32>,
}

@group(0) @binding(0) var<storage, read> input1: array<f32>;    // Embedding 1
@group(0) @binding(1) var<storage, read> input2: array<f32>;    // Embedding 2
@group(0) @binding(2) var<storage, read> label: array<f32>;     // 1.0 (similar) or -1.0 (dissimilar)
@group(0) @binding(3) var<storage, read_write> output: array<f32>; // Scalar loss
@group(0) @binding(4) var<uniform> params: Params;

var<workgroup> shared_dot: array<f32, 256>;
var<workgroup> shared_norm1: array<f32, 256>;
var<workgroup> shared_norm2: array<f32, 256>;

@compute @workgroup_size(256)
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>
) {
    let idx = global_id.x;
    let local_idx = local_id.x;
    
    // Compute local contributions
    var local_dot: f32 = 0.0;
    var local_norm1: f32 = 0.0;
    var local_norm2: f32 = 0.0;
    
    if (idx < params.size) {
        let v1 = input1[idx];
        let v2 = input2[idx];
        
        local_dot = v1 * v2;
        local_norm1 = v1 * v1;
        local_norm2 = v2 * v2;
    }
    
    shared_dot[local_idx] = local_dot;
    shared_norm1[local_idx] = local_norm1;
    shared_norm2[local_idx] = local_norm2;
    
    workgroupBarrier();
    
    // Reduction
    var stride = 128u;
    while (stride >= 1u) {
        if (local_idx < stride && local_idx + stride < 256u) {
            shared_dot[local_idx] += shared_dot[local_idx + stride];
            shared_norm1[local_idx] += shared_norm1[local_idx + stride];
            shared_norm2[local_idx] += shared_norm2[local_idx + stride];
        }
        workgroupBarrier();
        stride = stride / 2u;
    }
    
    // Compute loss
    if (local_idx == 0u) {
        let dot_product = shared_dot[0];
        let norm1 = sqrt(shared_norm1[0]);
        let norm2 = sqrt(shared_norm2[0]);
        
        // Cosine similarity
        let cos_sim = dot_product / (norm1 * norm2 + 1e-8);
        
        // Label: 1.0 = similar, -1.0 = dissimilar
        let y = label[0];
        
        var loss: f32;
        if (y > 0.0) {
            // Similar pair: minimize 1 - cos(x1, x2)
            loss = 1.0 - cos_sim;
        } else {
            // Dissimilar pair: maximize cos(x1, x2) up to margin
            loss = max(0.0, cos_sim - params.margin);
        }
        
        output[0] = loss;
    }
}
