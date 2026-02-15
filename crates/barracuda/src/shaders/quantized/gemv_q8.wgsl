// Q8_0 GEMV (General Matrix-Vector Multiply) Shader
//
// Computes y = A @ x where A is Q8_0 quantized.
// Higher precision than Q4 with similar bandwidth benefits.
//
// Deep Debt Compliance:
// - ✅ Pure WGSL (no vendor extensions)
// - ✅ Workgroup size 256 (universal)
// - ✅ On-the-fly dequantization

struct GemvParams {
    m: u32,          // Rows in A (output dimension)
    k: u32,          // Columns in A (input dimension)
    k_blocks: u32,   // Number of Q8 blocks per row (k / 32)
    _pad: u32,
}

// Q8_0 blocks: [scale:f16][data:32bytes]
@group(0) @binding(0) var<storage, read> a_quant: array<u32>;
// Input vector x (f32)
@group(0) @binding(1) var<storage, read> x: array<f32>;
// Output vector y (f32)
@group(0) @binding(2) var<storage, read_write> y: array<f32>;
@group(0) @binding(3) var<uniform> params: GemvParams;

const BLOCK_SIZE: u32 = 32u;
const BYTES_PER_BLOCK: u32 = 34u;
const WORKGROUP_SIZE: u32 = 256u;

var<workgroup> shared_x: array<f32, 256>;

@compute @workgroup_size(256)
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>
) {
    let row = global_id.x;
    
    if (row >= params.m) {
        return;
    }
    
    var sum: f32 = 0.0;
    
    let num_chunks = (params.k + WORKGROUP_SIZE - 1u) / WORKGROUP_SIZE;
    
    for (var chunk = 0u; chunk < num_chunks; chunk++) {
        // Cache x in shared memory
        let x_idx = chunk * WORKGROUP_SIZE + local_id.x;
        if (x_idx < params.k) {
            shared_x[local_id.x] = x[x_idx];
        } else {
            shared_x[local_id.x] = 0.0;
        }
        workgroupBarrier();
        
        // Process Q8 blocks
        let start_block = chunk * (WORKGROUP_SIZE / BLOCK_SIZE);
        let end_block = min(start_block + (WORKGROUP_SIZE / BLOCK_SIZE), params.k_blocks);
        
        for (var block = start_block; block < end_block; block++) {
            let byte_offset = (row * params.k_blocks + block) * BYTES_PER_BLOCK;
            let u32_offset = byte_offset / 4u;
            
            // Read scale
            let raw0 = a_quant[u32_offset];
            let scale = unpack2x16float(raw0 & 0xFFFFu).x;
            
            // Read quantized data (32 bytes = 8 u32s)
            // Data starts at byte 2 (offset 0.5 u32)
            let block_start = block * BLOCK_SIZE;
            let local_start = block_start - chunk * WORKGROUP_SIZE;
            
            // Read aligned with offset handling
            let d_offset = u32_offset;
            
            // Process each u32 of quantized data (4 i8 values each)
            for (var w = 0u; w < 8u; w++) {
                let word_byte_offset = 2u + w * 4u;
                let word_u32_offset = d_offset + (word_byte_offset / 4u);
                let shift = (word_byte_offset % 4u) * 8u;
                
                let raw_lo = a_quant[word_u32_offset];
                let raw_hi = a_quant[word_u32_offset + 1u];
                let packed = (raw_lo >> shift) | (raw_hi << (32u - shift));
                
                sum += dequant_dot_q8(packed, scale, &shared_x, local_start, w * 4u);
            }
        }
        
        workgroupBarrier();
    }
    
    y[row] = sum;
}

fn dequant_dot_q8(packed: u32, scale: f32, x_shared: ptr<workgroup, array<f32, 256>>, base: u32, offset: u32) -> f32 {
    var sum: f32 = 0.0;
    
    // Extract 4 signed bytes
    let b0 = i32((packed >> 0u) & 0xFFu);
    let b1 = i32((packed >> 8u) & 0xFFu);
    let b2 = i32((packed >> 16u) & 0xFFu);
    let b3 = i32((packed >> 24u) & 0xFFu);
    
    // Convert to signed
    let s0 = select(b0, b0 - 256, b0 > 127);
    let s1 = select(b1, b1 - 256, b1 > 127);
    let s2 = select(b2, b2 - 256, b2 > 127);
    let s3 = select(b3, b3 - 256, b3 > 127);
    
    let idx0 = base + offset;
    let idx1 = base + offset + 1u;
    let idx2 = base + offset + 2u;
    let idx3 = base + offset + 3u;
    
    if (idx0 < 256u) { sum += scale * f32(s0) * (*x_shared)[idx0]; }
    if (idx1 < 256u) { sum += scale * f32(s1) * (*x_shared)[idx1]; }
    if (idx2 < 256u) { sum += scale * f32(s2) * (*x_shared)[idx2]; }
    if (idx3 < 256u) { sum += scale * f32(s3) * (*x_shared)[idx3]; }
    
    return sum;
}
