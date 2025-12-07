use crate::fft::BoxSlice2D;
use crate::fft::Float;
use crate::rgba::*;

// TODO: figure out if theres an easier way

pub fn gen_spectrogram(spectra: BoxSlice2D<Float>) -> BoxSlice2D<Rgba> {
	let width = spectra.height; //TRANSPOSE!
	let height = spectra.width;

	let mut img = vec![Rgba::BLACK; width * height];

	for x in 0..width {
		let spectrum = spectra.row(x);
		for (y, rgba) in render_spectrum(spectrum).into_iter().enumerate() {
			let start = x + y * width;
			img[start] = rgba;
		}
	}

	BoxSlice2D {
		width,
		height,
		data: img.into_boxed_slice(),
	}
}

pub fn render_spectrum(spectrum: &[f32]) -> Vec<Rgba> {
	const RANGE: f32 = 0.005;
	const CUTOFF: f32 = 0.15; // Visual cutoff for what is black
	spectrum
		.iter()
		.map(|&value| {
			let normed_hue = ((value) / RANGE).clamp(0.0, 1.0);
			if normed_hue > CUTOFF { Rgba::hue(normed_hue) } else { Rgba::BLACK }
		})
		.collect()
}
