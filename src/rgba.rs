use std::fmt::Debug;

use bytemuck::{Pod, Zeroable, must_cast};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Eq, PartialEq)]
pub struct Rgba {
	pub r: u8,
	pub g: u8,
	pub b: u8,
	pub a: u8,
}

impl Rgba {
	pub const BLACK: Rgba = Rgba { r: 0, g: 0, b: 0, a: 255 };
	pub const WHITE: Rgba = Rgba { r: 255, g: 255, b: 255, a: 255 };

	pub fn rgb(r: u8, g: u8, b: u8) -> Self {
		Rgba { r, g, b, a: 255 }
	}

	pub fn hsv(h: f32, s: f32, v: f32) -> Self {
		let c = v * s;
		let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
		let m = v - c;
		let (r, g, b) = match h {
			h if h < 60.0 => (c, x, 0.0),
			h if h < 120.0 => (x, c, 0.0),
			h if h < 180.0 => (0.0, c, x),
			h if h < 240.0 => (0.0, x, c),
			h if h < 300.0 => (x, 0.0, c),
			_ => (c, 0.0, x),
		};

		Self::rgb(
			((r + m) * 255.0) as u8,
			((g + m) * 255.0) as u8,
			((b + m) * 255.0) as u8,
		)
	}

	pub fn hue(h: f32) -> Self {
		Self::hsv(360.0 * h, 1.0, 1.0)
	}

	pub fn to_bytes(self) -> [u8; 4] {
		must_cast(self)
	}
}

impl Default for Rgba {
	fn default() -> Self {
		Rgba::BLACK
	}
}
