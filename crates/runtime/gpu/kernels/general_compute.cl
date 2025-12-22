// General Compute OpenCL Kernel
// Simple element-wise operation for testing

__kernel void general_compute(
    __global const uchar* input,
    __global uchar* output
) {
    size_t gid = get_global_id(0);
    
    // Simple operation: increment each byte
    output[gid] = input[gid] + 1;
}

