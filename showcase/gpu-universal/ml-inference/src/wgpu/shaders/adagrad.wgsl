// AdaGrad Optimizer (Adaptive Gradient Algorithm)
// Adapts learning rate for each parameter based on historical gradients
//
// Update rule:
// accumulated_sq = accumulated_sq + gradient²
// weight = weight - learning_rate * gradient / (sqrt(accumulated_sq) + epsilon)
//
// Benefits: Automatically adapts learning rates for sparse features
// Used in: NLP, sparse data, Google's early deep learning systems

@group(0) @binding(0) var<storage, read> weights: array<f32>;
@group(0) @binding(1) var<storage, read> gradients: array<f32>;
@group(0) @binding(2) var<storage, read> accumulated_in: array<f32>;
@group(0) @binding(3) var<storage, read_write> weights_out: array<f32>;
@group(0) @binding(4) var<storage, read_write> accumulated_out: array<f32>;
@group(0) @binding(5) var<uniform> params: Params;

struct Params {
    learning_rate: f32,
    epsilon: f32,
    weight_decay: f32,
    _padding: u32,
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if idx >= arrayLength(&weights) {
        return;
    }

    let w = weights[idx];
    var g = gradients[idx];
    let acc = accumulated_in[idx];

    if params.weight_decay != 0.0 {
        g = g + params.weight_decay * w;
    }

    let acc_new = acc + g * g;
    accumulated_out[idx] = acc_new;

    let adaptive_lr = params.learning_rate / (sqrt(acc_new) + params.epsilon);
    weights_out[idx] = w - adaptive_lr * g;
}
