use crate::fft::BoxSlice2D;
use std::fmt::Debug;

use bytemuck::NoUninit;

use crate::fft::Float;

const CUTOFF: f32 = 0.05; // Visual cutoff for what is black
const CLAMP_FACTOR: f32 = 1.0; //Twiddle this to make loud things look more uniform

// TODO: figure out if theres an easier way
/// made because `f32` doesn't implement `Ord`, so can't just use the max or min methods
fn extrema<'a>(v: impl Iterator<Item = &'a f32>) -> (f32, f32) {
	v.fold((f32::MAX, f32::MIN), |(curr_min, curr_max), &x| {
		(curr_min.min(x), curr_max.max(x))
	})
}

#[repr(C)]
#[derive(Debug, Clone, Copy, NoUninit)]
pub struct Rgba {
	r: u8,
	g: u8,
	b: u8,
	a: u8,
}

impl Rgba {
	const BLACK: Rgba = Rgba { r: 0, g: 0, b: 0, a: 255 };
	const WHITE: Rgba = Rgba { r: 255, g: 255, b: 255, a: 255 };

	fn rgb(r: u8, g: u8, b: u8) -> Self {
		Rgba { r, g, b, a: 255 }
	}
	fn hsv(h: f32, s: f32, v: f32) -> Self {
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
	fn hue(h: f32) -> Self {
		Self::hsv(360.0 * h, 1.0, 1.0)
	}
}

impl Default for Rgba {
	fn default() -> Self {
		Rgba::BLACK
	}
}

// TODO: switch away from nested vec arguments across the codebase. This could be moving
// towards boxed slices which can be converted into &mut [T] to take &mut [&mut T] arguments,
// or it could be moving to a custom BoxSlice2d and Slice2d struct (I think this is likely to work out best)
pub fn gen_spectrogram(spectra: BoxSlice2D<Float>) -> BoxSlice2D<Rgba> {
	let width = spectra.height; //TRANSPOSE!
	let height = spectra.width;

	let mut img = vec![Rgba::BLACK; width * height];

	for x in 0..width {
		let spectrum = spectra.row(x);
		let (min, max) = extrema(spectrum.iter());
		let range = CLAMP_FACTOR * (max - min);
		for (y, &value) in spectrum.iter().enumerate() {
			let start = x + y * width;
			let normed_hue = ((value - min) / range).clamp(0.0, 1.0);
			let pix_color = Rgba::hue(normed_hue);

			if normed_hue > CUTOFF {
				img[start] = pix_color;
			}
		}
	}

	BoxSlice2D {
		width,
		height,
		data: img.into_boxed_slice(),
	}
}
