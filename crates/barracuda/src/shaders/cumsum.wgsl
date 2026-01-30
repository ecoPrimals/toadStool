// Cumulative sum - running sum
// Simplified: sequential implementation

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let size = arrayLength(&input);
    
    var sum = 0.0;
    for (var i = 0u; i < size; i = i + 1u) {
        sum = sum + input[i];
        output[i] = sum;
    }
}
