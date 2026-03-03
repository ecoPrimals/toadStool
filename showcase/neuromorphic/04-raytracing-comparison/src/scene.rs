// SPDX-License-Identifier: AGPL-3.0-or-later
//! Ray tracing scene representation
//!
//! Deep Debt: Simple, efficient scene for NPU vs GPU comparison

use glam::Vec3;

#[derive(Debug, Clone)]
pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
    pub color: Vec3,
}

#[derive(Debug, Clone)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self {
            origin,
            direction: direction.normalize(),
        }
    }

    pub fn at(&self, t: f32) -> Vec3 {
        self.origin + self.direction * t
    }
}

#[derive(Debug, Clone)]
pub struct Scene {
    pub spheres: Vec<Sphere>,
    pub image_width: u32,
    pub image_height: u32,
}

impl Scene {
    /// Create a sparse scene (few objects, many empty rays)
    /// NPU should excel here
    pub fn sparse() -> Self {
        Self {
            spheres: vec![
                Sphere {
                    center: Vec3::new(0.0, 0.0, -5.0),
                    radius: 1.0,
                    color: Vec3::new(1.0, 0.0, 0.0),
                },
                Sphere {
                    center: Vec3::new(2.0, 0.0, -5.0),
                    radius: 0.5,
                    color: Vec3::new(0.0, 1.0, 0.0),
                },
            ],
            image_width: 800,
            image_height: 600,
        }
    }

    /// Create a dense scene (many objects, most rays hit)
    /// GPU should excel here
    pub fn dense() -> Self {
        let mut spheres = Vec::new();

        // Create a grid of spheres
        for x in -5..5 {
            for y in -5..5 {
                for z in -10..-5 {
                    spheres.push(Sphere {
                        center: Vec3::new(x as f32, y as f32, z as f32),
                        radius: 0.3,
                        color: Vec3::new(
                            (x + 5) as f32 / 10.0,
                            (y + 5) as f32 / 10.0,
                            (z + 10) as f32 / 5.0,
                        ),
                    });
                }
            }
        }

        Self {
            spheres,
            image_width: 800,
            image_height: 600,
        }
    }

    /// Check ray-sphere intersection
    pub fn intersect_sphere(&self, ray: &Ray, sphere: &Sphere) -> Option<f32> {
        let oc = ray.origin - sphere.center;
        let a = ray.direction.dot(ray.direction);
        let b = 2.0 * oc.dot(ray.direction);
        let c = oc.dot(oc) - sphere.radius * sphere.radius;
        let discriminant = b * b - 4.0 * a * c;

        if discriminant < 0.0 {
            None
        } else {
            let t = (-b - discriminant.sqrt()) / (2.0 * a);
            if t > 0.0 {
                Some(t)
            } else {
                None
            }
        }
    }

    /// Trace a ray through the scene
    pub fn trace_ray(&self, ray: &Ray) -> Vec3 {
        let mut closest_t = f32::MAX;
        let mut hit_color = Vec3::new(0.1, 0.1, 0.1); // Background

        for sphere in &self.spheres {
            if let Some(t) = self.intersect_sphere(ray, sphere) {
                if t < closest_t {
                    closest_t = t;
                    hit_color = sphere.color;
                }
            }
        }

        hit_color
    }

    /// Generate camera ray for pixel
    pub fn camera_ray(&self, x: u32, y: u32) -> Ray {
        let aspect_ratio = self.image_width as f32 / self.image_height as f32;
        let fov = 60.0_f32.to_radians();
        let scale = (fov / 2.0).tan();

        let px = (2.0 * (x as f32 + 0.5) / self.image_width as f32 - 1.0) * aspect_ratio * scale;
        let py = (1.0 - 2.0 * (y as f32 + 0.5) / self.image_height as f32) * scale;

        Ray::new(Vec3::ZERO, Vec3::new(px, py, -1.0))
    }
}
