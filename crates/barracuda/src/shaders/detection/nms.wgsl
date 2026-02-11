// NMS - Non-Maximum Suppression (Pure GPU)
// Multi-pass GPU implementation:
// Pass 1: Compute IoU matrix for all box pairs
// Pass 2: Mark suppressed boxes based on sorted order and IoU threshold
// Pass 3: Compact results to get keep indices

// ============================================================================
// Pass 1: IoU Matrix Computation
// ============================================================================

struct IoUParams {
    num_boxes: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

@group(0) @binding(0) var<storage, read> boxes: array<f32>;  // [num_boxes, 5]: [x1, y1, x2, y2, score]
@group(0) @binding(1) var<storage, read_write> iou_matrix: array<f32>;  // [num_boxes, num_boxes]
@group(0) @binding(2) var<uniform> iou_params: IoUParams;

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
    let union_area = area_a + area_b - intersection;
    
    if (union_area > 0.0) {
        return intersection / union_area;
    } else {
        return 0.0;
    }
}

@compute @workgroup_size(16, 16)
fn compute_iou_matrix(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let i = global_id.x;
    let j = global_id.y;
    
    if (i >= iou_params.num_boxes || j >= iou_params.num_boxes) {
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
    let idx = i * iou_params.num_boxes + j;
    iou_matrix[idx] = iou;
}

// ============================================================================
// Pass 2: Suppression Marking
// ============================================================================

struct SuppressParams {
    num_boxes: u32,
    iou_threshold: f32,
    _pad0: u32,
    _pad1: u32,
}

@group(0) @binding(0) var<storage, read> sorted_indices: array<u32>;  // Indices sorted by score (descending)
@group(0) @binding(1) var<storage, read> iou_data: array<f32>;  // Pre-computed IoU matrix
@group(0) @binding(2) var<storage, read_write> suppressed: array<u32>;  // 1 if suppressed, 0 if kept
@group(0) @binding(3) var<uniform> suppress_params: SuppressParams;

// Mark suppressed boxes based on sorted order
// Each thread processes one box and checks if it should be suppressed
@compute @workgroup_size(256)
fn mark_suppressed(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    
    if (idx >= suppress_params.num_boxes) {
        return;
    }
    
    let current_box_idx = sorted_indices[idx];
    
    // Check if this box should be suppressed by any higher-scoring box
    for (var i = 0u; i < idx; i++) {
        let higher_box_idx = sorted_indices[i];
        
        // If higher box is not suppressed, check IoU
        if (suppressed[higher_box_idx] == 0u) {
            let iou_idx = current_box_idx * suppress_params.num_boxes + higher_box_idx;
            let iou = iou_data[iou_idx];
            
            if (iou > suppress_params.iou_threshold) {
                suppressed[current_box_idx] = 1u;
                return;  // Early exit once suppressed
            }
        }
    }
}

// ============================================================================
// Pass 3: Compact Results
// ============================================================================

struct CompactParams {
    num_boxes: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

@group(0) @binding(0) var<storage, read> compact_sorted_indices: array<u32>;  // Indices sorted by score
@group(0) @binding(1) var<storage, read> compact_suppressed: array<u32>;  // Suppression flags
@group(0) @binding(2) var<storage, read_write> keep_indices: array<u32>;  // Output: indices to keep
@group(0) @binding(3) var<storage, read_write> keep_count: array<atomic<u32>>;  // Output: number of kept boxes (atomic)
@group(0) @binding(4) var<uniform> compact_params: CompactParams;

// Compact suppressed array to get keep indices
// Uses atomic counter for parallel compaction
@compute @workgroup_size(256)
fn compact_results(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    
    if (idx >= compact_params.num_boxes) {
        return;
    }
    
    let box_idx = compact_sorted_indices[idx];
    
    // If not suppressed, add to keep list using atomic counter
    if (compact_suppressed[box_idx] == 0u) {
        // Use atomic to get position in keep array
        let position = atomicAdd(&keep_count[0], 1u);
        if (position < compact_params.num_boxes) {
            keep_indices[position] = box_idx;
        }
    }
}
