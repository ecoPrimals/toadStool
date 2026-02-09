// IoU Loss - Intersection over Union loss
// Direct optimization of IoU metric
// Used in segmentation and object detection
//
// Algorithm:
// IoU = (intersection + smooth_val) / (union + smooth_val)
// IoU Loss = 1 - IoU
// where intersection = sum(predictions * targets)
//       union = sum(predictions + targets - predictions * targets)

struct Params {
    size: u32,
    smooth_val: f32,
    _pad1: u32,
    _pad2: u32,
}

@group(0) @binding(0) var<storage, read> predictions: array<f32>;
@group(0) @binding(1) var<storage, read> targets: array<f32>;
@group(0) @binding(2) var<storage, read_write> intersection_buffer: array<atomic<i32>>; // [1] - atomic reduction
@group(0) @binding(3) var<storage, read_write> union_buffer: array<atomic<i32>>; // [1] - atomic reduction
@group(0) @binding(4) var<storage, read_write> output: array<f32>; // [1] - final loss
@group(0) @binding(5) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    
    if (idx >= params.size) {
        return;
    }
    
    let pred = predictions[idx];
    let targ = targets[idx];
    
    // Compute intersection and union for this element
    let intersection = pred * targ;
    let union_val = pred + targ - intersection;
    
    // Atomic add to reduction buffers
    atomicAdd(&intersection_buffer[0], bitcast<i32>(intersection));
    atomicAdd(&union_buffer[0], bitcast<i32>(union_val));
}

// Second pass: compute final loss
@compute @workgroup_size(1)
fn compute_loss(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let intersection = bitcast<f32>(atomicLoad(&intersection_buffer[0]));
    let union_val = bitcast<f32>(atomicLoad(&union_buffer[0]));
    
    // IoU = (intersection + smooth_val) / (union_val + smooth_val)
    let iou = (intersection + params.smooth_val) / (union_val + params.smooth_val);
    
    // IoU Loss = 1 - IoU
    output[0] = 1.0 - iou;
}
