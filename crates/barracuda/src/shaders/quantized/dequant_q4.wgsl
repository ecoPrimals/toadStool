// Q4_0 Dequantization Shader
// 
// Dequantizes 4-bit quantized weights to f32 for inference.
// Each block contains 32 elements packed in 18 bytes:
//   - 2 bytes: scale factor (f16)
//   - 16 bytes: 32 x 4-bit quantized values
//
// Deep Debt Compliance:
// - ✅ Pure WGSL (no vendor extensions)
// - ✅ Workgroup size 256 (universal)
// - ✅ Coalesced memory access patterns

struct Q4Block {
    scale: f32,
    // 16 bytes packed as 4 u32s
    data0: u32,
    data1: u32,
    data2: u32,
    data3: u32,
}

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
    
    // Each Q4_0 block is 18 bytes = 4.5 u32s, but we read 5 u32s and handle alignment
    // Block layout: [scale_f16:2][data:16] = 18 bytes
    // We read as u32s: offset = block_idx * 18 / 4
    let byte_offset = block_idx * 18u;
    let u32_offset = byte_offset / 4u;
    
    // Read the block data
    let raw0 = input[u32_offset];
    let raw1 = input[u32_offset + 1u];
    let raw2 = input[u32_offset + 2u];
    let raw3 = input[u32_offset + 3u];
    let raw4 = input[u32_offset + 4u];
    
    // Extract scale (first 2 bytes as f16)
    let byte_mod = byte_offset % 4u;
    var scale_bits: u32;
    if (byte_mod == 0u) {
        scale_bits = raw0 & 0xFFFFu;
    } else if (byte_mod == 2u) {
        scale_bits = raw0 >> 16u;
    } else {
        // Handle unaligned case
        scale_bits = ((raw0 >> (byte_mod * 8u)) | (raw1 << (32u - byte_mod * 8u))) & 0xFFFFu;
    }
    
    // Convert f16 to f32
    let scale = unpack2x16float(scale_bits).x;
    
    // Output base index
    let out_base = block_idx * BLOCK_SIZE;
    
    // Extract and dequantize 32 values (16 bytes = 32 nibbles)
    // Data starts at byte_offset + 2
    let data_byte_offset = byte_offset + 2u;
    let data_u32_offset = data_byte_offset / 4u;
    let data_byte_mod = data_byte_offset % 4u;
    
    // Read data words
    let d0 = input[data_u32_offset];
    let d1 = input[data_u32_offset + 1u];
    let d2 = input[data_u32_offset + 2u];
    let d3 = input[data_u32_offset + 3u];
    let d4 = input[data_u32_offset + 4u];
    
    // Extract and dequantize each nibble
    for (var i = 0u; i < 16u; i++) {
        let byte_idx = i;
        let word_idx = byte_idx / 4u;
        let local_byte = byte_idx % 4u;
        
        var byte_val: u32;
        if (word_idx == 0u) {
            byte_val = (d0 >> (local_byte * 8u + data_byte_mod * 8u)) & 0xFFu;
        } else if (word_idx == 1u) {
            byte_val = (d1 >> (local_byte * 8u)) & 0xFFu;
        } else if (word_idx == 2u) {
            byte_val = (d2 >> (local_byte * 8u)) & 0xFFu;
        } else {
            byte_val = (d3 >> (local_byte * 8u)) & 0xFFu;
        }
        
        // Each byte contains 2 nibbles
        let q0 = i32(byte_val & 0xFu) - 8;
        let q1 = i32((byte_val >> 4u) & 0xFu) - 8;
        
        let idx0 = out_base + i * 2u;
        let idx1 = out_base + i * 2u + 1u;
        
        output[idx0] = scale * f32(q0);
        output[idx1] = scale * f32(q1);
    }
}
