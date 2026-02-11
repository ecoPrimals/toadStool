// Cross Entropy Loss
// loss = -sum(targets * log(predictions))
// Simplified version

@group(0) @binding(0) var<storage, read> predictions: array<f32>;
@group(0) @binding(1) var<storage, read> targets: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;

var<workgroup> shared_loss: f32;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>, @builtin(local_invocation_id) local_id: vec3<u32>) {
    let size = arrayLength(&predictions);
    let tid = local_id.x;
    
    if (tid == 0u) {
        var sum = 0.0;
        for (var i = 0u; i < size; i = i + 1u) {
            let pred = max(predictions[i], 1e-7); // Avoid log(0)
            sum = sum + targets[i] * log(pred);
        }
        shared_loss = -sum / f32(size);
    }
    workgroupBarrier();
    
    if (tid == 0u) {
        output[0] = shared_loss;
    }
}
