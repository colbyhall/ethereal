use serde::{
	Deserialize,
	Deserializer,
	Serialize,
	Serializer,
};

use std::ops::Mul;

use crate::Vec3;
use crate::TO_RAD;

pub type Quat = Quaternion;

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct Quaternion {
	pub x: f32,
	pub y: f32,
	pub z: f32,
	pub w: f32,
}

impl Default for Quaternion {
	fn default() -> Self {
		Self::IDENTITY
	}
}

impl Quaternion {
	pub const IDENTITY: Self = Self {
		x: 0.0,
		y: 0.0,
		z: 0.0,
		w: 1.0,
	};

	pub fn from_axis_angle(axis: impl Into<Vec3>, theta: f32) -> Self {
		let axis = axis.into();

		let theta = theta / 2.0;

		let s = theta.sin();
		let c = theta.cos();
		Self {
			x: s * axis.x,
			y: s * axis.y,
			z: s * axis.z,
			w: c,
		}
	}

	pub fn from_euler(euler: impl Into<Vec3>) -> Self {
		const RADS_DIV_BY_2: f32 = TO_RAD / 2.0;

		let euler = euler.into();

		let pitch = euler.x % 360.0;
		let yaw = euler.y % 360.0;
		let roll = euler.z % 360.0;

		let sp = (pitch * RADS_DIV_BY_2).sin();
		let cp = (pitch * RADS_DIV_BY_2).cos();

		let sy = (yaw * RADS_DIV_BY_2).sin();
		let cy = (yaw * RADS_DIV_BY_2).cos();

		let sr = (roll * RADS_DIV_BY_2).sin();
		let cr = (roll * RADS_DIV_BY_2).cos();

		Self {
			x: cr * sp * sy - sr * cp * cy,
			y: -cr * sp * cy - sr * cp * sy,
			z: cr * cp * sy - sr * sp * cy,
			w: cr * cp * cy + sr * sp * sy,
		}
	}

	pub fn len_sq(self) -> f32 {
		self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w
	}

	pub fn len(self) -> f32 {
		self.len_sq().sqrt()
	}

	pub fn is_empty(self) -> bool {
		self.len() < crate::SMALL_NUMBER
	}

	#[must_use]
	pub fn norm(self) -> Self {
		let len = self.len();

		// TODO: Use a threshold
		if len == 0.0 {
			Default::default()
		} else {
			let inv = 1.0 / len;
			Self {
				x: self.x * inv,
				y: self.y * inv,
				z: self.z * inv,
				w: self.w * inv,
			}
		}
	}

	#[must_use]
	pub fn inverse(self) -> Self {
		Self {
			x: -self.x,
			y: -self.y,
			z: -self.z,
			w: self.w,
		}
	}

	pub fn rotate(self, xyz: Vec3) -> Vec3 {
		let t = self.xyz().cross(xyz) * 2.0;
		xyz + (t * self.w) + self.xyz().cross(t)
	}

	pub fn forward(self) -> Vec3 {
		self.rotate(Vec3::FORWARD)
	}

	pub fn right(self) -> Vec3 {
		self.rotate(Vec3::RIGHT)
	}

	pub fn up(self) -> Vec3 {
		self.rotate(Vec3::UP)
	}

	pub fn xyz(self) -> Vec3 {
		Vec3 {
			x: self.x,
			y: self.y,
			z: self.z,
		}
	}
}

impl Mul for Quaternion {
	type Output = Self;

	fn mul(self, rhs: Self) -> Self::Output {
		Self {
			x: (((self.w * rhs.x) + (self.x * rhs.w)) + (self.y * rhs.z)) - (self.z * rhs.y),
			y: (((self.w * rhs.y) + (self.y * rhs.w)) + (self.z * rhs.x)) - (self.x * rhs.z),
			z: (((self.w * rhs.z) + (self.z * rhs.w)) + (self.x * rhs.y)) - (self.y * rhs.x),
			w: (((self.w * rhs.w) - (self.x * rhs.x)) - (self.y * rhs.y)) - (self.z * rhs.z),
		}
		.norm()
	}
}

impl Serialize for Quaternion {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let xyzw = [self.x, self.y, self.z, self.w];
		xyzw.serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for Quaternion {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let xyzw = <[f32; 4]>::deserialize(deserializer)?;
		Ok(Quaternion {
			x: xyzw[0],
			y: xyzw[1],
			z: xyzw[2],
			w: xyzw[3],
		})
	}
}
