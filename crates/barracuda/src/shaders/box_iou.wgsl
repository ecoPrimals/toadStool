// box_iou.wgsl - Intersection over Union for bounding boxes
//
// Computes IoU between pairs of bounding boxes
// Used in object detection (YOLO, Faster R-CNN, etc.)
//
// IoU = Area(intersection) / Area(union)

struct Params {
    num_boxes_a: u32,
    num_boxes_b: u32,
    box_format: u32,  // 0 = xyxy, 1 = xywh, 2 = cxcywh
    _padding: u32,
}

@group(0) @binding(0) var<storage, read> boxes_a: array<f32>;      // [num_boxes_a, 4]
@group(0) @binding(1) var<storage, read> boxes_b: array<f32>;      // [num_boxes_b, 4]
@group(0) @binding(2) var<storage, read_write> output: array<f32>; // [num_boxes_a, num_boxes_b]
@group(0) @binding(3) var<uniform> params: Params;

// Convert box to (x1, y1, x2, y2) format
fn box_to_xyxy(box: vec4<f32>, format: u32) -> vec4<f32> {
    if (format == 0u) {
        // Already xyxy
        return box;
    } else if (format == 1u) {
        // xywh -> xyxy
        return vec4<f32>(
            box.x,
            box.y,
            box.x + box.z,
            box.y + box.w
        );
    } else {
        // cxcywh -> xyxy
        let half_w = box.z / 2.0;
        let half_h = box.w / 2.0;
        return vec4<f32>(
            box.x - half_w,
            box.y - half_h,
            box.x + half_w,
            box.y + half_h
        );
    }
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let i = global_id.x;
    let j = global_id.y;
    
    if (i >= params.num_boxes_a || j >= params.num_boxes_b) {
        return;
    }
    
    // Load boxes
    let box_a = vec4<f32>(
        boxes_a[i * 4u + 0u],
        boxes_a[i * 4u + 1u],
        boxes_a[i * 4u + 2u],
        boxes_a[i * 4u + 3u]
    );
    
    let box_b = vec4<f32>(
        boxes_b[j * 4u + 0u],
        boxes_b[j * 4u + 1u],
        boxes_b[j * 4u + 2u],
        boxes_b[j * 4u + 3u]
    );
    
    // Convert to xyxy format
    let a = box_to_xyxy(box_a, params.box_format);
    let b = box_to_xyxy(box_b, params.box_format);
    
    // Compute intersection
    let x1 = max(a.x, b.x);
    let y1 = max(a.y, b.y);
    let x2 = min(a.z, b.z);
    let y2 = min(a.w, b.w);
    
    let intersection_w = max(0.0, x2 - x1);
    let intersection_h = max(0.0, y2 - y1);
    let intersection_area = intersection_w * intersection_h;
    
    // Compute areas
    let area_a = (a.z - a.x) * (a.w - a.y);
    let area_b = (b.z - b.x) * (b.w - b.y);
    
    // Compute union
    let union_area = area_a + area_b - intersection_area;
    
    // Compute IoU
    let iou = select(0.0, intersection_area / union_area, union_area > 1e-6);
    
    output[i * params.num_boxes_b + j] = iou;
}
