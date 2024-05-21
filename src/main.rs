#![allow(unused, non_snake_case, non_upper_case_globals)]

mod sdf;

use glam::{vec2, vec3, Vec2, Vec3, Vec3Swizzles};
use image::{Rgb, RgbImage};
use rand::{thread_rng, Rng};
use anyhow::Result as AResult;

fn main() -> AResult<()> {
    let mut img = RgbImage::new(640, 360);
    let (width, height) = img.dimensions();
    
    let mut sdf = |point: Vec3| sdf::sd_sphere(point, 0.25);
    let lightDirection = Vec3::ONE.normalize();
    
    let mut camera = Camera::from_points(
        vec3(1.0, 0.0, 0.0),
        Vec3::ZERO,
        width as f32 / height as f32
    );
    dbg!(camera.up);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        let dx = (x as f32 / (width - 1) as f32) - 0.5;
        let dy = (y as f32 / (height - 1) as f32) - 0.5;
        let ray = camera.get_ray(vec2(dx, dy));
        if let Some(RayHit{ position }) = ray.hit(&mut sdf) {
            let shadowRay = Ray::from_points(position, position - lightDirection);
            if let Some(_) = shadowRay.hit(&mut sdf) {
                *pixel = Rgb([128, 0, 0]);
            } else {
                *pixel = Rgb([255, 0, 0]);
            }
        } else {
            let skyboxColor = (((ray.direction + 1.0) / 2.0) * 255.0).as_uvec3();
            *pixel = Rgb([skyboxColor.x as _, skyboxColor.y as _, skyboxColor.z as _]);
        }
    }
    
    img.save("out.png")?;
    Ok(())
}

#[derive(Clone, Debug)]
struct Camera {
    origin: Vec3,
    forward: Vec3,
    right: Vec3,
    up: Vec3,
    aspect: f32,
}

impl Camera {
    pub fn from_points(origin: Vec3, target: Vec3, aspectRatio: f32) -> Self {
        let forward = (target - origin).normalize();
        let right = Vec3::Y.cross(forward);
        let up = forward.cross(right);
        Self {
            origin,
            forward,
            right,
            up,
            aspect: aspectRatio,
        }
    }
    
    pub fn get_ray(&self, ndc: Vec2) -> Ray {
        let viewportCenter = self.origin + self.forward;
        let viewportPoint = viewportCenter +
            self.right * ndc.x * self.aspect +
            self.up * ndc.y;
        Ray::from_points(self.origin, viewportPoint)
    }
}

#[derive(Clone, Copy, Debug)]
struct RayHit {
    position: Vec3,
}

#[derive(Clone, Copy, Debug)]
struct Ray {
    origin: Vec3,
    direction: Vec3,
}

impl Ray {
    pub fn from_points(p1: Vec3, p2: Vec3) -> Self {
        Self {
            origin: p1,
            direction: (p2 - p1).normalize(),
        }
    }
    
    fn advance(&mut self, distance: f32) {
        self.origin += self.direction * distance    
    }
    
    pub fn hit(&self, sdf: &mut impl FnMut(Vec3) -> f32) -> Option<RayHit> {
        let mut ray = *self;
        let mut lastDistance = f32::INFINITY;
        for iteration in 0 .. 100 {
            let distance = sdf(ray.origin);
            if distance < 1e-2 {
                ray.advance(-1e-1);
                return Some(RayHit {
                    position: ray.origin,
                });
            }
            let delta = distance - lastDistance;
            if delta > 0.0 && iteration > 25 {
                break;
            }
            ray.advance(distance);
            lastDistance = distance;
        }
        None
    }
}
