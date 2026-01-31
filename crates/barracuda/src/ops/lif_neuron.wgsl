// Leaky Integrate-and-Fire (LIF) neuron shader
// Simulates spiking neuron dynamics with leak, integration, threshold, and reset

struct Params {
    n: u32,
    tau: f32,
    threshold: f32,
    reset: f32,
    dt: f32,
}

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> potential: array<f32>;
@group(0) @binding(2) var<storage, read_write> spikes: array<f32>;
@group(0) @binding(3) var<uniform> params: Params;

@compute @workgroup_size(1)
fn lif_neuron(@builtin(global_invocation_id) gid: vec3<u32>) {
    // Single neuron simulation (sequential over time)
    var v = params.reset;  // Initial potential
    
    for (var t = 0u; t < params.n; t = t + 1u) {
        // Leaky integration: dv/dt = (-v + I) / tau
        // Euler method: v_new = v + dt * (-v + I) / tau
        let leak = -v / params.tau;
        let current = input[t] / params.tau;
        v = v + params.dt * (leak + current);
        
        // Only clamp lower bound to prevent going below reset
        v = max(v, params.reset);
        
        // Check threshold
        if (v >= params.threshold) {
            spikes[t] = 1.0;  // Spike occurred
            v = params.reset;  // Reset potential
        } else {
            spikes[t] = 0.0;  // No spike
        }
        
        // Store potential
        potential[t] = v;
    }
}
