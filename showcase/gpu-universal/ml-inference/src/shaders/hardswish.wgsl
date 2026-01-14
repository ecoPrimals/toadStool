// HardSwish Activation
// Efficient approximation of Swish/SiLU for mobile and edge devices
//
// HardSwish(x) = x * ReLU6(x + 3) / 6
// where ReLU6(x) = min(max(x, 0), 6)
//
// Equivalent to: x * min(max(x + 3, 0), 6) / 6
//
// Used in: MobileNetV3, EfficientNet-Lite
// Benefits: Faster than Swish (no sigmoid), mobile-friendly

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if idx >= arrayLength(&input) {
        return;
    }
    
    let x = input[idx];
    
    // HardSwish computation: x * ReLU6(x + 3) / 6
    let relu6 = min(max(x + 3.0, 0.0), 6.0);
    output[idx] = x * relu6 / 6.0;
}
