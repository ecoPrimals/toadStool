// Dice Loss (F1 Loss)
// Measures overlap between predicted and target segmentation masks
//
// DiceLoss = 1 - (2 * |X ∩ Y|) / (|X| + |Y|)
// where X is prediction, Y is target
//
// Equivalent to F1 score loss. Range: [0, 1]
// 0 = perfect overlap, 1 = no overlap
//
// Used in: Medical image segmentation, semantic segmentation
// Benefits: Handles class imbalance, directly optimizes IoU-like metric

@group(0) @binding(0) var<storage, read> predictions: array<f32>;
@group(0) @binding(1) var<storage, read> targets: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;
@group(0) @binding(3) var<uniform> params: Params;

struct Params {
    smoothing: f32,          // Smoothing factor to avoid division by zero, typically 1.0
    reduction_mode: u32,  // 0=mean, 1=sum, 2=none
    batch_size: u32,
    elements_per_sample: u32,
}

// Shared memory for reduction within workgroup
var<workgroup> shared_intersection: array<f32, 256>;
var<workgroup> shared_pred_sum: array<f32, 256>;
var<workgroup> shared_target_sum: array<f32, 256>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>,
        @builtin(local_invocation_id) local_id: vec3<u32>,
        @builtin(workgroup_id) workgroup_id: vec3<u32>) {
    
    let batch_idx = workgroup_id.x;
    let local_idx = local_id.x;
    
    if batch_idx >= params.batch_size {
        return;
    }
    
    let base_idx = batch_idx * params.elements_per_sample;
    
    // Initialize shared memory
    shared_intersection[local_idx] = 0.0;
    shared_pred_sum[local_idx] = 0.0;
    shared_target_sum[local_idx] = 0.0;
    
    // Each thread processes multiple elements
    let threads_per_workgroup = 256u;
    let elements_per_thread = (params.elements_per_sample + threads_per_workgroup - 1u) / threads_per_workgroup;
    
    for (var i = 0u; i < elements_per_thread; i = i + 1u) {
        let idx = base_idx + local_idx + i * threads_per_workgroup;
        if idx < base_idx + params.elements_per_sample {
            let pred = predictions[idx];
            let targ = targets[idx];
            
            shared_intersection[local_idx] = shared_intersection[local_idx] + pred * targ;
            shared_pred_sum[local_idx] = shared_pred_sum[local_idx] + pred;
            shared_target_sum[local_idx] = shared_target_sum[local_idx] + targ;
        }
    }
    
    workgroupBarrier();
    
    // Reduction within workgroup
    for (var stride = 128u; stride > 0u; stride = stride / 2u) {
        if local_idx < stride {
            shared_intersection[local_idx] = shared_intersection[local_idx] + shared_intersection[local_idx + stride];
            shared_pred_sum[local_idx] = shared_pred_sum[local_idx] + shared_pred_sum[local_idx + stride];
            shared_target_sum[local_idx] = shared_target_sum[local_idx] + shared_target_sum[local_idx + stride];
        }
        workgroupBarrier();
    }
    
    // First thread computes Dice loss for this sample
    if local_idx == 0u {
        let intersection = shared_intersection[0];
        let pred_sum = shared_pred_sum[0];
        let target_sum = shared_target_sum[0];
        
        // Dice coefficient
        let dice = (2.0 * intersection + params.smoothing) / (pred_sum + target_sum + params.smoothing);
        
        // Dice loss = 1 - dice_coefficient
        output[batch_idx] = 1.0 - dice;
    }
}
