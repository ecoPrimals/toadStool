// RMSprop Optimizer
// Adaptive learning rate method, addresses AdaGrad's diminishing learning rates
//
// sq_avg = alpha * sq_avg + (1 - alpha) * gradient²
// weight = weight - lr * gradient / (sqrt(sq_avg) + epsilon)

@group(0) @binding(0) var<storage, read> weights: array<f32>;
@group(0) @binding(1) var<storage, read> gradients: array<f32>;
@group(0) @binding(2) var<storage, read> sq_avg_in: array<f32>;
@group(0) @binding(3) var<storage, read_write> weights_out: array<f32>;
@group(0) @binding(4) var<storage, read_write> sq_avg_out: array<f32>;
@group(0) @binding(5) var<uniform> params: Params;

struct Params {
    learning_rate: f32,
    alpha: f32,
    epsilon: f32,
    weight_decay: f32,
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if idx >= arrayLength(&weights) {
        return;
    }

    let w = weights[idx];
    var g = gradients[idx];
    let sq = sq_avg_in[idx];

    if params.weight_decay != 0.0 {
        g = g + params.weight_decay * w;
    }

    let sq_new = params.alpha * sq + (1.0 - params.alpha) * g * g;
    sq_avg_out[idx] = sq_new;

    let adaptive_lr = params.learning_rate / (sqrt(sq_new) + params.epsilon);
    weights_out[idx] = w - adaptive_lr * g;
}
