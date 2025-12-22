// Parallel Reduction OpenCL Kernel
// Sum all elements

__kernel void reduction(
    __global const uchar* input,
    __global ulong* output,
    const int n
) {
    size_t gid = get_global_id(0);
    size_t lid = get_local_id(0);
    size_t group_size = get_local_size(0);
    
    __local ulong local_sums[256];
    
    // Each work item sums its portion
    ulong sum = 0;
    for (size_t i = gid; i < n; i += get_global_size(0)) {
        sum += (ulong)input[i];
    }
    
    local_sums[lid] = sum;
    barrier(CLK_LOCAL_MEM_FENCE);
    
    // Reduce within work group
    for (size_t offset = group_size / 2; offset > 0; offset >>= 1) {
        if (lid < offset) {
            local_sums[lid] += local_sums[lid + offset];
        }
        barrier(CLK_LOCAL_MEM_FENCE);
    }
    
    // Write group result
    if (lid == 0) {
        output[get_group_id(0)] = local_sums[0];
    }
}

