// Q8_0 Dequantization Shader
//
// Dequantizes 8-bit quantized weights to f32 for inference.
// Each block contains 32 elements packed in 34 bytes:
//   - 2 bytes: scale factor (f16)
//   - 32 bytes: 32 x 8-bit quantized values (signed)
//
// Deep Debt Compliance:
// - ✅ Pure WGSL (no vendor extensions)
// - ✅ Workgroup size 256 (universal)
// - ✅ Coalesced memory access patterns

struct DequantParams {
    num_blocks: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

@group(0) @binding(0) var<storage, read> input: array<u32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: DequantParams;

const BLOCK_SIZE: u32 = 32u;
const WORKGROUP_SIZE: u32 = 256u;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let block_idx = global_id.x;
    
    if (block_idx >= params.num_blocks) {
        return;
    }
    
    // Q8_0 block is 34 bytes = 8.5 u32s
    // Layout: [scale_f16:2][data:32]
    let byte_offset = block_idx * 34u;
    let u32_offset = byte_offset / 4u;
    let byte_mod = byte_offset % 4u;
    
    // Read scale (first 2 bytes as f16)
    let raw0 = input[u32_offset];
    let raw1 = input[u32_offset + 1u];
    
    var scale_bits: u32;
    if (byte_mod == 0u) {
        scale_bits = raw0 & 0xFFFFu;
    } else if (byte_mod == 2u) {
        scale_bits = raw0 >> 16u;
    } else {
        scale_bits = ((raw0 >> (byte_mod * 8u)) | (raw1 << (32u - byte_mod * 8u))) & 0xFFFFu;
    }
    
    let scale = unpack2x16float(scale_bits).x;
    
    // Output base index
    let out_base = block_idx * BLOCK_SIZE;
    
    // Data starts at byte_offset + 2
    let data_byte_offset = byte_offset + 2u;
    let data_u32_offset = data_byte_offset / 4u;
    
    // Read 8 u32s (32 bytes of quantized data)
    let d0 = input[data_u32_offset];
    let d1 = input[data_u32_offset + 1u];
    let d2 = input[data_u32_offset + 2u];
    let d3 = input[data_u32_offset + 3u];
    let d4 = input[data_u32_offset + 4u];
    let d5 = input[data_u32_offset + 5u];
    let d6 = input[data_u32_offset + 6u];
    let d7 = input[data_u32_offset + 7u];
    
    // Dequantize all 32 values
    // Each u32 contains 4 signed bytes
    dequant_u32(d0, scale, out_base, 0u, &output);
    dequant_u32(d1, scale, out_base, 4u, &output);
    dequant_u32(d2, scale, out_base, 8u, &output);
    dequant_u32(d3, scale, out_base, 12u, &output);
    dequant_u32(d4, scale, out_base, 16u, &output);
    dequant_u32(d5, scale, out_base, 20u, &output);
    dequant_u32(d6, scale, out_base, 24u, &output);
    dequant_u32(d7, scale, out_base, 28u, &output);
}

fn dequant_u32(packed: u32, scale: f32, out_base: u32, offset: u32, out: ptr<storage, array<f32>, read_write>) {
    // Extract 4 signed bytes and dequantize
    let b0 = i32((packed >> 0u) & 0xFFu);
    let b1 = i32((packed >> 8u) & 0xFFu);
    let b2 = i32((packed >> 16u) & 0xFFu);
    let b3 = i32((packed >> 24u) & 0xFFu);
    
    // Convert unsigned to signed (values are stored as signed i8)
    let s0 = select(b0, b0 - 256, b0 > 127);
    let s1 = select(b1, b1 - 256, b1 > 127);
    let s2 = select(b2, b2 - 256, b2 > 127);
    let s3 = select(b3, b3 - 256, b3 > 127);
    
    (*out)[out_base + offset + 0u] = scale * f32(s0);
    (*out)[out_base + offset + 1u] = scale * f32(s1);
    (*out)[out_base + offset + 2u] = scale * f32(s2);
    (*out)[out_base + offset + 3u] = scale * f32(s3);
}
