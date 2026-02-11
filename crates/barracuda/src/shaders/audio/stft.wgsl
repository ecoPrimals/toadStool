// STFT - Short-Time Fourier Transform
// Converts time-domain signal to time-frequency representation
// Input: signal [length], window [n_fft]
// Output: Complex STFT [num_frames, n_fft/2+1] as [real, imag, real, imag, ...]

struct Params {
    signal_length: u32,
    n_fft: u32,
    hop_length: u32,
    num_frames: u32,
    bins_per_frame: u32,  // n_fft / 2 + 1
}

@group(0) @binding(0) var<storage, read> signal: array<f32>;
@group(0) @binding(1) var<storage, read> window: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;  // [real, imag, real, imag, ...]
@group(0) @binding(3) var<uniform> params: Params;

@compute @workgroup_size(64, 4)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let frame_idx = global_id.x;
    let k = global_id.y;  // Frequency bin index
    
    if (frame_idx >= params.num_frames || k >= params.bins_per_frame) {
        return;
    }
    
    let start = frame_idx * params.hop_length;
    var real: f32 = 0.0;
    var imag: f32 = 0.0;
    
    let pi = 3.14159265358979323846;
    let angle_base = -2.0 * pi * f32(k) / f32(params.n_fft);
    
    // Compute DFT for frequency bin k
    for (var n: u32 = 0u; n < params.n_fft; n = n + 1u) {
        let signal_idx = start + n;
        if (signal_idx < params.signal_length) {
            let windowed = signal[signal_idx] * window[n];
            let angle = angle_base * f32(n);
            real = real + windowed * cos(angle);
            imag = imag + windowed * sin(angle);
        }
    }
    
    // Write complex pair: [real, imag]
    let output_idx = (frame_idx * params.bins_per_frame + k) * 2u;
    output[output_idx] = real;
    output[output_idx + 1u] = imag;
}
