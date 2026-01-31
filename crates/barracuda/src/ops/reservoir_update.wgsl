// Reservoir state update shader
// Implements: x(t+1) = (1-α)·x(t) + α·tanh(W_in·u(t) + W_res·x(t))

struct Params {
    n: u32,
    m: u32,
    leak_rate: f32,
}

@group(0) @binding(0) var<storage, read> state: array<f32>;
@group(0) @binding(1) var<storage, read> input: array<f32>;
@group(0) @binding(2) var<storage, read> w_in: array<f32>;
@group(0) @binding(3) var<storage, read> w_res: array<f32>;
@group(0) @binding(4) var<storage, read_write> output: array<f32>;
@group(0) @binding(5) var<uniform> params: Params;

@compute @workgroup_size(256)
fn reservoir_update(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= params.n) {
        return;
    }
    
    // Compute W_in · u(t) - input contribution
    var input_sum = 0.0;
    for (var j = 0u; j < params.m; j = j + 1u) {
        let w_idx = i * params.m + j;
        input_sum = input_sum + w_in[w_idx] * input[j];
    }
    
    // Compute W_res · x(t) - recurrent contribution
    var recurrent_sum = 0.0;
    for (var j = 0u; j < params.n; j = j + 1u) {
        let w_idx = i * params.n + j;
        recurrent_sum = recurrent_sum + w_res[w_idx] * state[j];
    }
    
    // Apply nonlinearity: tanh(W_in·u + W_res·x)
    let activation = tanh(input_sum + recurrent_sum);
    
    // Leaky integration: x(t+1) = (1-α)·x(t) + α·f(...)
    let new_state = (1.0 - params.leak_rate) * state[i] + params.leak_rate * activation;
    
    output[i] = new_state;
}
