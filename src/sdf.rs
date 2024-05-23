use glam::{Affine3A, Vec3};
use image::Rgb;

pub trait DistanceFn: Send + Sync {
	fn eval(&self, point: Vec3) -> f32;
}

impl<Func: Send + Sync + Fn(Vec3) -> f32> DistanceFn for Func {
	fn eval(&self, point: Vec3) -> f32 {
		self(point)
	}
}

pub trait DistanceFnCombinators: Sized + DistanceFn {
	fn transform(self, transform: Affine3A) -> impl DistanceFn {
		let transform = transform.inverse();
		move |point: Vec3| self.eval(transform.transform_point3(point))
	}
	
	fn translate(self, translation: Vec3) -> impl DistanceFn {
		self.transform(Affine3A::from_translation(translation))
	}
	
	fn scale(self, scale: f32) -> impl DistanceFn {
		move |point: Vec3| self.eval(point / scale) * scale
	}
	
	fn union(self, mut other: impl DistanceFn) -> impl DistanceFn {
		move |point: Vec3| self.eval(point).min(other.eval(point))
	}
	
	fn intersection(self, mut other: impl DistanceFn) -> impl DistanceFn {
		move |point: Vec3| self.eval(point).max(other.eval(point))
	}
	
	fn difference(self, mut other: impl DistanceFn) -> impl DistanceFn {
		move |point: Vec3| (-self.eval(point)).max(other.eval(point))
	}
}

impl<T: DistanceFn> DistanceFnCombinators for T {
}

pub fn sd_sphere(radius: f32) -> impl DistanceFn {
	move |point: Vec3| point.length() - radius
}

pub fn sd_box(size: Vec3) -> impl DistanceFn {
	move |point: Vec3| {
		let v = point.abs() - size;
		// q.max(Vec3::splat(0.0)).length() + q.x.clamp(0.0, q.y.max(q.z))
		v.max(Vec3::splat(0.0)).length() +
		v.x.max(v.y.max(v.z)).min(0.0)
		// return length(max(q,0.0)) + min(max(q.x,max(q.y,q.z)),0.0);
	}
}
