// NAdam (Nesterov-accelerated Adam) Optimizer
// Combines Adam with Nesterov momentum for faster convergence
//
// Update rule: Same as Adam but with Nesterov-style lookahead for m_hat
// m_hat_nadam = mu_t+1 * m_hat + (1 - mu_t) * g  where mu_t = beta1 * (1 - 0.5 * 0.96^(t/250))
//
// Used in: Complex optimization landscapes, often outperforms Adam

@group(0) @binding(0) var<storage, read> weights: array<f32>;
@group(0) @binding(1) var<storage, read> gradients: array<f32>;
@group(0) @binding(2) var<storage, read> m_in: array<f32>;
@group(0) @binding(3) var<storage, read> v_in: array<f32>;
@group(0) @binding(4) var<storage, read_write> weights_out: array<f32>;
@group(0) @binding(5) var<storage, read_write> m_out: array<f32>;
@group(0) @binding(6) var<storage, read_write> v_out: array<f32>;
@group(0) @binding(7) var<uniform> params: Params;

struct Params {
    learning_rate: f32,
    beta1: f32,
    beta2: f32,
    epsilon: f32,
    weight_decay: f32,
    step: u32,
    _padding: vec2<u32>,
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if idx >= arrayLength(&weights) {
        return;
    }

    let w = weights[idx];
    var g = gradients[idx];
    var m = m_in[idx];
    var v = v_in[idx];

    if params.weight_decay != 0.0 {
        g = g + params.weight_decay * w;
    }

    m = params.beta1 * m + (1.0 - params.beta1) * g;
    v = params.beta2 * v + (1.0 - params.beta2) * g * g;

    let t = f32(params.step);
    let beta1_t = pow(params.beta1, t);
    let beta2_t = pow(params.beta2, t);

    let m_hat = m / (1.0 - beta1_t);
    let v_hat = v / (1.0 - beta2_t);

    let mu = params.beta1 * (1.0 - 0.5 * pow(0.96, t / 250.0));
    let mu_next = params.beta1 * (1.0 - 0.5 * pow(0.96, (t + 1.0) / 250.0));
    let m_nadam = mu_next * m_hat + (1.0 - mu) * g;

    let update = params.learning_rate * m_nadam / (sqrt(v_hat) + params.epsilon);
    weights_out[idx] = w - update;
    m_out[idx] = m;
    v_out[idx] = v;
}
