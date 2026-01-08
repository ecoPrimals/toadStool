//! Vulkan Compute Shaders for Neural Network Inference
//! 
//! These GLSL compute shaders will be compiled to SPIR-V
//! Compatible with Vulkan 1.2+, works on NVIDIA, AMD, Intel

// ============================================================================
// Matrix Multiplication: C = A * B
// ============================================================================
// Shader: matrix_multiply.comp
#version 450

layout(local_size_x = 16, local_size_y = 16) in;

layout(set = 0, binding = 0) readonly buffer MatrixA {
    float data[];
} a;

layout(set = 0, binding = 1) readonly buffer MatrixB {
    float data[];
} b;

layout(set = 0, binding = 2) writeonly buffer MatrixC {
    float data[];
} c;

layout(push_constant) uniform PushConstants {
    uint M;  // Rows in A
    uint K;  // Cols in A, Rows in B
    uint N;  // Cols in B
} dims;

void main() {
    uint row = gl_GlobalInvocationID.x;
    uint col = gl_GlobalInvocationID.y;
    
    if (row >= dims.M || col >= dims.N) {
        return;
    }
    
    float sum = 0.0;
    for (uint k = 0; k < dims.K; k++) {
        float a_val = a.data[row * dims.K + k];
        float b_val = b.data[k * dims.N + col];
        sum += a_val * b_val;
    }
    
    c.data[row * dims.N + col] = sum;
}

// ============================================================================
// ReLU Activation: y = max(0, x)
// ============================================================================
// Shader: relu.comp
#version 450

layout(local_size_x = 256) in;

layout(set = 0, binding = 0) buffer Data {
    float values[];
} data;

layout(push_constant) uniform PushConstants {
    uint size;
} constants;

void main() {
    uint idx = gl_GlobalInvocationID.x;
    
    if (idx >= constants.size) {
        return;
    }
    
    data.values[idx] = max(0.0, data.values[idx]);
}

// ============================================================================
// Softmax: exp(x_i) / sum(exp(x_j))
// ============================================================================
// Shader: softmax.comp
#version 450

layout(local_size_x = 256) in;

layout(set = 0, binding = 0) buffer Data {
    float values[];
} data;

layout(set = 0, binding = 1) buffer MaxBuffer {
    float max_val;
} max_buf;

layout(set = 0, binding = 2) buffer SumBuffer {
    float sum_val;
} sum_buf;

layout(push_constant) uniform PushConstants {
    uint size;
    uint stage;  // 0: find max, 1: exp and sum, 2: normalize
} constants;

// Shared memory for reduction
shared float shared_data[256];

void main() {
    uint idx = gl_GlobalInvocationID.x;
    uint local_idx = gl_LocalInvocationID.x;
    
    if (constants.stage == 0) {
        // Stage 1: Find maximum value
        float local_max = -1e38;
        if (idx < constants.size) {
            local_max = data.values[idx];
        }
        
        shared_data[local_idx] = local_max;
        barrier();
        
        // Reduction to find max
        for (uint stride = 128; stride > 0; stride >>= 1) {
            if (local_idx < stride && local_idx + stride < 256) {
                shared_data[local_idx] = max(shared_data[local_idx], 
                                             shared_data[local_idx + stride]);
            }
            barrier();
        }
        
        if (local_idx == 0) {
            atomicMax(max_buf.max_val, shared_data[0]);
        }
    }
    else if (constants.stage == 1) {
        // Stage 2: Subtract max, compute exp, and sum
        float local_sum = 0.0;
        if (idx < constants.size) {
            float val = exp(data.values[idx] - max_buf.max_val);
            data.values[idx] = val;
            local_sum = val;
        }
        
        shared_data[local_idx] = local_sum;
        barrier();
        
        // Reduction to find sum
        for (uint stride = 128; stride > 0; stride >>= 1) {
            if (local_idx < stride && local_idx + stride < 256) {
                shared_data[local_idx] += shared_data[local_idx + stride];
            }
            barrier();
        }
        
        if (local_idx == 0) {
            atomicAdd(sum_buf.sum_val, shared_data[0]);
        }
    }
    else if (constants.stage == 2) {
        // Stage 3: Normalize by sum
        if (idx < constants.size) {
            data.values[idx] /= sum_buf.sum_val;
        }
    }
}

