// WindowFunction - Various windowing functions for signal processing
// Implements Hann, Hamming, Blackman, Bartlett, and Rectangular windows

struct Params {
    length: u32,
    window_type: u32,  // 0=Hann, 1=Hamming, 2=Blackman, 3=Bartlett, 4=Rectangular
}

@group(0) @binding(0) var<storage, read_write> output: array<f32>;
@group(0) @binding(1) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    
    if (idx >= params.length) {
        return;
    }
    
    let n = f32(idx);
    let length_f = f32(params.length);
    let pi = 3.14159265358979323846;
    
    var val: f32;
    
    switch (params.window_type) {
        case 0u: { // Hann
            val = 0.5 * (1.0 - cos(2.0 * pi * n / (length_f - 1.0)));
        }
        case 1u: { // Hamming
            val = 0.54 - 0.46 * cos(2.0 * pi * n / (length_f - 1.0));
        }
        case 2u: { // Blackman
            val = 0.42 - 0.5 * cos(2.0 * pi * n / (length_f - 1.0)) +
                  0.08 * cos(4.0 * pi * n / (length_f - 1.0));
        }
        case 3u: { // Bartlett
            val = 1.0 - abs(2.0 * n / (length_f - 1.0) - 1.0);
        }
        case 4u: { // Rectangular
            val = 1.0;
        }
        default: {
            val = 1.0;
        }
    }
    
    output[idx] = val;
}
