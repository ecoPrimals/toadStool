// Adam Optimizer: Adaptive moment estimation
// Combines momentum (SGD) with RMSprop-style adaptive learning rates
//
// m = beta1 * m + (1 - beta1) * g
// v = beta2 * v + (1 - beta2) * g²
// m_hat = m / (1 - beta1^t), v_hat = v / (1 - beta2^t)
// param = param - lr * m_hat / (sqrt(v_hat) + epsilon)

@group(0) @binding(0) var<storage, read> gradients: array<f32>;
@group(0) @binding(1) var<storage, read_write> params: array<f32>;
@group(0) @binding(2) var<storage, read_write> m: array<f32>;
@group(0) @binding(3) var<storage, read_write> v: array<f32>;
@group(0) @binding(4) var<uniform> adam_params: Params;

struct Params {
    num_params: u32,
    learning_rate: f32,
    beta1: f32,
    beta2: f32,
    epsilon: f32,
    weight_decay: f32,
    step: u32,
    _padding: u32,
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let gid = global_id.x;

    if gid >= adam_params.num_params {
        return;
    }

    var grad = gradients[gid];
    let param = params[gid];
    var m_val = m[gid];
    var v_val = v[gid];

    if adam_params.weight_decay > 0.0 {
        grad = grad + adam_params.weight_decay * param;
    }

    m_val = adam_params.beta1 * m_val + (1.0 - adam_params.beta1) * grad;
    v_val = adam_params.beta2 * v_val + (1.0 - adam_params.beta2) * grad * grad;

    let m_hat = m_val / (1.0 - pow(adam_params.beta1, f32(adam_params.step)));
    let v_hat = v_val / (1.0 - pow(adam_params.beta2, f32(adam_params.step)));

    let update = adam_params.learning_rate * m_hat / (sqrt(v_hat) + adam_params.epsilon);
    params[gid] = param - update;
    m[gid] = m_val;
    v[gid] = v_val;
}
