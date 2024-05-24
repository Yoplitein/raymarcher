#![allow(unused, non_snake_case, non_upper_case_globals)]

mod sdf;

use clap::{builder::ValueParserFactory, Parser};
use glam::{vec2, vec3, Vec2, Vec3, Vec3Swizzles};
use image::{Pixel, Rgb, RgbImage};
use rand::{thread_rng, Rng};
use anyhow::{anyhow, Result as AResult};
use rayon::iter::ParallelIterator;
use sdf::{DistanceFn, DistanceFnCombinators};

#[derive(Debug, Parser)]
struct Args {
    cameraPos: String,
    
    #[arg(default_value_t = String::from("0,0,0"))]
    cameraTarget: String,
    
    #[arg(long)]
    height: f32,
    
    #[arg(short, long, default_value_t = false)]
    normals: bool,
}

fn main() -> AResult<()> {
    let mut args = Args::parse();
    let cameraPos: Vec3 = {
        let elems = args.cameraPos
            .split(",")
            .map(|v| v.parse::<f32>())
            .map(|v| v.map_err(|e| anyhow!("{e:?}")))
            .collect::<AResult<Vec<_>>>()?;
        let elems: [f32; 3] = elems
            .try_into()
            .map_err(|_| anyhow!("not exactly three elements"))?;
        Vec3::from(elems)
    };
    let cameraTarget: Vec3 = {
        let elems = args.cameraTarget
            .split(",")
            .map(|v| v.parse::<f32>())
            .map(|v| v.map_err(|e| anyhow!("{e:?}")))
            .collect::<AResult<Vec<_>>>()?;
        let elems: [f32; 3] = elems
            .try_into()
            .map_err(|_| anyhow!("not exactly three elements"))?;
        Vec3::from(elems)
    };
    
    let mut img = RgbImage::new(640, 360);
    let (width, height) = img.dimensions();
    
    #[cfg(none)]
    let sdf = {
        let bigSphere = sdf::sd_sphere(0.25);
        let smallSphere = sdf::sd_sphere(0.1);
        
        let sdf = bigSphere.union(smallSphere.clone().translate(vec3(0.25, 0.0, 0.0)));
        let sdf = sdf.union(smallSphere.clone().translate(vec3(-0.25, 0.0, 0.0)));
        let sdf = sdf.union(smallSphere.clone().translate(vec3(0.0, 0.25, 0.0)));
        let sdf = sdf.union(smallSphere.clone().translate(vec3(0.0, -0.25, 0.0)));
        let sdf = sdf.union(smallSphere.clone().translate(vec3(0.0, 0.0, 0.25)));
        let sdf = sdf.union(smallSphere.clone().translate(vec3(0.0, 0.0, -0.25)));
        sdf
    };
    let x: &dyn DistanceFn;
    let models = {
        /* let bx = sdf::sd_box(vec3(2.0, 0.5, 0.5)).translate(Vec3::X);
        let by = sdf::sd_box(vec3(0.5, 2.0, 0.5)).translate(Vec3::Y);
        let bz = sdf::sd_box(vec3(0.5, 0.5, 2.0)).translate(Vec3::Z);
        
        [
            Model::new(Rgb([255, 0, 0]), bx),
            Model::new(Rgb([0, 255, 0]), by),
            Model::new(Rgb([0, 0, 255]), bz),
        ] */
        
        let cube = sdf::sd_box(Vec3::ONE);
        let sphere = sdf::sd_sphere(1.0).translate(Vec3::ZERO.with_y(args.height));
        let melty = cube.smooth_union(1.0, sphere);
        [
            Model::new(Rgb([225, 225, 255]), melty)
        ]
    };
    let lightDirection = Vec3::ONE.normalize();
    
    let camera = Camera::from_points(
        // vec3(5.0, 2.5, 5.0),
        // Vec3::ZERO,
        cameraPos,
        cameraTarget,
        width as f32 / height as f32
    );
    let subpixel = vec2(
        1.0 / (width - 1) as f32,
        1.0 / (height - 1) as f32,
    ) / 4.0;
    let sampleSteps = (-1 ..= 1).map(|v| v as f32);
    let numSamples = sampleSteps.clone().count().pow(2);
    img.par_enumerate_pixels_mut().for_each(|(x, y, pixel)| {
        let mut samples = vec![Rgb([0, 0, 0]); numSamples];
        let mut sampleIndex = 0;
        for sampleY in sampleSteps.clone().map(|v| v * subpixel.y) {
            let uvY = (y as f32 / (height - 1) as f32) - 0.5 + sampleY;
            for sampleX in sampleSteps.clone().map(|v| v * subpixel.x) {
                let uvX = (x as f32 / (width - 1) as f32) - 0.5 + sampleX;
                let ray = camera.get_ray(vec2(uvX, uvY));
                
                let mut nearest = None;
                for model in &models {
                    if let Some(hit) = ray.hit(model.sdf.as_ref()) {
                        match nearest {
                            Some((RayHit { distance, .. }, _)) if distance < hit.distance => {}
                            _ => nearest = Some((hit, model.color)),
                        }
                    }
                }
                
                if let Some((RayHit { normal, .. }, mut color)) = nearest {
                    if args.normals {
                        let color = (normal + 1.0) / 2.0;
                        let color = (color * 255.0).to_array().map(|v| v as u8);
                        samples[sampleIndex] = Rgb(color);
                    } else {
                        let shadow = normal.dot(lightDirection);
                        if shadow < 0.0 {
                            let shadow = (1.0 - (shadow * 4.0).abs()).clamp(0.0, 1.0);
                            color.apply(|v|
                                ((v as f32 / 255.0) * shadow * 255.0) as u8
                            );
                        }
                        samples[sampleIndex] = color;
                    }
                } else {
                    let skyboxColor = (((ray.direction + 1.0) / 2.0) * 255.0).as_uvec3();
                    let color = skyboxColor.to_array().map(|v| v as u8);
                    samples[sampleIndex] = Rgb(color);
                }
                sampleIndex += 1;
            }
        }
        let sample = samples
            .into_iter()
            .map(|v| Vec3::from(v.0.map(|v| v as f32)))
            .fold(Vec3::ZERO, |l, r| l + r) / numSamples as f32;
        let sample = sample.to_array().map(|v| v as u8);
        *pixel = Rgb(sample);
    });
    
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
        let up = right.cross(forward);
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
    distance: f32,
    position: Vec3,
    normal: Vec3,
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
    
    pub fn hit(&self, sdf: &dyn DistanceFn) -> Option<RayHit> {
        let mut ray = *self;
        let mut lastDistance = f32::INFINITY;
        let mut totalDistance = 0.0;
        for iteration in 0 .. 100 {
            let distance = sdf.eval(ray.origin);
            
            if distance < lastDistance && distance < 1e-2 {
                ray.advance(-1e-1); // FIXME: shadow ray hack
                let normal = sdf.eval_normal(ray.origin);
                return Some(RayHit {
                    distance: totalDistance,
                    position: ray.origin,
                    normal,
                });
            }
            if distance > lastDistance && iteration > 25 {
                break;
            }
            
            ray.advance(distance);
            totalDistance += distance;
            lastDistance = distance;
        }
        None
    }
}

pub struct Model {
	color: Rgb<u8>,
	sdf: Box<dyn DistanceFn>,
}

impl Model {
    pub fn new<Func: 'static + DistanceFn>(color: Rgb<u8>, sdf: Func) -> Self {
        Self {
            color,
            sdf: Box::new(sdf),
        }
    }
}
