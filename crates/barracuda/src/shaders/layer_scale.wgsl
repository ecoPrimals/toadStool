// LayerScale - Per-layer learnable scaling
// Used in vision transformers (CaiT, LeViT) to stabilize training
//
// Algorithm: LayerScale(x) = gamma ⊙ x
// Element-wise multiplication with learnable per-channel parameters

struct Params {
    size: u32,
    _padding: [u32; 3],
}

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read> gamma: array<f32>;  // Per-channel scaling factors
@group(0) @binding(2) var<storage, read_write> output: array<f32>;
@group(0) @binding(3) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    
    if (idx >= params.size) {
        return;
    }
    
    // Element-wise scaling: output = input * gamma
    output[idx] = input[idx] * gamma[idx];
}
