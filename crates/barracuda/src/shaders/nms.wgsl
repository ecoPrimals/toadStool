// NMS - Non-Maximum Suppression
// Computes IoU matrix for all box pairs (GPU-accelerated)
// Iterative selection happens on CPU due to sequential dependencies
//
// Input: boxes [num_boxes, 5] where each box is [x1, y1, x2, y2, score]
// Output: iou_matrix [num_boxes, num_boxes] with IoU values

struct Params {
    num_boxes: u32,
    iou_threshold: f32,
    _padding: [u32; 2],
}

@group(0) @binding(0) var<storage, read> boxes: array<f32>;  // [num_boxes, 5]
@group(0) @binding(1) var<storage, read_write> iou_matrix: array<f32>;  // [num_boxes, num_boxes]
@group(0) @binding(2) var<uniform> params: Params;

// Compute IoU between two boxes
fn compute_iou(box_a: vec4<f32>, box_b: vec4<f32>) -> f32 {
    // box format: [x1, y1, x2, y2]
    let x1 = max(box_a.x, box_b.x);
    let y1 = max(box_a.y, box_b.y);
    let x2 = min(box_a.z, box_b.z);
    let y2 = min(box_a.w, box_b.w);
    
    let intersection = max(0.0, x2 - x1) * max(0.0, y2 - y1);
    
    let area_a = (box_a.z - box_a.x) * (box_a.w - box_a.y);
    let area_b = (box_b.z - box_b.x) * (box_b.w - box_b.y);
    let union = area_a + area_b - intersection;
    
    if (union > 0.0) {
        return intersection / union;
    } else {
        return 0.0;
    }
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let i = global_id.x;
    let j = global_id.y;
    
    if (i >= params.num_boxes || j >= params.num_boxes) {
        return;
    }
    
    // Read boxes (each box is 5 floats: x1, y1, x2, y2, score)
    let box_a_idx = i * 5u;
    let box_b_idx = j * 5u;
    
    let box_a = vec4<f32>(
        boxes[box_a_idx],
        boxes[box_a_idx + 1u],
        boxes[box_a_idx + 2u],
        boxes[box_a_idx + 3u]
    );
    
    let box_b = vec4<f32>(
        boxes[box_b_idx],
        boxes[box_b_idx + 1u],
        boxes[box_b_idx + 2u],
        boxes[box_b_idx + 3u]
    );
    
    // Compute IoU
    let iou = compute_iou(box_a, box_b);
    
    // Store in IoU matrix (symmetric matrix)
    let idx = i * params.num_boxes + j;
    iou_matrix[idx] = iou;
}
