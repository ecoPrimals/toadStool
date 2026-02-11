// Cast - Type conversion with multiple modes
//
// Supports f32↔i32 and f32↔u32 conversions via a mode parameter.
// Since WGSL storage buffers are typed, we use bitcast for reinterpretation
// and normal casting for value conversion.
//
// Modes:
//   0: f32 → f32 (identity / clamp)
//   1: f32 → i32 (truncate to integer, stored as f32 bit pattern)
//   2: f32 → u32 (clamp to non-negative, truncate)
//   3: i32 → f32 (interpret input bits as i32, convert to f32)
//   4: u32 → f32 (interpret input bits as u32, convert to f32)
//   5: f32 → f32 with clamp to [min_val, max_val]
//   6: f32 → bool (0.0 if input == 0, 1.0 otherwise)
//
// Cross-domain: data type conversion for mixed-precision training,
// quantization pipelines, boolean masks, physics simulations.

struct Params {
    total: u32,
    mode: u32,       // Cast mode (see above)
    min_val: f32,    // For clamp mode
    max_val: f32,    // For clamp mode
}

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;

    if (idx >= params.total) {
        return;
    }

    let x = input[idx];

    switch (params.mode) {
        case 0u: {
            // f32 → f32 identity
            output[idx] = x;
        }
        case 1u: {
            // f32 → i32 (truncate), store back as f32 representation
            let i = i32(x);
            output[idx] = f32(i);
        }
        case 2u: {
            // f32 → u32 (clamp non-negative, truncate)
            let u = u32(max(x, 0.0));
            output[idx] = f32(u);
        }
        case 3u: {
            // Reinterpret bits as i32, convert to f32
            let bits = bitcast<i32>(x);
            output[idx] = f32(bits);
        }
        case 4u: {
            // Reinterpret bits as u32, convert to f32
            let bits = bitcast<u32>(x);
            output[idx] = f32(bits);
        }
        case 5u: {
            // f32 → f32 with clamp
            output[idx] = clamp(x, params.min_val, params.max_val);
        }
        case 6u: {
            // f32 → bool (0.0 or 1.0)
            output[idx] = select(1.0, 0.0, x == 0.0);
        }
        default: {
            // Unknown mode: identity
            output[idx] = x;
        }
    }
}
