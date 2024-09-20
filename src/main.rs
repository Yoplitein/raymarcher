#![allow(unused, non_snake_case, non_upper_case_globals)]

mod sdf;

use clap::{builder::ValueParserFactory, Parser};
use glam::{vec2, vec3, Vec2, Vec3, Vec3Swizzles, Vec4};
use image::{Pixel, Rgb, RgbImage, Rgba, RgbaImage};
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

    #[arg(short, long, default_value_t = false)]
    transparent: bool,
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
        
        let floor = |pt: Vec3| pt.y;
        let floor = floor.translate(Vec3::NEG_Y);
        
        let frame = sdf::sd_box(vec3(2.0, 1.0, 0.25));
        let doorway = sdf::sd_sphere(1.0).translate(vec3(0.0, -0.25, 0.0));
        let arch = doorway.difference(frame).translate(vec3(0.0, 0.0, -5.0));
        [
            Model::new(Rgb([225, 225, 255]), melty),
            Model::new(Rgb([0xff, 0x7f, 0x00]), floor),
            Model::new(Rgb([0x00, 0x7f, 0xff]), arch),
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
    let sampleSteps = if args.normals {
        (0 ..= 0)
    } else {
        (-1 ..= 1)
    }.map(|v| v as f32);
    let numSamples = sampleSteps.clone().count().pow(2);
    img.par_enumerate_pixels_mut().for_each(|(x, y, pixel)| {
        let mut samples = vec![Rgba([0, 0, 0, 0]); numSamples];
        let mut sampleIndex = 0;
        for sampleY in sampleSteps.clone().map(|v| v * subpixel.y) {
            let uvY = (y as f32 / (height - 1) as f32) - 0.5 + sampleY;
            for sampleX in sampleSteps.clone().map(|v| v * subpixel.x) {
                let uvX = (x as f32 / (width - 1) as f32) - 0.5 + sampleX;
                let ray = camera.get_ray(vec2(uvX, uvY));
                
                let nearest = Model::nearest_hit(&models, ray);
                
                if let Ok((RayHit { position: hitPosition, normal: hitNormal, .. }, mut color)) = nearest {
                    if args.normals {
                        let tforward = (camera.origin - hitPosition).normalize();
                        let tright = Vec3::Y.cross(tforward);
                        let tup = tright.cross(tforward);
                        let hitNormal = {
                            let y = hitNormal.dot(tforward);
                            let x = hitNormal.dot(tright);
                            let z = hitNormal.dot(tup);
                            1.0 - vec3(x, y, z)
                        };
                        let color = (hitNormal + 1.0) / 2.0;
                        let color = Vec4::from((color, 1.0));
                        let color = (color * 255.0).to_array().map(|v| v as u8);
                        samples[sampleIndex] = Rgba(color);
                    } else {
                        let shadowRay = Ray {
                            origin: hitPosition + hitNormal * 1e-5,
                            direction: lightDirection,
                        };
                        let mul = match Model::nearest_hit(&models, shadowRay) {
                            Ok(_) => 0.5,
                            Err(e) => e / 2.0 + 0.5,
                        };
                        color.apply(|v| (v as f32 * mul) as u8);
                        samples[sampleIndex] = color.to_rgba();
                    }
                } else {
                    if !args.transparent {
                        let skyboxColor = (((ray.direction + 1.0) / 2.0) * 255.0);
                        let skyboxColor = Vec4::from((skyboxColor, 255.0)).as_uvec4();
                        let color = skyboxColor.to_array().map(|v| v as u8);
                        samples[sampleIndex] = Rgba(color);
                    }
                }
                sampleIndex += 1;
            }
        }
        let sample = samples
            .into_iter()
            .map(|v| Vec4::from(v.0.map(|v| v as f32)))
            .fold(Vec4::ZERO, |l, r| l + r) / numSamples as f32;
        let mut sample = sample.to_array().map(|v| v as u8);
        *pixel = Rgba(sample);
    });
    
    img.save("out.png")?;
    Ok(())
}

#[derive(Clone, Debug)]
pub struct Camera {
    pub origin: Vec3,
    pub forward: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    pub aspect: f32,
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
pub struct RayHit {
    pub distance: f32,
    pub position: Vec3,
    pub normal: Vec3,
}

#[derive(Clone, Copy, Debug)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
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
    
    pub fn hit(&self, sdf: &dyn DistanceFn) -> Result<RayHit, f32> {
        let mut ray = *self;
        let mut totalDistance = 0.0;
        let mut occlusion = 1.0f32;
        for iteration in 0 .. 100 {
            let distance = sdf.eval(ray.origin);
            occlusion = occlusion.min(8.0 * distance / totalDistance);
            totalDistance += distance;

            if distance < 1e-5 {
                let normal = sdf.eval_normal(ray.origin);
                return Ok(RayHit {
                    distance: totalDistance,
                    position: ray.origin,
                    normal,
                });
            }
            if distance > 1e10 {
                break;
            }

            ray.advance(distance);
        }
        Err(occlusion)
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
    
    pub fn nearest_hit(models: &[Self], ray: Ray) -> Result<(RayHit, Rgb<u8>), f32> {
        let mut nearest = None;
        let mut minOcclusion = f32::INFINITY;
        for model in models {
            match ray.hit(model.sdf.as_ref()) {
                Ok(hit) => match nearest {
                    Some((RayHit { distance, .. }, _)) if distance < hit.distance => {}
                    _ => nearest = Some((hit, model.color)),
                },
                Err(occlusion) => minOcclusion = minOcclusion.min(occlusion),
            }
        }
        nearest.ok_or(minOcclusion)
    }
}
