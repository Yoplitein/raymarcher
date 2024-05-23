use glam::{vec3, Affine3A, Vec3};
use image::Rgb;

pub trait DistanceFn: Send + Sync {
	fn eval(&self, point: Vec3) -> f32;
	
	fn eval_normal(&self, point: Vec3) -> Vec3 {
		let epsilon = 1e-2;
		let v = Vec3::ZERO;
		vec3(
			self.eval(point + v.with_x(epsilon)) - self.eval(point - v.with_x(epsilon)),
			self.eval(point + v.with_y(epsilon)) - self.eval(point - v.with_y(epsilon)),
			self.eval(point + v.with_z(epsilon)) - self.eval(point - v.with_z(epsilon)),
		).normalize()
	}
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
	
	fn smooth_union(self, factor: f32, mut other: impl DistanceFn) -> impl DistanceFn {
		move |point: Vec3| {
			let d1 = self.eval(point);
			let d2 = other.eval(point);
			let h = (0.5 + 0.5 * (d2 - d1) / factor).clamp(0.0, 1.0);
			lerp(d2, d1, h) - factor * h * (1.0 - h)
		}
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

fn lerp(from: f32, to: f32, t: f32) -> f32 {
	(1.0 - t) * from + t * to
}
