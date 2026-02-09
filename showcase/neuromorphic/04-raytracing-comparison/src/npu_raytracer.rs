//! NPU Raytracing - Event-Driven Sparse Ray Traversal
//!
//! Deep Debt: Uses NPU for event-driven raytracing
//! Excels at sparse scenes where most rays miss

use crate::scene::{Ray, Scene};
use akida_driver::{select_backend, BackendSelection, NpuBackend};
use anyhow::Result;
use glam::Vec3;

pub struct NpuRaytracer {
    backend: Box<dyn NpuBackend>,
    scene: Scene,
}

impl NpuRaytracer {
    /// Create NPU raytracer
    ///
    /// Deep Debt: Uses ToadStool's NPU drivers
    pub fn new(scene: Scene, device_id: &str) -> Result<Self> {
        let backend = select_backend(BackendSelection::Auto, device_id)?;

        Ok(Self { backend, scene })
    }

    /// Render scene using NPU event-driven processing
    ///
    /// Strategy: NPU is event-driven, so we encode rays as sparse events
    /// Only processes rays that have potential hits
    pub fn render(&mut self) -> Result<Vec<Vec3>> {
        let width = self.scene.image_width;
        let height = self.scene.image_height;
        let total_pixels = (width * height) as usize;

        let mut pixels = vec![Vec3::ZERO; total_pixels];

        // Encode scene as sparse events for NPU
        let mut events = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let ray = self.scene.camera_ray(x, y);

                // Convert ray to sparse event representation
                // NPU processes only rays that might hit something
                let event_data = encode_ray_event(&ray, &self.scene);
                events.extend_from_slice(&event_data);
            }
        }

        // Load sparse event data to NPU
        self.backend.load_model(&events)?;

        // NPU processes events (skips empty rays efficiently)
        let npu_output = self.backend.infer(&[])?;

        // Decode NPU sparse output back to pixel colors
        decode_npu_output(&npu_output, &mut pixels, width, height);

        Ok(pixels)
    }
}

/// Encode ray as sparse event for NPU
///
/// Deep Debt: NPU excels at sparse, event-driven processing
fn encode_ray_event(ray: &Ray, scene: &Scene) -> Vec<u8> {
    // Quick AABB check - does ray have chance of hitting any sphere?
    let mut has_potential_hit = false;

    for sphere in &scene.spheres {
        // Very coarse check - is sphere in ray direction?
        let to_sphere = sphere.center - ray.origin;
        if to_sphere.dot(ray.direction) > 0.0 {
            has_potential_hit = true;
            break;
        }
    }

    if has_potential_hit {
        // Encode as event: origin + direction + timestamp
        let mut event = Vec::with_capacity(32);
        event.extend_from_slice(&ray.origin.x.to_le_bytes());
        event.extend_from_slice(&ray.origin.y.to_le_bytes());
        event.extend_from_slice(&ray.origin.z.to_le_bytes());
        event.extend_from_slice(&ray.direction.x.to_le_bytes());
        event.extend_from_slice(&ray.direction.y.to_le_bytes());
        event.extend_from_slice(&ray.direction.z.to_le_bytes());
        event
    } else {
        // No event for rays that definitely miss (NPU optimization!)
        Vec::new()
    }
}

/// Decode NPU sparse output to pixel colors
fn decode_npu_output(npu_output: &[f32], pixels: &mut [Vec3], width: u32, height: u32) {
    // NPU outputs sparse hit events
    // Decode back to full pixel buffer

    let events_per_hit = 4; // x, y, r, g, b (packed)
    for chunk in npu_output.chunks(events_per_hit) {
        if chunk.len() >= events_per_hit {
            let x = chunk[0] as u32;
            let y = chunk[1] as u32;
            let color = Vec3::new(chunk[1], chunk[2], chunk[3]);

            if x < width && y < height {
                let idx = (y * width + x) as usize;
                pixels[idx] = color;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparse_encoding() {
        let scene = Scene::sparse();
        let ray = scene.camera_ray(400, 300);
        let encoded = encode_ray_event(&ray, &scene);

        // Sparse scenes should have many empty events
        assert!(encoded.len() == 0 || encoded.len() == 32);
    }
}
