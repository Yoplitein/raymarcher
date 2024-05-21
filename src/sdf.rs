use glam::{Affine3A, Vec3};


/*pub trait DistanceFn {
	fn eval(&mut self, point: Vec3);
}

impl<Func: FnMut(Vec3) -> f32> DistanceFn for Func {
	fn eval(&mut self, point: Vec3) -> f32 {
		self(point)
	}
}

pub fn transform(point: Vec3, transform: Affine3A) -> impl DistanceFn {
	todo!()
}

pub fn scale() -> impl DistanceFn {
	todo!()
} */

pub fn sd_sphere(point: Vec3, radius: f32) -> f32 {
	point.length() - radius
}

pub fn sd_box(point: Vec3, b: Vec3, r: f32) -> f32 {
	let q = point.abs() - b;
	q.max(Vec3::splat(0.0)).length() + q.x.clamp(0.0, q.y.max(q.z))
	// return length(max(q,0.0)) + min(max(q.x,max(q.y,q.z)),0.0);
}
