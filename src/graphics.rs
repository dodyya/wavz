use crate::fft::BoxSlice2D;
use crate::fft::Float;
use crate::rgba::*;

// TODO: figure out if theres an easier way

pub fn gen_spectrogram(spectra: BoxSlice2D<Float>, range: f32) -> BoxSlice2D<Rgba> {
	let width = spectra.height; //TRANSPOSE!
	let height = spectra.width;

	let mut img = vec![Rgba::BLACK; width * height];

	for x in 0..width {
		let spectrum = spectra.row(x);
		for (y, rgba) in render_spectrum_iter(spectrum, range).enumerate() {
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

pub fn render_spectrum(spectrum: &[f32], range: f32) -> Vec<Rgba> {
	render_spectrum_iter(spectrum, range).collect()
}

pub fn render_spectrum_iter(spectrum: &[f32], range: f32) -> impl Iterator<Item = Rgba> {
	// const RANGE: f32 = 0.005;
	const CUTOFF: f32 = 0.2; // Visual cutoff for what is black
	let growth = 1.001f32;

	spectrum.iter().scan(0.2f32, move |factor, &value| {
		let normed_hue = (*factor * (value) / range).clamp(0.0, 1.0);
		*factor = *factor * growth;
		if normed_hue > CUTOFF {
			Some(Rgba::hue(normed_hue))
		} else {
			Some(Rgba::BLACK)
		}
	})
}
