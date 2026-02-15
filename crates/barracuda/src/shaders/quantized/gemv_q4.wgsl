// Q4_0 GEMV (General Matrix-Vector Multiply) Shader
//
// Computes y = A @ x where A is Q4_0 quantized.
// Performs on-the-fly dequantization during computation.
//
// This is more efficient than dequant-then-multiply for inference:
//   - Reduces memory bandwidth (read 4-bit, not 32-bit)
//   - Uses GPU's parallelism for dequantization
//   - Ideal for LLM inference where weight access dominates
//
// Deep Debt Compliance:
// - ✅ Pure WGSL (no vendor extensions)
// - ✅ Workgroup size 256 (universal)
// - ✅ Partial sum reduction in registers

struct GemvParams {
    m: u32,          // Rows in A (output dimension)
    k: u32,          // Columns in A (input dimension)
    k_blocks: u32,   // Number of Q4 blocks per row (k / 32)
    _pad: u32,
}

// Q4_0 blocks: [scale:f16][data:16bytes]
@group(0) @binding(0) var<storage, read> a_quant: array<u32>;
// Input vector x (f32)
@group(0) @binding(1) var<storage, read> x: array<f32>;
// Output vector y (f32)
@group(0) @binding(2) var<storage, read_write> y: array<f32>;
@group(0) @binding(3) var<uniform> params: GemvParams;

const BLOCK_SIZE: u32 = 32u;
const BYTES_PER_BLOCK: u32 = 18u;
const WORKGROUP_SIZE: u32 = 256u;

// Shared memory for input vector caching
var<workgroup> shared_x: array<f32, 256>;

@compute @workgroup_size(256)
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) wg_id: vec3<u32>
) {
    let row = global_id.x;
    
    if (row >= params.m) {
        return;
    }
    
    var sum: f32 = 0.0;
    
    // Process blocks in chunks to use shared memory
    let num_chunks = (params.k + WORKGROUP_SIZE - 1u) / WORKGROUP_SIZE;
    
    for (var chunk = 0u; chunk < num_chunks; chunk++) {
        // Load x values into shared memory
        let x_idx = chunk * WORKGROUP_SIZE + local_id.x;
        if (x_idx < params.k) {
            shared_x[local_id.x] = x[x_idx];
        } else {
            shared_x[local_id.x] = 0.0;
        }
        workgroupBarrier();
        
        // Process Q4 blocks that overlap with this chunk
        let start_block = chunk * (WORKGROUP_SIZE / BLOCK_SIZE);
        let end_block = min(start_block + (WORKGROUP_SIZE / BLOCK_SIZE), params.k_blocks);
        
        for (var block = start_block; block < end_block; block++) {
            // Calculate byte offset for this block in row
            let byte_offset = (row * params.k_blocks + block) * BYTES_PER_BLOCK;
            let u32_offset = byte_offset / 4u;
            
            // Read scale (f16)
            let raw0 = a_quant[u32_offset];
            let scale = unpack2x16float(raw0 & 0xFFFFu).x;
            
            // Read quantized data (16 bytes = 4 u32s, starting at byte 2)
            let data_u32_offset = u32_offset; // Data starts at byte 2, we handle alignment
            let d0_raw = a_quant[data_u32_offset] >> 16u | (a_quant[data_u32_offset + 1u] << 16u);
            let d1_raw = a_quant[data_u32_offset + 1u] >> 16u | (a_quant[data_u32_offset + 2u] << 16u);
            let d2_raw = a_quant[data_u32_offset + 2u] >> 16u | (a_quant[data_u32_offset + 3u] << 16u);
            let d3_raw = a_quant[data_u32_offset + 3u] >> 16u | (a_quant[data_u32_offset + 4u] << 16u);
            
            // Compute dot product with on-the-fly dequantization
            let block_start = block * BLOCK_SIZE;
            let local_start = block_start - chunk * WORKGROUP_SIZE;
            
            // Process 32 elements (8 bytes = 16 nibble pairs)
            sum += dequant_dot_q4(d0_raw, scale, &shared_x, local_start, 0u);
            sum += dequant_dot_q4(d1_raw, scale, &shared_x, local_start, 8u);
            sum += dequant_dot_q4(d2_raw, scale, &shared_x, local_start, 16u);
            sum += dequant_dot_q4(d3_raw, scale, &shared_x, local_start, 24u);
        }
        
        workgroupBarrier();
    }
    
    y[row] = sum;
}

fn dequant_dot_q4(packed: u32, scale: f32, x_shared: ptr<workgroup, array<f32, 256>>, base: u32, offset: u32) -> f32 {
    // Each u32 contains 8 nibbles = 8 quantized values
    var sum: f32 = 0.0;
    
    for (var i = 0u; i < 8u; i++) {
        let nibble = (packed >> (i * 4u)) & 0xFu;
        let q = i32(nibble) - 8;
        let val = scale * f32(q);
        let x_idx = base + offset + i;
        if (x_idx < 256u) {
            sum += val * (*x_shared)[x_idx];
        }
    }
    
    return sum;
}
